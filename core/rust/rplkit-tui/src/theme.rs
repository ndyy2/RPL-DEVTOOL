//! theme — token visual tunggal (Fase 0 kontrak §8).
//!
//! Monokrom + satu aksen Blue Iris `#5B5BD6`. Aksen HANYA untuk
//! selection/focus/status. Dilarang menambah warna lain.

use ratatui::style::{Color, Modifier, Style};

/// Blue Iris — satu-satunya aksen.
pub const ACCENT: Color = Color::Rgb(0x5B, 0x5B, 0xD6);
pub const BG: Color = Color::Rgb(0x0B, 0x0F, 0x16);
pub const PANEL: Color = Color::Rgb(0x11, 0x16, 0x1F);
pub const BORDER: Color = Color::Rgb(0x2A, 0x33, 0x42);
pub const TEXT: Color = Color::Rgb(0xC9, 0xD1, 0xD9);
pub const DIM: Color = Color::Rgb(0x6E, 0x76, 0x81);

pub fn normal() -> Style {
    Style::default().fg(TEXT).bg(BG)
}

pub fn dim() -> Style {
    Style::default().fg(DIM).bg(BG)
}

/// Baris terpilih: aksen sebagai latar + teks terang + bold.
pub fn selected() -> Style {
    Style::default().fg(Color::White).bg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Fokus/status: aksen sebagai foreground.
pub fn accent() -> Style {
    Style::default().fg(ACCENT).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn accent_on_panel() -> Style {
    Style::default().fg(ACCENT).bg(PANEL).add_modifier(Modifier::BOLD)
}

pub fn border(focused: bool) -> Style {
    if focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(BORDER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_accent_rule() {
        // Kontrak: hanya ACCENT yang boleh dipakai sebagai warna sorot.
        // Test ini mengunci nilai token agar perubahan disengaja.
        assert_eq!(ACCENT, Color::Rgb(0x5B, 0x5B, 0xD6));
        assert_ne!(BG, ACCENT);
        assert_ne!(TEXT, ACCENT);
    }
}
