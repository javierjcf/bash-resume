//! Datos y utilidades compartidos por los tests de la TUI.

use super::menu::{Choice, Key, Menu, Outcome, StepFile};
use crate::engine::parser::Line;
use crate::engine::runner::Mode;
use crate::engine::state::State;

/// Fichero en memoria a partir de su texto.
pub(super) fn step(name: &str, content: &str) -> StepFile {
    StepFile {
        name: name.to_string(),
        lines: content.lines().map(Line::new).collect(),
    }
}

/// Tres ficheros: 01-a (3 líneas), 02-vacio (0) y 03-c (2).
pub(super) fn files() -> Vec<StepFile> {
    vec![
        step("01-a", "#> inicio\necho a\necho b"),
        step("02-vacio", ""),
        step("03-c", "echo c\n#. parada"),
    ]
}

/// Estado (sin fichero detrás) que apunta a `file`:`line` con `command`.
pub(super) fn state_at(file: &str, line: usize, command: &str) -> State {
    // `State { .., ..State::default() }` no vale aquí: `State` tiene un campo privado (`path`)
    // y este módulo no puede nombrarlo. Se parte del estado por defecto y se cambian los públicos.
    let mut state = State::default();
    state.current_file = Some(file.to_string());
    state.last_line_index = Some(line);
    state.last_command = Some(command.to_string());
    state
}

/// Pulsa varias teclas y devuelve el resultado de la última.
pub(super) fn press(menu: &mut Menu, keys: &[Key]) -> Outcome {
    keys.iter()
        .map(|&k| menu.handle_key(k))
        .last()
        .unwrap_or(Outcome::Stay)
}

pub(super) fn run(file: &str, line: usize, command: &str, mode: Mode) -> Outcome {
    Outcome::Run(Choice {
        file: file.to_string(),
        line,
        command: command.to_string(),
        mode,
    })
}
