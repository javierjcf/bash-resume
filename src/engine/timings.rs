//! Tiempos de los comandos, guardados en `times.json` del proyecto (spec §6.1, D16).
//!
//! Se guarda la última duración buena de cada comando, con su texto como clave: sin fichero ni número de línea, para
//! que añadir, mover o renombrar no haga perder nada. Una línea cambiada es un comando nuevo, sin tiempo. Los totales
//! se calculan siempre recorriendo los ficheros actuales y buscando cada comando; lo que no se encuentra, no cuenta.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::parser::{Kind, Line, StepFile};
use crate::log::format_duration;
use crate::write_atomic;

/// Duración de cada comando (texto recortado → segundos), con la ruta de su `times.json`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Timings {
    secs: BTreeMap<String, u64>,
    path: PathBuf,
}

/// Suma de los tiempos conocidos de un grupo de comandos, y cuántos no tienen tiempo.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Estimate {
    pub known: Duration,
    /// Comandos con tiempo.
    pub timed: usize,
    /// Comandos sin tiempo (nuevos o cambiados).
    pub missing: usize,
}

impl Estimate {
    /// `~01:20:00`, con `, 2 sin tiempo` si falta alguno; `None` si no se conoce ninguno.
    pub fn describe(&self) -> Option<String> {
        if self.timed == 0 {
            return None;
        }
        let mut text = format!("~{}", format_duration(self.known));
        if self.missing > 0 {
            text.push_str(&format!(", {} sin tiempo", self.missing));
        }
        Some(text)
    }

    fn add(&mut self, other: Estimate) {
        self.known += other.known;
        self.timed += other.timed;
        self.missing += other.missing;
    }
}

impl Timings {
    /// Carga `path`. Si no existe, empieza vacío; si no se puede leer o está corrupto, avisa y empieza vacío:
    /// los tiempos nunca impiden ejecutar.
    pub fn load(path: &Path) -> Self {
        let secs = match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| {
                eprintln!("⚠️ times.json corrupto: se empieza sin tiempos guardados.");
                BTreeMap::new()
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => {
                eprintln!("⚠️ No se pudo leer {}: {e}", path.display());
                BTreeMap::new()
            }
        };
        Self {
            secs,
            path: path.to_path_buf(),
        }
    }

    /// Guarda en `times.json`, de forma atómica.
    pub fn save(&self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.secs)?;
        write_atomic(&self.path, &(json + "\n"))
    }

    /// Anota lo que tardó `command` (sustituye lo anterior).
    pub fn record(&mut self, command: &str, elapsed: Duration) {
        self.secs
            .insert(command.trim().to_string(), elapsed.as_secs());
    }

    /// Lo que tardó `command` la última vez que terminó bien.
    pub fn get(&self, command: &str) -> Option<Duration> {
        self.secs
            .get(command.trim())
            .copied()
            .map(Duration::from_secs)
    }

    /// Suma de los comandos de `lines`.
    pub fn total(&self, lines: &[Line]) -> Estimate {
        let mut estimate = Estimate::default();
        for line in lines.iter().filter(|l| l.kind == Kind::Cmd) {
            match self.get(&line.text) {
                Some(d) => {
                    estimate.known += d;
                    estimate.timed += 1;
                }
                None => estimate.missing += 1,
            }
        }
        estimate
    }

    /// Lo que queda desde la línea `line` (0-based, incluida) del fichero `file` hasta el final.
    pub fn remaining(&self, files: &[StepFile], file: usize, line: usize) -> Estimate {
        let mut estimate = Estimate::default();
        for (i, f) in files.iter().enumerate().skip(file) {
            let from = if i == file { line } else { 0 };
            estimate.add(self.total(f.lines.get(from..).unwrap_or_default()));
        }
        estimate
    }
}

/// Número (1-based) del comando de la línea `line` del fichero `file` entre todos los comandos, y cuántos hay.
pub fn progress(files: &[StepFile], file: usize, line: usize) -> (usize, usize) {
    let commands = |lines: &[Line]| lines.iter().filter(|l| l.kind == Kind::Cmd).count();
    let before: usize = files.iter().take(file).map(|f| commands(&f.lines)).sum();
    let in_file = files
        .get(file)
        .map_or(0, |f| commands(f.lines.get(..line).unwrap_or_default()));
    let total = files.iter().map(|f| commands(&f.lines)).sum();
    (before + in_file + 1, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;

    fn step(name: &str, content: &str) -> StepFile {
        StepFile {
            name: name.to_string(),
            lines: content.lines().map(Line::new).collect(),
        }
    }

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn records_and_gets_by_command_text() {
        let mut t = Timings::default();
        t.record("  echo a ", secs(10));
        assert_eq!(t.get("echo a"), Some(secs(10)), "se busca recortado");
        assert_eq!(t.get("echo b"), None);
        t.record("echo a", secs(7));
        assert_eq!(t.get("echo a"), Some(secs(7)), "la última vez sustituye");
    }

    #[test]
    fn editing_lines_only_affects_the_changed_ones() {
        let mut t = Timings::default();
        for (cmd, n) in [("a", 10), ("b", 20), ("c", 30), ("d", 40)] {
            t.record(cmd, secs(n));
        }
        // `c` cambia a `c1` y se añaden `x` (antes) e `y` (después): el resto conserva su tiempo.
        let lines = step("01", "a\nb\n#> nota\nx\nc1\ny\nd").lines;
        let total = t.total(&lines);
        assert_eq!(total.known, secs(70), "a + b + d; `c` ya no se suma");
        assert_eq!((total.timed, total.missing), (3, 3));
    }

    #[test]
    fn remaining_and_progress_across_files() {
        let mut t = Timings::default();
        t.record("a", secs(60));
        t.record("c", secs(120));
        let files = [step("01", "#> hola\na\nb"), step("02", "c")];

        let from_b = t.remaining(&files, 0, 2);
        assert_eq!(
            (from_b.known, from_b.timed, from_b.missing),
            (secs(120), 1, 1)
        );
        assert_eq!(
            from_b.describe().as_deref(),
            Some("~00:02:00, 1 sin tiempo")
        );
        assert_eq!(t.remaining(&files, 0, 0).known, secs(180));

        assert_eq!(progress(&files, 0, 1), (1, 3));
        assert_eq!(progress(&files, 0, 2), (2, 3));
        assert_eq!(progress(&files, 1, 0), (3, 3));
    }

    #[test]
    fn nothing_known_describes_nothing() {
        let files = [step("01", "a\nb")];
        let mut t = Timings::default();
        assert_eq!(t.remaining(&files, 0, 0).describe(), None);
        t.record("a", secs(5));
        assert_eq!(
            t.remaining(&files, 0, 0).describe().as_deref(),
            Some("~00:00:05, 1 sin tiempo")
        );
    }

    #[test]
    fn missing_or_corrupt_file_starts_empty() -> io::Result<()> {
        let dir = temp_dir("timings-load")?;
        let path = dir.join("times.json");
        assert_eq!(Timings::load(&path).get("a"), None);
        fs::write(&path, "{ no es json")?;
        assert_eq!(Timings::load(&path).get("a"), None);
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn save_and_load_round_trip() -> io::Result<()> {
        let dir = temp_dir("timings-save")?;
        let path = dir.join("times.json");
        let mut t = Timings::load(&path);
        t.record("echo a", secs(5));
        t.save()?;
        assert_eq!(Timings::load(&path), t);
        assert!(fs::read_to_string(&path)?.contains("\"echo a\": 5"));
        fs::remove_dir_all(&dir)
    }
}
