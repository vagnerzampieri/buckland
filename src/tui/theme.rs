//! Theme — converts the user's `ui.accent_color` config string into a
//! ratatui `Color`, plus shortcut helpers for the canonical styles
//! (selected row, dim metadata, bold totals, slow-blinking running marker).

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub accent: Color,
}

impl Theme {
    pub fn from_config_accent(name: &str) -> Self {
        Self {
            accent: parse_color(name).unwrap_or(Color::Cyan),
        }
    }

    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::REVERSED)
    }

    pub fn running(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::SLOW_BLINK)
    }

    pub fn dim(&self) -> Style {
        Style::default().add_modifier(Modifier::DIM)
    }

    pub fn total(&self) -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }
}

fn parse_color(name: &str) -> Option<Color> {
    match name.trim().to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_cyan_when_unknown() {
        let t = Theme::from_config_accent("not-a-color");
        assert_eq!(t.accent, Color::Cyan);
    }

    #[test]
    fn parses_named_ansi_colors_case_insensitively() {
        assert_eq!(Theme::from_config_accent("Magenta").accent, Color::Magenta);
        assert_eq!(
            Theme::from_config_accent("LIGHTGREEN").accent,
            Color::LightGreen
        );
    }

    #[test]
    fn selected_uses_accent_and_reversed() {
        let t = Theme::from_config_accent("red");
        let style = t.selected();
        assert_eq!(style.fg, Some(Color::Red));
        assert!(style.add_modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn running_uses_accent_and_slow_blink() {
        let t = Theme::from_config_accent("yellow");
        let style = t.running();
        assert_eq!(style.fg, Some(Color::Yellow));
        assert!(style.add_modifier.contains(Modifier::SLOW_BLINK));
    }

    #[test]
    fn dim_does_not_carry_accent() {
        let t = Theme::from_config_accent("blue");
        assert_eq!(t.dim().fg, None);
        assert!(t.dim().add_modifier.contains(Modifier::DIM));
    }
}
