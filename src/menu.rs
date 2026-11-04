//! Menú interactivo para elegir desde qué línea ejecutar (spec §7.2).
//!
//! Este módulo separa la lógica (estado y teclas, testeable sin terminal) del dibujo con ratatui.

use std::io::{self, IsTerminal};
use std::path::Path;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use crate::parser::{Kind, Line, list_step_files, parse_file};
use crate::runner::Mode;
use crate::state::State;

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

/// Un fichero de pasos con sus líneas, cargado al abrir el menú.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepFile {
    pub name: String,
    pub lines: Vec<Line>,
}

/// Estado del menú: qué ficheros hay, qué está seleccionado y qué panel tiene el foco.
#[derive(Debug)]
pub struct Menu {
    files: Vec<StepFile>,
    panel: Panel,
    /// Fichero seleccionado (índice en `files`).
    file: usize,
    /// Línea seleccionada dentro del fichero (índice 0-based).
    line: usize,
    /// Posición guardada en el estado, si sigue existiendo: (fichero, línea 0-based).
    saved: Option<(usize, usize)>,
}

impl Menu {
    /// Carga todos los ficheros de `steps_dir` y abre el menú en la posición del estado.
    pub fn load(steps_dir: &Path, state: &State) -> io::Result<Self> {
        let mut files = Vec::new();
        for name in list_step_files(steps_dir)? {
            let lines = parse_file(&steps_dir.join(&name))?;
            files.push(StepFile { name, lines });
        }
        Ok(Self::new(files, state))
    }

    /// Abre el menú sobre `files`, con el fichero del estado seleccionado y,
    /// si su línea sigue teniendo el mismo comando, también esa línea (como en Python).
    pub fn new(files: Vec<StepFile>, state: &State) -> Self {
        let file = state
            .current_file
            .as_ref()
            .and_then(|name| files.iter().position(|f| &f.name == name))
            .unwrap_or(0);
        let saved = saved_position(&files, state);
        let mut menu = Self {
            files,
            panel: Panel::Files,
            file: 0,
            line: 0,
            saved,
        };
        menu.select_file(file);
        menu
    }

    /// Aplica una tecla y dice si hay que seguir, ejecutar o salir.
    pub fn handle_key(&mut self, key: Key) -> Outcome {
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
            (Panel::Lines, Key::Enter | Key::Char('y' | 's')) => return self.choose(Mode::All),
            (Panel::Lines, Key::Char('o')) => return self.choose(Mode::Once),

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
    fn current_lines(&self) -> &[Line] {
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

/// Abre el menú en la terminal y devuelve la línea elegida, o `None` si se sale sin elegir.
pub fn choose(steps_dir: &Path, state: &State) -> io::Result<Option<Choice>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "el menú necesita una terminal interactiva",
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
        terminal.draw(|frame| render(menu, frame))?;
        let Event::Key(event) = event::read()? else {
            continue; // cambio de tamaño, ratón…: basta con redibujar
        };
        let Some(key) = key_from(event) else {
            continue;
        };
        match menu.handle_key(key) {
            Outcome::Stay => {}
            Outcome::Run(choice) => return Ok(Some(choice)),
            Outcome::Quit => return Ok(None),
        }
    }
}

/// Dibuja el menú: ficheros a la izquierda, líneas del fichero seleccionado a la derecha y una barra abajo.
///
/// Solo lee el estado del menú; no decide nada (eso es `handle_key`).
fn render(menu: &Menu, frame: &mut Frame) {
    let [main, bar] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());
    let [files_area, lines_area] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(main);

    // Panel de ficheros: `●` en el del estado guardado.
    let files: Vec<ListItem> = menu
        .files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mark = if menu.saved.is_some_and(|(file, _)| file == i) {
                "● "
            } else {
                "  "
            };
            ListItem::new(format!("{mark}{}", f.name))
        })
        .collect();
    let focused = menu.panel == Panel::Files;
    let files = List::new(files)
        .block(panel(" Ficheros ", focused))
        .highlight_style(highlight(focused))
        .highlight_symbol("> ");
    let mut files_state = ListState::default().with_selected(Some(menu.file));
    frame.render_stateful_widget(files, files_area, &mut files_state);

    // Panel de líneas del fichero seleccionado, coloreadas por tipo.
    let lines = menu.current_lines();
    let items: Vec<ListItem> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let saved = menu.saved == Some((menu.file, i));
            ListItem::new(format!("[{:03}] {}", i + 1, line.text))
                .style(line_style(line.kind, saved))
        })
        .collect();
    let title = match menu.files.get(menu.file) {
        Some(f) if f.lines.is_empty() => format!(" {} (vacío) ", f.name),
        Some(f) => format!(" {} ", f.name),
        None => " (no hay ficheros de pasos) ".to_string(),
    };
    let focused = menu.panel == Panel::Lines;
    let lines_list = List::new(items)
        .block(panel(&title, focused))
        .highlight_style(highlight(focused))
        .highlight_symbol("> ");
    let mut lines_state =
        ListState::default().with_selected((!lines.is_empty()).then_some(menu.line));
    frame.render_stateful_widget(lines_list, lines_area, &mut lines_state);

    frame.render_widget(Paragraph::new(status_text(menu)), bar);
}

/// Marco de un panel; el que tiene el foco, en cian.
fn panel(title: &str, focused: bool) -> Block<'_> {
    let color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    Block::bordered()
        .title(title)
        .border_style(Style::new().fg(color))
}

/// Estilo de la fila seleccionada: llamativo en el panel con el foco, discreto en el otro.
fn highlight(focused: bool) -> Style {
    if focused {
        Style::new()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::new().add_modifier(Modifier::REVERSED)
    }
}

/// Color de una línea según su tipo; la guardada en el estado, en amarillo.
fn line_style(kind: Kind, saved: bool) -> Style {
    if saved {
        return Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    }
    match kind {
        Kind::Log => Style::new().fg(Color::Green),
        Kind::Stop => Style::new().fg(Color::Red),
        Kind::Comment => Style::new().fg(Color::DarkGray),
        Kind::Cmd | Kind::Empty => Style::new(),
    }
}

/// Barra inferior: la línea seleccionada y la ayuda de teclas.
fn status_text(menu: &Menu) -> String {
    let help = "↑↓/jk mover · Tab panel · Enter/y ejecutar · o una vez · q salir";
    match (
        menu.files.get(menu.file),
        menu.current_lines().get(menu.line),
    ) {
        (Some(file), Some(line)) => format!(
            " {}:{} → {}  │  {help}",
            file.name,
            menu.line + 1,
            line.text
        ),
        _ => format!(" {help}"),
    }
}

/// Traduce una tecla de crossterm a nuestra `Key`. Las que el menú no usa dan `None`.
///
/// En modo raw, Ctrl+C no genera una señal: llega como una tecla más, así que se trata como `q`.
fn key_from(event: KeyEvent) -> Option<Key> {
    if event.kind != KeyEventKind::Press {
        return None;
    }
    if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c') {
        return Some(Key::Char('q'));
    }
    let key = match event.code {
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Tab => Key::Tab,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Esc,
        KeyCode::Char(c) => Key::Char(c.to_ascii_lowercase()),
        _ => return None,
    };
    Some(key)
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
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::fs;

    /// Fichero en memoria a partir de su texto.
    fn step(name: &str, content: &str) -> StepFile {
        StepFile {
            name: name.to_string(),
            lines: content.lines().map(Line::new).collect(),
        }
    }

    /// Tres ficheros: 01-a (3 líneas), 02-vacio (0) y 03-c (2).
    fn files() -> Vec<StepFile> {
        vec![
            step("01-a", "#> inicio\necho a\necho b"),
            step("02-vacio", ""),
            step("03-c", "echo c\n#. parada"),
        ]
    }

    /// Estado (sin fichero detrás) que apunta a `file`:`line` con `command`.
    fn state_at(file: &str, line: usize, command: &str) -> State {
        // `State { .., ..State::default() }` no vale aquí: `State` tiene un campo privado (`path`)
        // y este módulo no puede nombrarlo. Se parte del estado por defecto y se cambian los públicos.
        let mut state = State::default();
        state.current_file = Some(file.to_string());
        state.last_line_index = Some(line);
        state.last_command = Some(command.to_string());
        state
    }

    /// Pulsa varias teclas y devuelve el resultado de la última.
    fn press(menu: &mut Menu, keys: &[Key]) -> Outcome {
        keys.iter()
            .map(|&k| menu.handle_key(k))
            .last()
            .unwrap_or(Outcome::Stay)
    }

    fn run(file: &str, line: usize, command: &str, mode: Mode) -> Outcome {
        Outcome::Run(Choice {
            file: file.to_string(),
            line,
            command: command.to_string(),
            mode,
        })
    }

    #[test]
    fn opens_at_saved_position() {
        let mut menu = Menu::new(files(), &state_at("03-c", 2, "#. parada"));
        assert_eq!((menu.file, menu.line, menu.saved), (2, 1, Some((2, 1))));
        assert_eq!(
            press(&mut menu, &[Key::Enter, Key::Enter]),
            run("03-c", 2, "#. parada", Mode::All)
        );
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
    fn line_navigation_wraps_and_choose_returns_1_based_line() {
        let mut menu = Menu::new(files(), &State::default());
        press(&mut menu, &[Key::Enter, Key::Char('k')]);
        assert_eq!(
            (menu.panel, menu.line),
            (Panel::Lines, 2),
            "↑ desde la primera va a la última"
        );
        assert_eq!(
            press(&mut menu, &[Key::Char('y')]),
            run("01-a", 3, "echo b", Mode::All)
        );
    }

    #[test]
    fn once_and_spanish_yes() {
        let mut menu = Menu::new(files(), &State::default());
        assert_eq!(
            press(&mut menu, &[Key::Tab, Key::Down, Key::Char('o')]),
            run("01-a", 2, "echo a", Mode::Once)
        );
        assert_eq!(
            press(&mut menu, &[Key::Char('s')]),
            run("01-a", 2, "echo a", Mode::All)
        );
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

    /// Dibuja el menú en una terminal falsa de `width`×`height` y devuelve la pantalla como texto.
    fn draw(menu: &Menu, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(menu, frame)).unwrap();
        terminal.backend().to_string()
    }

    #[test]
    fn render_shows_files_lines_and_status_bar() {
        let menu = Menu::new(files(), &state_at("03-c", 2, "#. parada"));
        let screen = draw(&menu, 100, 8);
        assert!(
            screen.contains("● 03-c"),
            "marca del fichero guardado:\n{screen}"
        );
        assert!(
            screen.contains("[002] #. parada"),
            "líneas numeradas:\n{screen}"
        );
        assert!(
            screen.contains("03-c:2 → #. parada"),
            "barra inferior:\n{screen}"
        );
        assert!(screen.contains("02-vacio"), "{screen}");
    }

    #[test]
    fn render_without_files_does_not_panic() {
        let screen = draw(&Menu::new(Vec::new(), &State::default()), 60, 6);
        assert!(screen.contains("no hay ficheros"), "{screen}");
    }

    #[test]
    fn translates_crossterm_keys() {
        let press = |code, mods| key_from(KeyEvent::new(code, mods));
        assert_eq!(press(KeyCode::Down, KeyModifiers::NONE), Some(Key::Down));
        assert_eq!(
            press(KeyCode::Char('Y'), KeyModifiers::SHIFT),
            Some(Key::Char('y'))
        );
        assert_eq!(
            press(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(Key::Char('q'))
        );
        assert_eq!(press(KeyCode::F(1), KeyModifiers::NONE), None);
        let release =
            KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Release);
        assert_eq!(
            key_from(release),
            None,
            "solo cuenta al pulsar, no al soltar"
        );
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
