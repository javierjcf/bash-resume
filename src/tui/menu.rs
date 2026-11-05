//! Estado del menú y teclas: qué está seleccionado y qué hace cada tecla. Sin terminal ni dibujo.

use std::io;
use std::path::{Path, PathBuf};

use crate::engine::parser::{Line, load_step_files};
use crate::engine::runner::Mode;
use crate::engine::state::State;
use crate::engine::timings::Timings;

pub use crate::engine::parser::StepFile;

/// Panel con el foco.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Files,
    Lines,
}

/// Teclas que entiende el menú, independientes de la librería de terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Tab,
    Enter,
    Esc,
    Char(char),
}

/// Línea elegida para empezar a ejecutar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub file: String,
    /// Número de línea, 1-based (como en el estado).
    pub line: usize,
    pub command: String,
    pub mode: Mode,
}

/// Qué pasa tras pulsar una tecla.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Seguir en el menú.
    Stay,
    /// Ejecutar desde la línea elegida.
    Run(Choice),
    /// Salir sin ejecutar.
    Quit,
}

/// Estado del menú: qué ficheros hay, qué está seleccionado y qué panel tiene el foco.
#[derive(Debug)]
pub struct Menu {
    pub(super) files: Vec<StepFile>,
    pub(super) panel: Panel,
    /// Fichero seleccionado (índice en `files`).
    pub(super) file: usize,
    /// Línea seleccionada dentro del fichero (índice 0-based).
    pub(super) line: usize,
    /// Posición guardada en el estado, si sigue existiendo: (fichero, línea 0-based).
    pub(super) saved: Option<(usize, usize)>,
    /// `true` mientras se muestra el popup de confirmación de la línea seleccionada.
    pub(super) confirm: bool,
    /// Directorio de trabajo y carpeta de pasos, para mostrarlos en la cabecera y el popup.
    pub(super) workdir: Option<PathBuf>,
    pub(super) steps_dir: Option<PathBuf>,
    /// Lo que tardó cada comando la última vez (vacío si no hay `times.json`).
    pub(super) timings: Timings,
}

impl Menu {
    /// Carga todos los ficheros de `steps_dir` y abre el menú en la posición del estado.
    pub fn load(steps_dir: &Path, state: &State) -> io::Result<Self> {
        Ok(Self::new(load_step_files(steps_dir)?, state))
    }

    /// Abre el menú sobre `files`, con el fichero del estado seleccionado y,
    /// si su línea sigue teniendo el mismo comando, también esa línea (como en Python).
    ///
    /// Con una posición guardada válida, arranca en el panel de líneas con el cursor sobre ella; el popup de
    /// confirmación solo se abre al elegir (`Enter`).
    pub fn new(files: Vec<StepFile>, state: &State) -> Self {
        let file = state
            .current_file
            .as_ref()
            .and_then(|name| files.iter().position(|f| &f.name == name))
            .unwrap_or(0);
        let saved = saved_position(&files, state);
        let mut menu = Self {
            files,
            panel: if saved.is_some() {
                Panel::Lines
            } else {
                Panel::Files
            },
            file: 0,
            line: 0,
            saved,
            confirm: false,
            workdir: state.execution_path.clone(),
            steps_dir: state.steps_path.clone(),
            timings: Timings::default(),
        };
        menu.select_file(file);
        menu
    }

    /// Añade los tiempos guardados, para mostrarlos junto a las líneas, los ficheros y en el popup.
    pub fn with_timings(mut self, timings: Timings) -> Self {
        self.timings = timings;
        self
    }

    /// Aplica una tecla y dice si hay que seguir, ejecutar o salir.
    ///
    /// Solo se ejecuta desde el popup de confirmación: navegando, elegir una línea lo abre.
    pub fn handle_key(&mut self, key: Key) -> Outcome {
        // Popup abierto: solo cuentan sus teclas (estado "modal").
        if self.confirm {
            match key {
                Key::Enter | Key::Char('y' | 's') => return self.choose(Mode::All),
                Key::Char('o') => return self.choose(Mode::Once),
                Key::Esc | Key::Char('m' | 'n') => self.confirm = false,
                Key::Char('q') => return Outcome::Quit,
                _ => {}
            }
            return Outcome::Stay;
        }

        let files = self.files.len();
        let lines = self.current_lines().len();
        match (self.panel, key) {
            (_, Key::Char('q')) | (Panel::Files, Key::Esc) => return Outcome::Quit,

            (Panel::Files, Key::Up | Key::Char('k')) => self.select_file(prev(self.file, files)),
            (Panel::Files, Key::Down | Key::Char('j')) => self.select_file(next(self.file, files)),
            // No se entra en un fichero vacío: no hay línea que elegir.
            (Panel::Files, Key::Enter | Key::Right | Key::Tab) if lines > 0 => {
                self.panel = Panel::Lines
            }

            (Panel::Lines, Key::Up | Key::Char('k')) => self.line = prev(self.line, lines),
            (Panel::Lines, Key::Down | Key::Char('j')) => self.line = next(self.line, lines),
            (Panel::Lines, Key::Esc | Key::Left | Key::Tab) => self.panel = Panel::Files,
            (Panel::Lines, Key::Enter | Key::Char('y' | 's' | 'o')) if lines > 0 => {
                self.confirm = true
            }

            _ => {}
        }
        Outcome::Stay
    }

    /// Selecciona un fichero y coloca el cursor en su línea guardada, o en la primera.
    fn select_file(&mut self, file: usize) {
        self.file = file;
        self.line = match self.saved {
            Some((f, line)) if f == file => line,
            _ => 0,
        };
    }

    /// Líneas del fichero seleccionado (vacío si no hay ficheros).
    pub(super) fn current_lines(&self) -> &[Line] {
        self.files.get(self.file).map_or(&[], |f| &f.lines)
    }

    /// Construye la elección con la línea seleccionada.
    fn choose(&self, mode: Mode) -> Outcome {
        let Some(file) = self.files.get(self.file) else {
            return Outcome::Stay;
        };
        let Some(line) = file.lines.get(self.line) else {
            return Outcome::Stay;
        };
        Outcome::Run(Choice {
            file: file.name.clone(),
            line: self.line + 1,
            command: line.text.clone(),
            mode,
        })
    }
}

/// Posición del estado (fichero, línea 0-based) si el fichero existe y la línea tiene el mismo comando.
fn saved_position(files: &[StepFile], state: &State) -> Option<(usize, usize)> {
    let name = state.current_file.as_ref()?;
    let file = files.iter().position(|f| &f.name == name)?;
    let line = state.last_line_index?.checked_sub(1)?;
    let text = files[file].lines.get(line)?.text.trim();
    let command = state.last_command.as_deref()?.trim();
    (text == command).then_some((file, line))
}

/// Índice siguiente, volviendo al principio al pasar del final.
fn next(i: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (i + 1) % len }
}

/// Índice anterior, saltando al final al pasar del principio.
fn prev(i: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (i + len - 1) % len }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;
    use crate::tui::testdata::{files, press, run, state_at};
    use std::fs;

    #[test]
    fn opens_at_saved_line_and_enter_asks_for_confirmation() {
        let mut menu = Menu::new(files(), &state_at("03-c", 2, "#. parada"));
        assert_eq!((menu.file, menu.line, menu.saved), (2, 1, Some((2, 1))));
        assert_eq!(menu.panel, Panel::Lines, "arranca en el panel de líneas");
        assert!(!menu.confirm, "sin popup al abrir");
        assert_eq!(
            press(&mut menu, &[Key::Enter]),
            Outcome::Stay,
            "Enter abre el popup"
        );
        assert!(menu.confirm);
        assert_eq!(
            press(&mut menu, &[Key::Enter]),
            run("03-c", 2, "#. parada", Mode::All)
        );
    }

    #[test]
    fn without_saved_position_starts_browsing() {
        let menu = Menu::new(files(), &state_at("03-c", 2, "comando que ya no está"));
        assert!(!menu.confirm);
    }

    #[test]
    fn saved_file_with_changed_line_opens_at_its_first_line() {
        let menu = Menu::new(files(), &state_at("03-c", 2, "comando que ya no está"));
        assert_eq!((menu.file, menu.line, menu.saved), (2, 0, None));
    }

    #[test]
    fn unknown_saved_file_opens_at_the_beginning() {
        let menu = Menu::new(files(), &state_at("99-borrado", 1, "x"));
        assert_eq!((menu.panel, menu.file, menu.line), (Panel::Files, 0, 0));
    }

    #[test]
    fn file_navigation_wraps_around() {
        let mut menu = Menu::new(files(), &State::default());
        press(&mut menu, &[Key::Up]);
        assert_eq!(menu.file, 2, "↑ desde el primero va al último");
        press(&mut menu, &[Key::Down, Key::Char('j')]);
        assert_eq!(menu.file, 1);
    }

    #[test]
    fn choosing_a_line_opens_the_popup_and_does_not_run() {
        let mut menu = Menu::new(files(), &State::default());
        press(&mut menu, &[Key::Enter, Key::Char('k')]);
        assert_eq!(
            (menu.panel, menu.line),
            (Panel::Lines, 2),
            "↑ desde la primera va a la última"
        );

        assert_eq!(
            press(&mut menu, &[Key::Enter]),
            Outcome::Stay,
            "navegando, Enter no ejecuta"
        );
        assert!(menu.confirm);
        assert_eq!(
            press(&mut menu, &[Key::Enter]),
            run("01-a", 3, "echo b", Mode::All)
        );
    }

    #[test]
    fn popup_keys_once_yes_and_back() {
        let mut menu = Menu::new(files(), &State::default());
        press(&mut menu, &[Key::Tab, Key::Down, Key::Char('o')]);
        assert!(menu.confirm, "`o` navegando también abre el popup");
        assert_eq!(
            press(&mut menu, &[Key::Char('o')]),
            run("01-a", 2, "echo a", Mode::Once)
        );
        assert_eq!(
            press(&mut menu, &[Key::Char('s')]),
            run("01-a", 2, "echo a", Mode::All)
        );

        assert_eq!(press(&mut menu, &[Key::Esc]), Outcome::Stay);
        assert!(!menu.confirm, "Esc cierra el popup y vuelve a navegar");
        assert_eq!(press(&mut menu, &[Key::Down]), Outcome::Stay);
        assert_eq!(menu.line, 2, "ya navegando, las flechas mueven el cursor");
    }

    #[test]
    fn popup_ignores_navigation_and_q_quits() {
        let mut menu = Menu::new(files(), &state_at("01-a", 2, "echo a"));
        press(&mut menu, &[Key::Enter]); // abrir el popup
        assert_eq!(press(&mut menu, &[Key::Down, Key::Tab]), Outcome::Stay);
        assert_eq!(
            (menu.line, menu.panel, menu.confirm),
            (1, Panel::Lines, true),
            "no se mueve"
        );
        assert_eq!(press(&mut menu, &[Key::Char('q')]), Outcome::Quit);
    }

    #[test]
    fn changing_file_resets_line_cursor() {
        let mut menu = Menu::new(files(), &State::default());
        press(
            &mut menu,
            &[
                Key::Enter,
                Key::Down,
                Key::Down,
                Key::Esc,
                Key::Down,
                Key::Down,
            ],
        );
        assert_eq!((menu.panel, menu.file, menu.line), (Panel::Files, 2, 0));
    }

    #[test]
    fn empty_file_cannot_be_entered() {
        let mut menu = Menu::new(files(), &State::default());
        press(&mut menu, &[Key::Down, Key::Enter, Key::Char('y')]);
        assert_eq!(menu.panel, Panel::Files, "no entra en 02-vacio");
        assert_eq!(press(&mut menu, &[Key::Enter]), Outcome::Stay);
    }

    #[test]
    fn esc_goes_back_then_quits_and_q_quits_anywhere() {
        let mut menu = Menu::new(files(), &State::default());
        assert_eq!(press(&mut menu, &[Key::Enter, Key::Esc]), Outcome::Stay);
        assert_eq!(menu.panel, Panel::Files);
        assert_eq!(press(&mut menu, &[Key::Esc]), Outcome::Quit);
        assert_eq!(
            press(&mut menu, &[Key::Enter, Key::Char('q')]),
            Outcome::Quit
        );
    }

    #[test]
    fn no_files_does_not_panic() {
        let mut menu = Menu::new(Vec::new(), &State::default());
        let keys = [
            Key::Up,
            Key::Down,
            Key::Enter,
            Key::Char('y'),
            Key::Char('o'),
        ];
        assert_eq!(press(&mut menu, &keys), Outcome::Stay);
        assert_eq!(press(&mut menu, &[Key::Char('q')]), Outcome::Quit);
    }

    #[test]
    fn load_reads_steps_dir() -> io::Result<()> {
        let dir = temp_dir("menu-load")?;
        fs::write(dir.join("02-b"), "echo b\n")?;
        fs::write(dir.join("01-a"), "echo a\n")?;

        let menu = Menu::load(&dir, &State::default())?;

        let names: Vec<&str> = menu.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["01-a", "02-b"]);
        fs::remove_dir_all(&dir)
    }
}
