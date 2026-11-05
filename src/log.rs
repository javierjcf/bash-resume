//! Log a fichero (`migration.log`) y a consola con colores (spec §8).

use std::fs::{File, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::time::Duration;

use crate::with_path;

const RESET: &str = "\x1b[0m";
const BLUE: &str = "\x1b[34m";

/// Duración como `HH:MM:SS` (`00:05:02`, `27:00:00`): la de comandos, ficheros, totales y estimaciones.
pub fn format_duration(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// Nivel de un mensaje de log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    /// Nombre del nivel tal y como aparece en el log.
    fn name(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warning => "WARNING",
            Level::Error => "ERROR",
        }
    }

    /// Código ANSI del color del nivel en consola.
    fn color(self) -> &'static str {
        match self {
            Level::Info => "\x1b[32m",
            Level::Warning => "\x1b[33m",
            Level::Error => "\x1b[31m",
        }
    }
}

/// Construye una línea de log: `fecha - [fuente] NIVEL   : mensaje`, con o sin colores ANSI.
fn format_line(
    timestamp: &str,
    source: Option<&str>,
    level: Level,
    msg: &str,
    color: bool,
) -> String {
    let source = match source {
        Some(s) => format!("[{s}] "),
        None => String::new(),
    };
    let name = level.name();
    if color {
        let c = level.color();
        format!("{BLUE}{timestamp}{RESET} - {source}{c}{name:<8}{RESET}: {c}{msg}{RESET}")
    } else {
        format!("{timestamp} - {source}{name:<8}: {msg}")
    }
}

/// Escribe cada mensaje en el fichero de log (sin colores) y en stderr (con colores si es una terminal).
pub struct Logger {
    file: File,
    /// Fichero de pasos en proceso, que se antepone a cada línea como `[fuente]`.
    source: Option<String>,
    color: bool,
}

impl Logger {
    /// Abre (o crea) el fichero de log en modo append.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(with_path(path))?;
        Ok(Self {
            file,
            source: None,
            color: io::stderr().is_terminal(),
        })
    }

    /// Fija la fuente que se antepone a las líneas (`None` para quitarla).
    pub fn set_source(&mut self, source: Option<&str>) {
        self.source = source.map(str::to_string);
    }

    pub fn info(&self, msg: &str) {
        self.log(Level::Info, msg);
    }

    pub fn warn(&self, msg: &str) {
        self.log(Level::Warning, msg);
    }

    pub fn error(&self, msg: &str) {
        self.log(Level::Error, msg);
    }

    /// Escribe el mensaje. Un fallo al escribir el log no debe parar la ejecución, así que se ignora.
    pub fn log(&self, level: Level, msg: &str) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let source = self.source.as_deref();
        let _ = writeln!(
            &self.file,
            "{}",
            format_line(&now, source, level, msg, false)
        );
        eprintln!("{}", format_line(&now, source, level, msg, self.color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;
    use std::fs;

    const TS: &str = "2026-11-04 10:00:00";

    #[test]
    fn formats_durations() {
        let secs = Duration::from_secs;
        assert_eq!(format_duration(secs(0)), "00:00:00");
        assert_eq!(format_duration(secs(302)), "00:05:02");
        assert_eq!(format_duration(secs(27 * 3600)), "27:00:00");
        assert_eq!(format_duration(Duration::from_millis(59_999)), "00:00:59");
    }

    #[test]
    fn formats_plain_line_like_python() {
        let line = format_line(TS, Some("01-a"), Level::Info, "hola", false);
        assert_eq!(line, "2026-11-04 10:00:00 - [01-a] INFO    : hola");
    }

    #[test]
    fn omits_source_when_none() {
        let line = format_line(TS, None, Level::Warning, "aviso", false);
        assert_eq!(line, "2026-11-04 10:00:00 - WARNING : aviso");
    }

    #[test]
    fn colored_line_wraps_parts_in_ansi_codes() {
        let line = format_line(TS, None, Level::Error, "mal", true);
        assert_eq!(
            line,
            "\x1b[34m2026-11-04 10:00:00\x1b[0m - \x1b[31mERROR   \x1b[0m: \x1b[31mmal\x1b[0m"
        );
    }

    #[test]
    fn logger_appends_to_file_with_current_source() -> io::Result<()> {
        let dir = temp_dir("log")?;
        let path = dir.join("migration.log");
        fs::write(&path, "línea previa\n")?;

        let mut log = Logger::open(&path)?;
        log.info("sin fuente");
        log.set_source(Some("01-a"));
        log.error("con fuente");

        let content = fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3, "{content}");
        assert_eq!(lines[0], "línea previa", "append, no sobrescribe");
        // Las líneas empiezan por la fecha (19 caracteres); comprobamos el resto.
        assert_eq!(&lines[1][TS.len()..], " - INFO    : sin fuente");
        assert_eq!(&lines[2][TS.len()..], " - [01-a] ERROR   : con fuente");
        fs::remove_dir_all(&dir)
    }
}
