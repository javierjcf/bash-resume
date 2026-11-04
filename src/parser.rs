//! Ficheros de pasos: tipos de línea y clasificación (spec §3).

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
