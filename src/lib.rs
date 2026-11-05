//! bash-resume: ejecutor de pasos de shell reanudable.
//!
//! Crate librería con toda la lógica. El binario (`main.rs`) solo la arranca.

pub mod app;
pub mod cli;
pub mod engine;
pub mod exec;
pub mod log;
pub mod tui;

#[cfg(test)]
mod testutil;

use std::io;
use std::path::Path;

/// Añade la ruta al mensaje de un error de E/S, para que se sepa qué fichero falló.
///
/// Uso: `fs::read_to_string(path).map_err(with_path(path))?`.
pub(crate) fn with_path(path: &Path) -> impl FnOnce(io::Error) -> io::Error + '_ {
    move |e| io::Error::new(e.kind(), format!("{}: {e}", path.display()))
}
