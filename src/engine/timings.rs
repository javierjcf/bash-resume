//! Tiempos de los comandos, guardados en `times.json` del proyecto (spec §6.1, D17).
//!
//! Por cada fichero de pasos, la última duración buena de cada comando, con su texto como clave y en `HH:MM:SS`. Sin
//! número de línea, para que añadir o mover líneas no haga perder nada; con el fichero, para que un mismo comando en
//! dos ficheros (el `--update all` de cada versión) tenga su propio tiempo. Una línea cambiada es un comando nuevo, sin
//! tiempo. Los totales se calculan siempre recorriendo los ficheros actuales; lo que no se encuentra, no cuenta.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::parser::{Kind, Line, StepFile};
use crate::log::format_duration;
use crate::write_atomic;

/// Comando (texto recortado) → duración, de un fichero de pasos.
type FileTimes = BTreeMap<String, Duration>;

/// Duración de cada comando de cada fichero de pasos, con la ruta de su `times.json`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Timings {
    files: BTreeMap<String, FileTimes>,
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
    /// Carga `path`. Si no existe, empieza vacío; si no se puede leer, está corrupto o tiene el formato antiguo (solo
    /// comando → segundos, D16), avisa y empieza vacío: los tiempos nunca impiden ejecutar.
    pub fn load(path: &Path) -> Self {
        let files = match fs::read_to_string(path) {
            Ok(text) => parse(&text).unwrap_or_else(|| {
                if serde_json::from_str::<BTreeMap<String, u64>>(&text).is_ok() {
                    eprintln!(
                        "⚠️ times.json tiene el formato antiguo: se empieza sin tiempos guardados."
                    );
                } else {
                    eprintln!("⚠️ times.json corrupto: se empieza sin tiempos guardados.");
                }
                BTreeMap::new()
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => {
                eprintln!("⚠️ No se pudo leer {}: {e}", path.display());
                BTreeMap::new()
            }
        };
        Self {
            files,
            path: path.to_path_buf(),
        }
    }

    /// Guarda en `times.json`, de forma atómica, con los tiempos en `HH:MM:SS`.
    pub fn save(&self) -> io::Result<()> {
        let shown: BTreeMap<&str, BTreeMap<&str, String>> = self
            .files
            .iter()
            .filter(|(_, commands)| !commands.is_empty())
            .map(|(file, commands)| {
                let commands = commands
                    .iter()
                    .map(|(cmd, d)| (cmd.as_str(), format_duration(*d)))
                    .collect();
                (file.as_str(), commands)
            })
            .collect();
        let json = serde_json::to_string_pretty(&shown)?;
        write_atomic(&self.path, &(json + "\n"))
    }

    /// Anota lo que tardó `command` del fichero `file` (sustituye lo anterior).
    pub fn record(&mut self, file: &str, command: &str, elapsed: Duration) {
        self.files.entry(file.to_string()).or_default().insert(
            command.trim().to_string(),
            Duration::from_secs(elapsed.as_secs()),
        );
    }

    /// Olvida los tiempos de `file` cuyos comandos ya no están en `lines` (líneas editadas o borradas).
    pub fn prune(&mut self, file: &str, lines: &[Line]) {
        if let Some(commands) = self.files.get_mut(file) {
            commands.retain(|cmd, _| {
                lines
                    .iter()
                    .any(|l| l.kind == Kind::Cmd && l.text.trim() == cmd)
            });
        }
    }

    /// Lo que tardó `command` del fichero `file` la última vez que terminó bien.
    pub fn get(&self, file: &str, command: &str) -> Option<Duration> {
        self.files.get(file)?.get(command.trim()).copied()
    }

    /// Suma de los comandos de `lines`, que son (parte de) el fichero `file`.
    pub fn total(&self, file: &str, lines: &[Line]) -> Estimate {
        let mut estimate = Estimate::default();
        for line in lines.iter().filter(|l| l.kind == Kind::Cmd) {
            match self.get(file, &line.text) {
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
            estimate.add(self.total(&f.name, f.lines.get(from..).unwrap_or_default()));
        }
        estimate
    }
}

/// Lee `{ "fichero": { "comando": "HH:MM:SS" } }`; `None` si no tiene esa forma.
fn parse(text: &str) -> Option<BTreeMap<String, FileTimes>> {
    let raw: BTreeMap<String, BTreeMap<String, String>> = serde_json::from_str(text).ok()?;
    raw.into_iter()
        .map(|(file, commands)| {
            let commands = commands
                .into_iter()
                .map(|(cmd, time)| Some((cmd, parse_duration(&time)?)))
                .collect::<Option<FileTimes>>()?;
            Some((file, commands))
        })
        .collect()
}

/// `HH:MM:SS` (las horas pueden pasar de 99) → duración; `None` si no tiene ese formato.
fn parse_duration(text: &str) -> Option<Duration> {
    let mut parts = text.trim().split(':').map(|p| p.parse::<u64>().ok());
    let (h, m, s) = (parts.next()??, parts.next()??, parts.next()??);
    if parts.next().is_some() || m >= 60 || s >= 60 {
        return None;
    }
    Some(Duration::from_secs(h * 3600 + m * 60 + s))
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
    fn records_and_gets_by_file_and_command_text() {
        let mut t = Timings::default();
        t.record("01", "  echo a ", secs(10));
        assert_eq!(t.get("01", "echo a"), Some(secs(10)), "se busca recortado");
        assert_eq!(t.get("01", "echo b"), None);
        assert_eq!(t.get("02", "echo a"), None, "cada fichero, lo suyo");
        t.record("01", "echo a", secs(7));
        assert_eq!(
            t.get("01", "echo a"),
            Some(secs(7)),
            "la última vez sustituye"
        );
    }

    #[test]
    fn same_command_in_two_files_keeps_two_times() {
        // El caso real: el mismo `--update all` en cada versión, con tiempos muy distintos.
        let mut t = Timings::default();
        t.record("09-mig", "update all", secs(9 * 3600));
        t.record("13-mig", "update all", secs(2 * 3600));
        let files = [step("09-mig", "update all"), step("13-mig", "update all")];
        assert_eq!(t.total("09-mig", &files[0].lines).known, secs(9 * 3600));
        assert_eq!(t.total("13-mig", &files[1].lines).known, secs(2 * 3600));
        assert_eq!(t.remaining(&files, 0, 0).known, secs(11 * 3600));
    }

    #[test]
    fn editing_lines_only_affects_the_changed_ones() {
        let mut t = Timings::default();
        for (cmd, n) in [("a", 10), ("b", 20), ("c", 30), ("d", 40)] {
            t.record("01", cmd, secs(n));
        }
        // `c` cambia a `c1` y se añaden `x` (antes) e `y` (después): el resto conserva su tiempo.
        let lines = step("01", "a\nb\n#> nota\nx\nc1\ny\nd").lines;
        let total = t.total("01", &lines);
        assert_eq!(total.known, secs(70), "a + b + d; `c` ya no se suma");
        assert_eq!((total.timed, total.missing), (3, 3));

        t.prune("01", &lines);
        assert_eq!(t.get("01", "c"), None, "`c` ya no está en el fichero");
        assert_eq!(t.get("01", "d"), Some(secs(40)));
    }

    #[test]
    fn remaining_and_progress_across_files() {
        let mut t = Timings::default();
        t.record("01", "a", secs(60));
        t.record("02", "c", secs(120));
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
        t.record("01", "a", secs(5));
        assert_eq!(
            t.remaining(&files, 0, 0).describe().as_deref(),
            Some("~00:00:05, 1 sin tiempo")
        );
    }

    #[test]
    fn parses_hh_mm_ss() {
        assert_eq!(
            parse_duration("08:59:12"),
            Some(secs(8 * 3600 + 59 * 60 + 12))
        );
        assert_eq!(parse_duration("120:00:00"), Some(secs(120 * 3600)));
        for bad in ["", "1:2", "00:60:00", "00:00:60", "a:b:c", "1:2:3:4"] {
            assert_eq!(parse_duration(bad), None, "{bad}");
        }
    }

    #[test]
    fn missing_corrupt_or_old_file_starts_empty() -> io::Result<()> {
        let dir = temp_dir("timings-load")?;
        let path = dir.join("times.json");
        assert_eq!(Timings::load(&path).get("01", "a"), None);
        fs::write(&path, "{ no es json")?;
        assert_eq!(Timings::load(&path).get("01", "a"), None);
        fs::write(&path, r#"{ "a": 5 }"#)?;
        assert_eq!(
            Timings::load(&path).files,
            BTreeMap::new(),
            "formato antiguo"
        );
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn save_and_load_round_trip() -> io::Result<()> {
        let dir = temp_dir("timings-save")?;
        let path = dir.join("times.json");
        let mut t = Timings::load(&path);
        t.record("09-mig", "echo a", secs(32352));
        t.save()?;
        assert_eq!(Timings::load(&path), t);
        let text = fs::read_to_string(&path)?;
        assert!(text.contains("\"09-mig\": {"), "{text}");
        assert!(text.contains("\"echo a\": \"08:59:12\""), "{text}");
        fs::remove_dir_all(&dir)
    }
}
