//! Ejecución de un comando de shell (spec §4).

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::log::Logger;

/// Ejecuta `cmd` con `sh -c` dentro de `workdir` y devuelve si terminó bien.
///
/// La salida del comando va directamente a la terminal (stdio heredado).
pub fn execute(cmd: &str, workdir: &Path, log: &Logger) -> bool {
    log.info(&format!("Ejecutando comando: {cmd}"));
    let start = Instant::now();

    let status = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(workdir)
        .status();

    let ok = match status {
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
    fn formats_duration_like_python() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0:0:0");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1:2:5");
        assert_eq!(format_duration(Duration::from_millis(59_999)), "0:0:59");
    }
}
