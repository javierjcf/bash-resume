//! Traducir las teclas de crossterm a las nuestras (`Key`).

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::menu::Key;

/// Traduce una tecla de crossterm a nuestra `Key`. Las que el menú no usa dan `None`.
///
/// En modo raw, Ctrl+C no genera una señal: llega como una tecla más, así que se trata como `q`.
pub(super) fn key_from(event: KeyEvent) -> Option<Key> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
