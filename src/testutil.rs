//! Utilidades compartidas por los tests unitarios. Solo se compila con `cargo test`.

use std::fs;
use std::io;
use std::path::PathBuf;

/// Crea un directorio temporal vacío y único para cada test (se ejecutan en paralelo).
pub(crate) fn temp_dir(name: &str) -> io::Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("stepwise-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}
