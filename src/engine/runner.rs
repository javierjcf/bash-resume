//! Recorrido de los ficheros de pasos: ejecutar, parar, avanzar el estado y `onerror` (spec §5).

use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;

use super::parser::{Kind, Line, StepFile, list_step_files, load_step_files, parse_file};
use super::state::State;
use super::timings::{Estimate, Timings, progress};
use crate::exec::{Progress, execute};
use crate::log::{Logger, format_duration};

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
/// Lo que tarda cada comando que termina bien se anota en `timings` (spec §6.1); al acabar se registra el tiempo de
/// esta ejecución y, si se completaron todos los pasos, el resumen por fichero.
pub fn run_steps(
    state: &mut State,
    steps_dir: &Path,
    workdir: &Path,
    onerror: &Path,
    log: &mut Logger,
    mode: Mode,
    timings: &mut Timings,
) -> io::Result<Outcome> {
    let start = Instant::now();
    let outcome = walk(state, steps_dir, workdir, onerror, log, mode, timings)?;
    log.set_source(None);
    if outcome == Outcome::Finished {
        log_summary(steps_dir, timings, log);
    }
    log.info(&format!(
        "⏱ Esta ejecución: {}",
        format_duration(start.elapsed())
    ));
    Ok(outcome)
}

/// El recorrido de los pasos: qué se ejecuta, dónde se para y qué posición se guarda.
fn walk(
    state: &mut State,
    steps_dir: &Path,
    workdir: &Path,
    onerror: &Path,
    log: &mut Logger,
    mode: Mode,
    timings: &mut Timings,
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
        // Todos los ficheros tal y como están ahora, solo para numerar y estimar (no cambia qué se ejecuta).
        let all = load_step_files(steps_dir).unwrap_or_default();
        let here = all.iter().position(|f| &f.name == file);

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
                    let progress = here.map(|i| command_progress(&all, i, li, timings));
                    let run = execute(&line.text, workdir, log, progress.as_ref());
                    if !run.ok {
                        state.set_position(file, &line.text, li + 1)?;
                        run_onerror(onerror, workdir, log);
                        return Ok(Outcome::Failed);
                    }
                    timings.record(&line.text, run.elapsed);
                    if let Err(e) = timings.save() {
                        log.warn(&format!("No se pudieron guardar los tiempos: {e}"));
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
        let total = timings.total(&lines);
        if total.timed + total.missing > 0 {
            log.info(&format!("⏱ Fichero {file}: {}", show_total(total)));
        }
    }

    log.set_source(None);
    log.info("Procesado todos los ficheros disponibles.");
    Ok(Outcome::Finished)
}

/// Número de comando y estimación de lo que queda desde la línea `line` del fichero `file`.
fn command_progress(files: &[StepFile], file: usize, line: usize, timings: &Timings) -> Progress {
    let (number, total) = progress(files, file, line);
    Progress {
        number,
        total,
        remaining: timings.remaining(files, file, line).describe(),
    }
}

/// `01:05:30`, con `(2 sin tiempo)` si falta algún comando.
fn show_total(total: Estimate) -> String {
    let mut text = format_duration(total.known);
    if total.missing > 0 {
        text.push_str(&format!(" ({} sin tiempo)", total.missing));
    }
    text
}

/// Tabla con el tiempo de cada fichero y el total, sumando lo guardado (incluye lo hecho antes de reanudar).
fn log_summary(steps_dir: &Path, timings: &Timings, log: &Logger) {
    let Ok(files) = load_step_files(steps_dir) else {
        return;
    };
    let rows: Vec<(&str, Estimate)> = files
        .iter()
        .map(|f| (f.name.as_str(), timings.total(&f.lines)))
        .filter(|(_, t)| t.timed + t.missing > 0)
        .collect();
    if rows.is_empty() {
        return;
    }
    let width = rows
        .iter()
        .map(|(name, _)| name.chars().count())
        .max()
        .unwrap_or(0)
        .max(5);
    let mut sum = Estimate::default();
    log.info("⏱ Resumen de tiempos:");
    for (name, total) in &rows {
        log.info(&format!("   {name:<width$}   {}", show_total(*total)));
        sum.known += total.known;
        sum.timed += total.timed;
        sum.missing += total.missing;
    }
    log.info(&format!("   {:<width$}   {}", "Total", show_total(sum)));
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
        execute(cmd, workdir, log, None);
    }
}
