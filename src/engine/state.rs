//! Estado de ejecución persistido en `state.json` (spec §6).

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::parser::parse_file;
use crate::with_path;

/// Estado de ejecución. El formato JSON es compatible con la versión Python (D4).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Directorio donde se ejecutan los comandos (WORKDIR).
    pub execution_path: Option<PathBuf>,
    /// Carpeta con los ficheros de pasos.
    pub steps_path: Option<PathBuf>,
    /// Fichero de pasos del siguiente comando a ejecutar.
    pub current_file: Option<String>,
    /// Texto del siguiente comando, para comprobar que la línea no ha cambiado.
    pub last_command: Option<String>,
    /// Número de línea (1-based) de `last_command` dentro de `current_file`.
    pub last_line_index: Option<usize>,

    /// Ruta de `state.json`. No se guarda en el JSON.
    #[serde(skip)]
    path: PathBuf,
}

impl State {
    /// Carga el estado de `path` y lo vuelve a guardar normalizado.
    ///
    /// Si el fichero no existe, lo crea vacío. Si está corrupto, avisa y lo recrea.
    /// Las claves que falten se completan con `null`.
    pub fn load(path: &Path) -> io::Result<Self> {
        let mut state = match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| {
                eprintln!("⚠️ state.json corrupto. Se recreará.");
                State::default()
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => State::default(),
            Err(e) => return Err(with_path(path)(e)),
        };
        state.path = path.to_path_buf();
        state.save()?;
        Ok(state)
    }

    /// Escribe el estado en su `state.json`.
    ///
    /// Se escribe en `state.json.tmp` y se renombra: renombrar es atómico, así que si el proceso
    /// muere a mitad, `state.json` sigue siendo el anterior (nunca queda a medias).
    pub fn save(&self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json + "\n").map_err(with_path(&tmp))?;
        fs::rename(&tmp, &self.path).map_err(with_path(&self.path))
    }

    /// Guarda la posición del siguiente comando (línea 1-based) y escribe el estado.
    pub fn set_position(&mut self, file: &str, command: &str, line: usize) -> io::Result<()> {
        self.current_file = Some(file.to_string());
        self.last_command = Some(command.to_string());
        self.last_line_index = Some(line);
        self.save()
    }

    /// Indica si el estado apunta a una línea que existe en `steps_dir` y sigue teniendo el mismo comando.
    pub fn is_valid(&self, steps_dir: &Path) -> bool {
        self.position_matches(steps_dir).unwrap_or(false)
    }

    /// Devuelve `None` en cuanto falte un dato; si los hay todos, `Some(¿coincide el comando?)`.
    fn position_matches(&self, steps_dir: &Path) -> Option<bool> {
        let file = self.current_file.as_ref()?;
        let command = self.last_command.as_ref()?;
        let index = self.last_line_index?;
        let lines = parse_file(&steps_dir.join(file)).ok()?;
        let line = lines.get(index.checked_sub(1)?)?;
        Some(line.text.trim() == command.trim())
    }
}

impl fmt::Display for State {
    /// Resumen legible del punto de ejecución pendiente.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let workdir = display_path(self.execution_path.as_deref());
        let steps = display_path(self.steps_path.as_deref());
        row(f, "Directorio ejecución", &workdir)?;
        row(f, "Directorio pasos", &steps)?;
        row(
            f,
            "Fichero a ejecutar",
            self.current_file.as_deref().unwrap_or("(sin fichero)"),
        )?;
        if let Some(line) = self.last_line_index {
            row(f, "Línea", &line.to_string())?;
        }
        row(
            f,
            "Comando a ejecutar",
            self.last_command.as_deref().unwrap_or("(sin comando)"),
        )
    }
}

/// Escribe una fila `etiqueta: valor` del resumen, con la etiqueta alineada.
fn row(f: &mut fmt::Formatter, label: &str, value: &str) -> fmt::Result {
    writeln!(f, "  {label:<24}: {value}")
}

/// Ruta como texto, o un marcador si no hay.
fn display_path(path: Option<&Path>) -> String {
    match path {
        Some(p) => p.display().to_string(),
        None => "(sin ruta)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;

    /// Tal cual lo escribe la versión Python (`json.dump(data, f, indent=4)`).
    const PYTHON_STATE: &str = r#"{
    "execution_path": "/home/javier/doodba",
    "steps_path": "/home/javier/pasos",
    "current_file": "02-fallar",
    "last_command": "false",
    "last_line_index": 3
}"#;

    #[test]
    fn reads_state_written_by_python() -> io::Result<()> {
        let dir = temp_dir("state-python")?;
        let path = dir.join("state.json");
        fs::write(&path, PYTHON_STATE)?;

        let state = State::load(&path)?;

        assert_eq!(
            state.execution_path,
            Some(PathBuf::from("/home/javier/doodba"))
        );
        assert_eq!(state.current_file.as_deref(), Some("02-fallar"));
        assert_eq!(state.last_command.as_deref(), Some("false"));
        assert_eq!(state.last_line_index, Some(3));
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn creates_empty_state_if_missing() -> io::Result<()> {
        let dir = temp_dir("state-missing")?;
        let path = dir.join("state.json");

        let state = State::load(&path)?;

        assert_eq!(state.current_file, None);
        assert!(path.exists(), "debe crear state.json");
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn recreates_corrupt_state() -> io::Result<()> {
        let dir = temp_dir("state-corrupt")?;
        let path = dir.join("state.json");
        fs::write(&path, "{ esto no es json")?;

        let state = State::load(&path)?;

        assert_eq!(state.last_line_index, None);
        let saved: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        assert!(
            saved.get("current_file").is_some(),
            "debe reescribirlo válido"
        );
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn fills_missing_keys_with_null() -> io::Result<()> {
        let dir = temp_dir("state-partial")?;
        let path = dir.join("state.json");
        fs::write(&path, r#"{"current_file": "01-a"}"#)?;

        let state = State::load(&path)?;

        assert_eq!(state.current_file.as_deref(), Some("01-a"));
        assert_eq!(state.last_command, None);
        let saved = fs::read_to_string(&path)?;
        assert!(saved.contains(r#""last_command": null"#), "{saved}");
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn save_then_load_roundtrip() -> io::Result<()> {
        let dir = temp_dir("state-roundtrip")?;
        let path = dir.join("state.json");
        let mut state = State::load(&path)?;
        state.current_file = Some("01-a".to_string());
        state.last_line_index = Some(2);
        state.save()?;

        assert_eq!(State::load(&path)?, state);
        assert!(
            !dir.join("state.json.tmp").exists(),
            "el temporal se renombra, no se queda"
        );
        fs::remove_dir_all(&dir)
    }

    /// Estado que apunta a `file`:`line` con `command`, sin fichero `state.json` detrás.
    fn pointing_to(file: &str, line: usize, command: &str) -> State {
        State {
            current_file: Some(file.to_string()),
            last_line_index: Some(line),
            last_command: Some(command.to_string()),
            ..State::default()
        }
    }

    #[test]
    fn is_valid_checks_file_line_and_command() -> io::Result<()> {
        let dir = temp_dir("state-valid")?;
        fs::write(dir.join("01-a"), "#> hola\n  ls -la  \npwd\n")?;

        assert!(
            pointing_to("01-a", 2, "ls -la").is_valid(&dir),
            "espacios ignorados"
        );
        assert!(pointing_to("01-a", 3, "pwd").is_valid(&dir));
        assert!(
            !pointing_to("01-a", 3, "ls -la").is_valid(&dir),
            "comando cambiado"
        );
        assert!(
            !pointing_to("01-a", 4, "pwd").is_valid(&dir),
            "línea fuera de rango"
        );
        assert!(
            !pointing_to("01-a", 0, "pwd").is_valid(&dir),
            "la línea 0 no existe"
        );
        assert!(
            !pointing_to("99-z", 1, "pwd").is_valid(&dir),
            "fichero inexistente"
        );
        assert!(!State::default().is_valid(&dir), "estado vacío");
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn summary_shows_placeholders_when_empty() {
        let text = State::default().to_string();
        assert!(text.contains("(sin ruta)"));
        assert!(text.contains("(sin comando)"));
        assert!(!text.contains("Línea"), "sin línea no se muestra la fila");
    }
}
