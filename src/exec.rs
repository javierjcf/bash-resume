//! Ejecución de un comando de shell (spec §4).

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use signal_hook::consts::SIGINT;
use signal_hook::flag;

use crate::log::Logger;

/// Estado de Ctrl+C, compartido entre el programa y el manejador de señales del sistema operativo.
struct Sigint {
    /// `true` fuera de un comando: Ctrl+C hace lo de siempre (terminar bash-resume).
    terminate: Arc<AtomicBool>,
    /// Se pone a `true` cuando llega un Ctrl+C.
    received: Arc<AtomicBool>,
}

/// Registra los manejadores de SIGINT la primera vez que se llama y devuelve sus flags.
///
/// Las señales son de todo el proceso, así que esto es global por naturaleza: se registra una sola
/// vez y no se quita nunca (quitar un manejador de signal-hook no restaura el comportamiento por defecto).
/// Devuelve `None` si no se pudo registrar: entonces Ctrl+C simplemente termina bash-resume.
fn sigint() -> Option<&'static Sigint> {
    static SIGINT_FLAGS: OnceLock<Option<Sigint>> = OnceLock::new();
    SIGINT_FLAGS
        .get_or_init(|| {
            let terminate = Arc::new(AtomicBool::new(true));
            let received = Arc::new(AtomicBool::new(false));
            flag::register_conditional_default(SIGINT, Arc::clone(&terminate)).ok()?;
            flag::register(SIGINT, Arc::clone(&received)).ok()?;
            Some(Sigint {
                terminate,
                received,
            })
        })
        .as_ref()
}

/// Opciones estrictas de bash (D10): `-e` para al primer fallo dentro de la línea, `-u` da error si
/// se usa una variable sin definir y `-o pipefail` hace fallar una tubería si falla cualquiera de sus partes.
const BASH_OPTIONS: [&str; 4] = ["-e", "-u", "-o", "pipefail"];

/// Ejecuta `cmd` con `bash -euo pipefail -c` dentro de `workdir` y devuelve si terminó bien.
///
/// La salida del comando va directamente a la terminal (stdio heredado).
pub fn execute(cmd: &str, workdir: &Path, log: &Logger) -> bool {
    log.info(&format!("Ejecutando comando: {cmd}"));
    let start = Instant::now();

    // Mientras dura el comando, Ctrl+C lo detiene a él (la terminal le envía SIGINT), pero no a bash-resume.
    let sigint = sigint();
    if let Some(s) = sigint {
        s.received.store(false, Ordering::SeqCst);
        s.terminate.store(false, Ordering::SeqCst);
    }

    let status = Command::new("bash")
        .args(BASH_OPTIONS)
        .arg("-c")
        .arg(cmd)
        .current_dir(workdir)
        .status();

    let interrupted = match sigint {
        Some(s) => {
            s.terminate.store(true, Ordering::SeqCst);
            s.received.swap(false, Ordering::SeqCst)
        }
        None => false,
    };

    let ok = match status {
        // Un comando interrumpido es un fallo aunque haya terminado con 0 (como en Python).
        _ if interrupted => {
            log.warn("⛔ Ctrl+C detectado: comando interrumpido");
            log.error(&format!("Comando fallido (interrumpido con Ctrl+C): {cmd}"));
            false
        }
        Ok(status) if status.success() => {
            log.info("🔚 Proceso finalizado correctamente (exit code: 0)");
            log.info(&format!(
                "✅ Comando completado correctamente (exit code 0): {cmd}"
            ));
            true
        }
        Ok(status) => {
            // `code()` es `None` si el proceso murió por una señal (por ejemplo, kill -9).
            let code = status.code().map_or(status.to_string(), |c| c.to_string());
            log.warn(&format!(
                "⚠️ Proceso finalizado con código de salida no cero: {code}"
            ));
            log.error(&format!("Comando fallido (exit code {code}): {cmd}"));
            false
        }
        Err(e) => {
            log.error(&format!("No se pudo lanzar el comando ({e}): {cmd}"));
            false
        }
    };

    log.info(&format!(
        "Tiempo de ejecución: {}",
        format_duration(start.elapsed())
    ));
    ok
}

/// Formatea una duración como `h:m:s` sin ceros a la izquierda, igual que la versión Python.
fn format_duration(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    format!("{}:{}:{}", secs / 3600, secs % 3600 / 60, secs % 60)
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
        assert!(execute("true", &dir, &log));
        assert!(!execute("false", &dir, &log));
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn logs_exit_code_of_failed_command() -> io::Result<()> {
        let (dir, log) = setup("exec-code")?;
        assert!(!execute("exit 3", &dir, &log));
        let content = fs::read_to_string(dir.join("migration.log"))?;
        assert!(
            content.contains("Comando fallido (exit code 3): exit 3"),
            "{content}"
        );
        assert!(content.contains("Tiempo de ejecución: 0:0:0"), "{content}");
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn runs_inside_workdir() -> io::Result<()> {
        let (dir, log) = setup("exec-workdir")?;
        let cmd = format!("test \"$(pwd)\" = '{}'", dir.display());
        assert!(
            execute(&cmd, &dir, &log),
            "el comando debe ver workdir como directorio actual"
        );
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn missing_workdir_is_a_failure() -> io::Result<()> {
        let (dir, log) = setup("exec-nodir")?;
        assert!(!execute("true", &dir.join("no-existe"), &log));
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn pipeline_fails_if_any_part_fails() -> io::Result<()> {
        let (dir, log) = setup("exec-pipefail")?;
        assert!(
            !execute("false | true", &dir, &log),
            "pipefail: con sh daría éxito"
        );
        assert!(execute("true | true", &dir, &log));
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn unset_variable_is_an_error() -> io::Result<()> {
        let (dir, log) = setup("exec-nounset")?;
        assert!(!execute(
            "echo \"$VARIABLE_QUE_NO_EXISTE_XYZ\" > /dev/null",
            &dir,
            &log
        ));
        assert!(execute(
            "echo \"${VARIABLE_QUE_NO_EXISTE_XYZ:-por_defecto}\" > /dev/null",
            &dir,
            &log
        ));
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn errexit_stops_inside_the_line() -> io::Result<()> {
        let (dir, log) = setup("exec-errexit")?;
        assert!(!execute("false; touch hecho", &dir, &log));
        assert!(
            !dir.join("hecho").exists(),
            "-e para en el primer fallo de la línea"
        );
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn bash_syntax_is_available() -> io::Result<()> {
        let (dir, log) = setup("exec-bashisms")?;
        assert!(execute(
            "[[ -d . ]] && arr=(a b) && test \"${arr[1]}\" = b",
            &dir,
            &log
        ));
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn formats_duration_like_python() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0:0:0");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1:2:5");
        assert_eq!(format_duration(Duration::from_millis(59_999)), "0:0:59");
    }
}
