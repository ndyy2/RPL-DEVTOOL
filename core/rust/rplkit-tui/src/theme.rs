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

/// Baris terpilih: bar aksen tipis di kiri + teks bold, latar tetap
/// gelap. Jauh lebih tenang daripada blok aksen penuh; selection tetap
/// terbaca karena satu-satunya elemen bold-terang di daftar.
pub fn selected() -> Style {
    Style::default().fg(Color::White).bg(BG).add_modifier(Modifier::BOLD)
}

/// Bar 2 kolom untuk sisi kiri baris terpilih ("▌").
pub fn select_bar() -> Style {
    Style::default().fg(ACCENT).bg(BG).add_modifier(Modifier::BOLD)
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

/// Meter presisi 1/8 blok: `▁▂▃▄▅▆▇█` + track redup.
/// Ganti pola kasar `█/░`. pct dijepit 0–100.
pub fn meter(pct: f32, width: usize) -> (String, String) {
    const PARTS: &[char] = &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇'];
    let pct = pct.clamp(0.0, 100.0);
    let total_sub = (pct / 100.0 * width as f32 * 8.0).round() as usize;
    let full = total_sub / 8;
    let mut fill = "█".repeat(full.min(width));
    let mut track = String::new();
    if full < width {
        let rest = (total_sub % 8) as usize;
        if rest > 0 {
            fill.push(PARTS[rest]);
        }
        track = "░".repeat(width - full - usize::from(rest > 0));
    }
    (fill, track)
}

/// Garis section proporsional: judul + ── secukupnya (tak penuh lebar).
/// `budget` = lebar area; garis dibatasi 24 kolom agar bernapas.
pub fn rule(title: &str, budget: usize) -> String {
    let dashes = (budget.saturating_sub(title.len() + 4)).min(24).max(4);
    format!("{title}  {}", "─".repeat(dashes))
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

    #[test]
    fn meter_vectors() {
        assert_eq!(meter(0.0, 4), ("".into(), "░░░░".into()));
        assert_eq!(meter(100.0, 4), ("████".into(), "".into()));
        let (fill, track) = meter(50.0, 4);
        assert_eq!(fill.chars().count() + track.chars().count(), 4);
        assert!(fill.starts_with("██"));
        // Clamp di luar rentang.
        assert_eq!(meter(140.0, 2), ("██".into(), "".into()));
        assert_eq!(meter(-5.0, 2), ("".into(), "░░".into()));
    }

    #[test]
    fn meter_partial_block() {
        // 12.5% dari 1 kolom = tepat 1 sub-blok penuh (▁).
        let (fill, track) = meter(12.5, 1);
        assert_eq!((fill.as_str(), track.as_str()), ("▁", ""));
    }

    #[test]
    fn rule_is_bounded() {
        let r = rule("SYSTEM", 80);
        assert!(r.starts_with("SYSTEM  "));
        assert!(r.chars().count() <= "SYSTEM".len() + 2 + 24);
        let short = rule("A VERY LONG SECTION TITLE THAT EXCEEDS", 20);
        assert!(short.contains("────"));
    }
}
