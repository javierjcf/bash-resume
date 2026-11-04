//! Recorrido de los ficheros de pasos: ejecutar, parar, avanzar el estado y `onerror` (spec §5).

use std::fs;
use std::io;
use std::path::Path;

use crate::exec::execute;
use crate::log::Logger;
use crate::parser::{Kind, Line, list_step_files, parse_file};
use crate::state::State;

/// Cuánto ejecutar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Hasta el final, una parada `#.` o un fallo.
    All,
    /// Solo el primer comando que se encuentre.
    Once,
}

/// Cómo terminó el recorrido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Se procesaron todos los ficheros.
    Finished,
    /// Se llegó a una línea `#.`.
    Stopped,
    /// Falló un comando (y se ejecutó `onerror`).
    Failed,
    /// Modo una vez: se ejecutó un comando con éxito.
    OnceDone,
}

/// Ejecuta los pasos de `steps_dir` desde la posición guardada en `state`.
///
/// Cada comando se lanza en `workdir`. Si uno falla, se ejecutan las líneas de `onerror`.
pub fn run_steps(
    state: &mut State,
    steps_dir: &Path,
    workdir: &Path,
    onerror: &Path,
    log: &mut Logger,
    mode: Mode,
) -> io::Result<Outcome> {
    let files = list_step_files(steps_dir)?;
    if files.is_empty() {
        log.warn("No hay ficheros de pasos.");
        return Ok(Outcome::Finished);
    }

    // Fichero y línea (0-based) de arranque: los del estado si ese fichero existe; si no, el principio.
    let saved_file = files
        .iter()
        .position(|f| Some(f) == state.current_file.as_ref());
    let (start_file, start_line) = match saved_file {
        Some(i) => (i, state.last_line_index.map_or(0, |n| n.saturating_sub(1))),
        None => (0, 0),
    };

    for (fi, file) in files.iter().enumerate().skip(start_file) {
        let lines = parse_file(&steps_dir.join(file))?;
        let first = if fi == start_file { start_line } else { 0 };
        log.set_source(Some(file));

        for (li, line) in lines.iter().enumerate().skip(first) {
            match line.kind {
                Kind::Empty | Kind::Comment => {}
                Kind::Log => log.info(&line.text),
                Kind::Stop => {
                    log.warn(&line.text);
                    state.set_position(file, &line.text, li + 1)?;
                    return Ok(Outcome::Stopped);
                }
                Kind::Cmd => {
                    if !execute(&line.text, workdir, log) {
                        state.set_position(file, &line.text, li + 1)?;
                        run_onerror(onerror, workdir, log);
                        return Ok(Outcome::Failed);
                    }
                    // Avanzar al siguiente comando; si no hay más, quedarse en este (como en Python).
                    match next_command(steps_dir, &files, fi, &lines, li)? {
                        Some(next) => state.set_position(&next.file, &next.command, next.line)?,
                        None => state.set_position(file, &line.text, li + 1)?,
                    }
                    if mode == Mode::Once {
                        return Ok(Outcome::OnceDone);
                    }
                }
            }
        }
    }

    log.set_source(None);
    log.info("Procesado todos los ficheros disponibles.");
    Ok(Outcome::Finished)
}

/// Posición de un comando: fichero, línea (1-based) y texto.
struct Position {
    file: String,
    line: usize,
    command: String,
}

/// Busca el siguiente comando tras `lines[li]`: primero en el mismo fichero y después en los siguientes.
fn next_command(
    steps_dir: &Path,
    files: &[String],
    fi: usize,
    lines: &[Line],
    li: usize,
) -> io::Result<Option<Position>> {
    if let Some((i, line)) = first_command(lines, li + 1) {
        return Ok(Some(Position {
            file: files[fi].clone(),
            line: i + 1,
            command: line.text.clone(),
        }));
    }
    for file in &files[fi + 1..] {
        let lines = parse_file(&steps_dir.join(file))?;
        if let Some((i, line)) = first_command(&lines, 0) {
            return Ok(Some(Position {
                file: file.clone(),
                line: i + 1,
                command: line.text.clone(),
            }));
        }
    }
    Ok(None)
}

/// Primer comando de `lines` a partir del índice `from`, con su índice (0-based).
fn first_command(lines: &[Line], from: usize) -> Option<(usize, &Line)> {
    lines
        .iter()
        .enumerate()
        .skip(from)
        .find(|(_, line)| line.kind == Kind::Cmd)
}

/// Ejecuta las líneas de `onerror`, saltando las vacías y los comentarios. Sus fallos se ignoran.
fn run_onerror(path: &Path, workdir: &Path, log: &mut Logger) {
    let Ok(content) = fs::read_to_string(path) else {
        log.warn("No se encontró fichero 'onerror' para ejecutar.");
        return;
    };
    log.set_source(Some("onerror"));
    for cmd in content.lines().map(str::trim) {
        if cmd.is_empty() || cmd.starts_with('#') {
            continue;
        }
        log.info(&format!("Ejecutando comando onerror: {cmd}"));
        execute(cmd, workdir, log);
    }
}
