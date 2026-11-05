//! Dibujo de la TUI con ratatui: cabecera, paneles, barra y popup de confirmación.

use std::path::PathBuf;

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line as TextLine, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap};

use super::menu::{Menu, Panel};
use crate::engine::parser::Kind;

/// Dibuja el menú: ficheros a la izquierda, líneas del fichero seleccionado a la derecha y una barra abajo.
///
/// Solo lee el estado del menú; no decide nada (eso es `handle_key`).
pub(super) fn render(menu: &Menu, frame: &mut Frame) {
    let [header, main, bar] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    frame.render_widget(Paragraph::new(header_line(menu)), header);
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

    if menu.confirm {
        render_confirm(menu, frame);
    }
}

/// Cabecera: directorio de trabajo, carpeta de pasos y si el estado guardado es válido.
fn header_line(menu: &Menu) -> TextLine<'static> {
    let status = if menu.saved.is_some() {
        Span::styled("● estado válido", Style::new().fg(Color::Green))
    } else {
        Span::styled(
            "⚠ estado no válido: elige una línea",
            Style::new().fg(Color::Yellow),
        )
    };
    TextLine::from(vec![
        Span::raw(format!(
            " Workdir: {}  ·  Pasos: {}  ·  ",
            show_path(&menu.workdir),
            show_path(&menu.steps_dir)
        )),
        status,
    ])
}

/// Popup de confirmación centrado, encima de los paneles: qué se va a ejecutar y con qué teclas.
fn render_confirm(menu: &Menu, frame: &mut Frame) {
    let (Some(file), Some(line)) = (
        menu.files.get(menu.file),
        menu.current_lines().get(menu.line),
    ) else {
        return;
    };
    let key = |k: &'static str| {
        Span::styled(k, Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    };
    let label = |l: &'static str| Span::styled(l, Style::new().fg(Color::LightBlue));
    let text = vec![
        TextLine::from(vec![
            label("Fichero  "),
            Span::raw(format!("{} · línea {}", file.name, menu.line + 1)),
        ]),
        TextLine::from(Span::styled(
            format!("▶ {}", line.text.trim()),
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        TextLine::from(vec![
            label("Workdir  "),
            Span::raw(show_path(&menu.workdir)),
        ]),
        TextLine::from(vec![
            label("Pasos    "),
            Span::raw(show_path(&menu.steps_dir)),
        ]),
        TextLine::default(),
        TextLine::from(vec![
            key("[Enter]"),
            Span::raw(" continuar   "),
            key("[o]"),
            Span::raw(" una vez   "),
            key("[Esc]"),
            Span::raw(" elegir otra línea   "),
            key("[q]"),
            Span::raw(" salir"),
        ]),
    ];

    // Alto: 2 de borde + 5 líneas fijas + las que ocupe el comando al partirse.
    let area = frame.area();
    let width = area.width.saturating_sub(4).min(90);
    let inner = usize::from(width.saturating_sub(2)).max(1);
    let command_rows = (line.text.trim().chars().count() + 2).div_ceil(inner) as u16;
    let popup = centered(area, width, 7 + command_rows);

    frame.render_widget(Clear, popup);
    let block = Block::bordered()
        .title(" ¿Ejecutar? ")
        .border_style(Style::new().fg(Color::Yellow));
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        popup,
    );
}

/// Ruta para mostrar, o `?` si no hay.
fn show_path(path: &Option<PathBuf>) -> String {
    path.as_ref()
        .map_or("?".to_string(), |p| p.display().to_string())
}

/// Rectángulo de `width` × `height` centrado en `area` (recortado si no cabe).
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let [row] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    let [cell] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(row);
    cell
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
    let help = "↑↓/jk mover · Tab panel · Enter elegir · q salir";
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::State;
    use crate::tui::menu::Key;
    use crate::tui::testdata::{files, press, state_at};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// Dibuja el menú en una terminal falsa de `width`×`height` y devuelve la pantalla como texto.
    fn draw(menu: &Menu, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(menu, frame)).unwrap();
        terminal.backend().to_string()
    }

    #[test]
    fn render_shows_files_lines_and_status_bar() {
        let menu = Menu::new(files(), &state_at("03-c", 2, "#. parada"));
        let screen = draw(&menu, 100, 9);
        assert!(
            !screen.contains("¿Ejecutar?"),
            "sin popup al abrir:\n{screen}"
        );
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
        assert!(screen.contains("● estado válido"), "cabecera:\n{screen}");
        assert!(screen.contains("02-vacio"), "{screen}");
    }

    #[test]
    fn render_shows_confirmation_popup() {
        let mut state = state_at("03-c", 2, "#. parada");
        state.execution_path = Some(PathBuf::from("/opt/odoo/doodba"));
        let mut menu = Menu::new(files(), &state);
        press(&mut menu, &[Key::Enter]); // elegir la línea guardada abre el popup
        let screen = draw(&menu, 100, 14);
        assert!(screen.contains("¿Ejecutar?"), "{screen}");
        assert!(
            screen.contains("▶ #. parada"),
            "comando destacado:\n{screen}"
        );
        assert!(screen.contains("03-c · línea 2"), "{screen}");
        assert!(screen.contains("/opt/odoo/doodba"), "workdir:\n{screen}");
        assert!(screen.contains("[Esc] elegir otra línea"), "{screen}");
    }

    #[test]
    fn render_warns_when_state_is_not_valid() {
        let screen = draw(&Menu::new(files(), &State::default()), 100, 9);
        assert!(screen.contains("⚠ estado no válido"), "{screen}");
        assert!(!screen.contains("¿Ejecutar?"), "{screen}");
    }

    #[test]
    fn render_without_files_does_not_panic() {
        let screen = draw(&Menu::new(Vec::new(), &State::default()), 60, 6);
        assert!(screen.contains("no hay ficheros"), "{screen}");
    }
}
