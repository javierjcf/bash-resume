//! Ejecutar un comando de shell en el directorio de trabajo (spec §4).

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{pty, signals};
use crate::log::{Logger, format_duration};

/// Opciones estrictas de bash (D10): `-e` para al primer fallo dentro de la línea, `-u` da error si
/// se usa una variable sin definir y `-o pipefail` hace fallar una tubería si falla cualquiera de sus partes.
const BASH_OPTIONS: [&str; 4] = ["-e", "-u", "-o", "pipefail"];

/// Hueco del ancho de una duración (`HH:MM:SS`) en la línea `⏳`, para que el comando quede en la misma columna que
/// en la de resultado.
const NO_DURATION: &str = "--:--:--";

/// Resultado de ejecutar un comando.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Execution {
    /// Terminó con 0 y sin Ctrl+C.
    pub ok: bool,
    /// Lo que tardó.
    pub elapsed: Duration,
}

/// Por dónde va la ejecución, para la línea `⏳` de inicio de cada comando (spec §8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// Número del comando (1-based) entre todos los de los ficheros de pasos.
    pub number: usize,
    pub total: usize,
    /// Estimación de lo que queda (`~01:20:00`), si se conoce algo.
    pub remaining: Option<String>,
}

/// Ejecuta `cmd` con `bash -euo pipefail -c` dentro de `workdir` y devuelve si terminó bien y lo que tardó.
///
/// La salida del comando va directamente a la terminal (stdout y stderr heredados), pero **no** el teclado: stdin
/// es una terminal falsa vacía (D15), así que `docker run -it` funciona, nadie puede poner la terminal real en modo
/// raw y Ctrl+C siempre llega como señal a bash-resume, que lo trata como fallo aunque el comando salga con 0.
///
/// En el log, dos líneas por comando, cada una completa por sí misma (spec §8): al empezar,
/// `⏳ [13/40] --:--:-- comando   (quedan ~01:20:00)`, y al terminar, `✅ [13/40] 00:00:07 comando` o
/// `❌ [13/40] 00:00:51 comando   (exit 1)`. El hueco `--:--:--` y el número rellenado (`[ 9/40]`) dejan el comando
/// siempre en la misma columna. Sin `progress` (como en `onerror`), sin `[n/N]` ni estimación.
pub fn execute(cmd: &str, workdir: &Path, log: &Logger, progress: Option<&Progress>) -> Execution {
    let number = progress.map_or(String::new(), |p| {
        let width = p.total.to_string().len();
        format!("[{:>width$}/{}] ", p.number, p.total)
    });
    let remaining = progress
        .and_then(|p| p.remaining.as_ref())
        .map_or(String::new(), |r| format!("   (quedan {r})"));
    log.info(&format!("⏳ {number}{NO_DURATION} {cmd}{remaining}"));
    let start = Instant::now();

    // Mientras dura el comando, Ctrl+C lo detiene a él (la terminal le envía SIGINT), pero no a bash-resume.
    signals::begin_command();

    // stdin: una terminal falsa vacía; si no se puede crear (por ejemplo, sin /dev/ptmx), /dev/null.
    let (stdin, master) = match pty::empty_tty() {
        Ok((master, slave)) => (Stdio::from(slave), Some(master)),
        Err(e) => {
            log.warn(&format!(
                "No se pudo crear la terminal falsa ({e}): stdin será /dev/null"
            ));
            (Stdio::null(), None)
        }
    };

    let status = Command::new("bash")
        .args(BASH_OPTIONS)
        .arg("-c")
        .arg(cmd)
        .current_dir(workdir)
        .stdin(stdin)
        .status();

    drop(master); // el comando ya terminó: se cierra la terminal falsa
    let interrupted = signals::end_command();

    let elapsed = start.elapsed();
    let done = format!("{number}{} {cmd}", format_duration(elapsed));
    let ok = match status {
        // Un comando interrumpido es un fallo aunque haya terminado con 0 (Odoo hace `os._exit(0)` al segundo Ctrl+C).
        _ if interrupted => {
            log.warn(&format!("⛔ {done}   (Ctrl+C)"));
            false
        }
        Ok(status) if status.success() => {
            log.info(&format!("✅ {done}"));
            true
        }
        Ok(status) => {
            // `code()` es `None` si el proceso murió por una señal (por ejemplo, kill -9).
            let why = status
                .code()
                .map_or(status.to_string(), |c| format!("exit {c}"));
            log.error(&format!("❌ {done}   ({why})"));
            false
        }
        Err(e) => {
            log.error(&format!("❌ {done}   (no se pudo lanzar: {e})"));
            false
        }
    };
    Execution { ok, elapsed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;
    use std::fs;
    use std::io;

    /// Directorio temporal con un logger que escribe en `migration.log` dentro de él.
    fn setup(name: &str) -> io::Result<(std::path::PathBuf, Logger)> {
        let dir = temp_dir(name)?;
        let log = Logger::open(&dir.join("migration.log"))?;
        Ok((dir, log))
    }

    #[test]
    fn success_and_failure() -> io::Result<()> {
        let (dir, log) = setup("exec-basic")?;
        assert!(execute("true", &dir, &log, None).ok);
        assert!(!execute("false", &dir, &log, None).ok);
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn logs_exit_code_of_failed_command() -> io::Result<()> {
        let (dir, log) = setup("exec-code")?;
        assert!(!execute("exit 3", &dir, &log, None).ok);
        let content = fs::read_to_string(dir.join("migration.log"))?;
        assert!(
            content.contains(" INFO    ⏳ --:--:-- exit 3\n"),
            "{content}"
        );
        assert!(
            content.contains(" ERROR   ❌ 00:00:00 exit 3   (exit 3)\n"),
            "{content}"
        );
        assert_eq!(
            content.lines().count(),
            2,
            "dos líneas por comando:\n{content}"
        );
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn pads_step_number_so_commands_line_up() -> io::Result<()> {
        let (dir, log) = setup("exec-number")?;
        let progress = Progress {
            number: 9,
            total: 197,
            remaining: None,
        };
        assert!(execute("true", &dir, &log, Some(&progress)).ok);
        let content = fs::read_to_string(dir.join("migration.log"))?;
        assert!(
            content.contains("⏳ [  9/197] --:--:-- true\n"),
            "{content}"
        );
        assert!(
            content.contains("✅ [  9/197] 00:00:00 true\n"),
            "{content}"
        );
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn runs_inside_workdir() -> io::Result<()> {
        let (dir, log) = setup("exec-workdir")?;
        let cmd = format!("test \"$(pwd)\" = '{}'", dir.display());
        assert!(
            execute(&cmd, &dir, &log, None).ok,
            "el comando debe ver workdir como directorio actual"
        );
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn missing_workdir_is_a_failure() -> io::Result<()> {
        let (dir, log) = setup("exec-nodir")?;
        assert!(!execute("true", &dir.join("no-existe"), &log, None).ok);
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn pipeline_fails_if_any_part_fails() -> io::Result<()> {
        let (dir, log) = setup("exec-pipefail")?;
        assert!(
            !execute("false | true", &dir, &log, None).ok,
            "pipefail: con sh daría éxito"
        );
        assert!(execute("true | true", &dir, &log, None).ok);
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn unset_variable_is_an_error() -> io::Result<()> {
        let (dir, log) = setup("exec-nounset")?;
        assert!(
            !execute(
                "echo \"$VARIABLE_QUE_NO_EXISTE_XYZ\" > /dev/null",
                &dir,
                &log,
                None
            )
            .ok
        );
        assert!(
            execute(
                "echo \"${VARIABLE_QUE_NO_EXISTE_XYZ:-por_defecto}\" > /dev/null",
                &dir,
                &log,
                None
            )
            .ok
        );
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn errexit_stops_inside_the_line() -> io::Result<()> {
        let (dir, log) = setup("exec-errexit")?;
        assert!(!execute("false; touch hecho", &dir, &log, None).ok);
        assert!(
            !dir.join("hecho").exists(),
            "-e para en el primer fallo de la línea"
        );
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn bash_syntax_is_available() -> io::Result<()> {
        let (dir, log) = setup("exec-bashisms")?;
        assert!(
            execute(
                "[[ -d . ]] && arr=(a b) && test \"${arr[1]}\" = b",
                &dir,
                &log,
                None
            )
            .ok
        );
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn stdin_is_an_empty_terminal() -> io::Result<()> {
        let (dir, log) = setup("exec-stdin")?;
        assert!(
            execute("test -t 0", &dir, &log, None).ok,
            "stdin es una terminal (docker -it la exige)"
        );
        // `cat` termina al instante: la terminal le da fin de entrada en vez de esperar al teclado.
        assert!(execute("cat", &dir, &log, None).ok);
        fs::remove_dir_all(&dir)
    }
}
