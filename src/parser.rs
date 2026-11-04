//! Ficheros de pasos: tipos de línea, clasificación y lectura (spec §3).

use std::fs;
use std::io;
use std::path::Path;

/// Tipo de una línea de un fichero de pasos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Comando a ejecutar.
    Cmd,
    /// `#>`: mensaje que va al log.
    Log,
    /// `#.`: parada de la ejecución.
    Stop,
    /// `#`: comentario, se ignora.
    Comment,
    /// Línea vacía o solo con espacios.
    Empty,
}

/// Una línea de un fichero de pasos, ya clasificada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: Kind,
    /// Texto original de la línea, sin el salto de línea final.
    pub text: String,
}

impl Line {
    /// Crea una línea a partir de su texto, clasificándolo.
    pub fn new(text: &str) -> Self {
        Self {
            kind: classify(text),
            text: text.to_string(),
        }
    }
}

/// Clasifica una línea por su prefijo, ignorando los espacios al inicio y al final.
///
/// El orden importa: `#>` y `#.` deben comprobarse antes que `#`.
pub fn classify(line: &str) -> Kind {
    match line.trim() {
        "" => Kind::Empty,
        t if t.starts_with("#>") => Kind::Log,
        t if t.starts_with("#.") => Kind::Stop,
        t if t.starts_with('#') => Kind::Comment,
        _ => Kind::Cmd,
    }
}

/// Lista los ficheros de pasos de `dir` en orden alfabético.
///
/// Ignora los ocultos (que empiezan por `.`), los subdirectorios y los nombres que no son UTF-8.
pub fn list_step_files(dir: &Path) -> io::Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.path().is_file() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !name.starts_with('.') {
            files.push(name);
        }
    }
    files.sort();
    Ok(files)
}

/// Lee un fichero de pasos y devuelve sus líneas clasificadas, sin el salto de línea.
pub fn parse_file(path: &Path) -> io::Result<Vec<Line>> {
    let content = fs::read_to_string(path)?;
    Ok(content.lines().map(Line::new).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;

    #[test]
    fn classifies_each_kind() {
        assert_eq!(classify("docker compose up -d"), Kind::Cmd);
        assert_eq!(classify("#> EMPEZANDO SETUP"), Kind::Log);
        assert_eq!(classify("#. Parada manual"), Kind::Stop);
        assert_eq!(classify("# comentario"), Kind::Comment);
        assert_eq!(classify(""), Kind::Empty);
    }

    #[test]
    fn whitespace_only_is_empty() {
        assert_eq!(classify("   "), Kind::Empty);
        assert_eq!(classify("\t"), Kind::Empty);
    }

    #[test]
    fn ignores_surrounding_whitespace() {
        assert_eq!(classify("   #> log con sangría"), Kind::Log);
        assert_eq!(classify("  ls -la  "), Kind::Cmd);
    }

    #[test]
    fn special_markers_take_precedence_over_comment() {
        assert_eq!(classify("#>"), Kind::Log);
        assert_eq!(classify("#."), Kind::Stop);
        assert_eq!(classify("#"), Kind::Comment);
        assert_eq!(classify("#!/bin/bash"), Kind::Comment);
    }

    #[test]
    fn hash_inside_command_is_still_cmd() {
        assert_eq!(classify("echo '#> no soy log'"), Kind::Cmd);
    }

    #[test]
    fn line_keeps_original_text() {
        let line = Line::new("  ls -la ");
        assert_eq!(line.kind, Kind::Cmd);
        assert_eq!(line.text, "  ls -la ");
    }

    #[test]
    fn lists_files_sorted_without_hidden_or_dirs() -> io::Result<()> {
        let dir = temp_dir("list")?;
        fs::write(dir.join("02-b"), "")?;
        fs::write(dir.join("01-a"), "")?;
        fs::write(dir.join(".oculto"), "")?;
        fs::create_dir(dir.join("subdir"))?;

        assert_eq!(list_step_files(&dir)?, vec!["01-a", "02-b"]);

        fs::remove_dir_all(&dir)
    }

    #[test]
    fn list_fails_if_dir_missing() {
        assert!(list_step_files(Path::new("/no/existe")).is_err());
    }

    #[test]
    fn parses_file_into_classified_lines() -> io::Result<()> {
        let dir = temp_dir("parse")?;
        let file = dir.join("01-a");
        fs::write(&file, "#> hola\nls -la\r\n\n# nota\n")?;

        let lines = parse_file(&file)?;

        let kinds: Vec<Kind> = lines.iter().map(|l| l.kind).collect();
        assert_eq!(kinds, [Kind::Log, Kind::Cmd, Kind::Empty, Kind::Comment]);
        assert_eq!(lines[1].text, "ls -la");
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn parse_fails_if_file_missing() {
        assert!(parse_file(Path::new("/no/existe")).is_err());
    }
}
