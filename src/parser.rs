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
