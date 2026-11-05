//! Interfaz de terminal (TUI): menú de dos paneles y popup de confirmación (spec §7).

mod keys;
mod menu;
mod render;
#[cfg(test)]
mod testdata;

use std::io::{self, IsTerminal};
use std::path::Path;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};

use crate::engine::state::State;
use menu::{Menu, Outcome};

pub use menu::Choice;

/// Abre el menú en la terminal y devuelve la línea elegida, o `None` si se sale sin elegir.
pub fn choose(steps_dir: &Path, state: &State) -> io::Result<Option<Choice>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "bash-resume necesita una terminal interactiva (o usa -y/-o)",
        ));
    }
    let mut menu = Menu::load(steps_dir, state)?;
    let mut terminal = ratatui::try_init()?;
    let result = event_loop(&mut menu, &mut terminal);
    // Se restaura la terminal pase lo que pase en el bucle (el panic lo cubre el hook de ratatui).
    ratatui::try_restore()?;
    result
}

/// Dibuja, espera una tecla y la aplica, hasta que se elige una línea o se sale.
fn event_loop(menu: &mut Menu, terminal: &mut DefaultTerminal) -> io::Result<Option<Choice>> {
    loop {
        terminal.draw(|frame| render::render(menu, frame))?;
        let Event::Key(event) = event::read()? else {
            continue; // cambio de tamaño, ratón…: basta con redibujar
        };
        let Some(key) = keys::key_from(event) else {
            continue;
        };
        match menu.handle_key(key) {
            Outcome::Stay => {}
            Outcome::Run(choice) => return Ok(Some(choice)),
            Outcome::Quit => return Ok(None),
        }
    }
}
