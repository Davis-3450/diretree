use ratatui::style::{Color, Modifier, Style};

/// LLM-TUI-inspired palette: dark base, violet/cyan accents, soft borders.
pub struct Theme {
    pub base: Color,
    pub surface: Color,
    pub accent: Color,
    pub accent2: Color,
    pub text: Color,
    pub muted: Color,
    pub border: Color,
    pub danger: Color,
    pub highlight_bg: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            base: Color::Rgb(10, 12, 18),
            surface: Color::Rgb(17, 20, 30),
            accent: Color::Rgb(139, 92, 246),  // violet
            accent2: Color::Rgb(45, 212, 191), // teal
            text: Color::Rgb(226, 228, 235),
            muted: Color::Rgb(110, 116, 134),
            border: Color::Rgb(46, 52, 70),
            danger: Color::Rgb(239, 68, 68),
            highlight_bg: Color::Rgb(44, 36, 74),
        }
    }

    pub fn base_style(&self) -> Style {
        Style::default().fg(self.text)
    }

    pub fn accent_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Muted secondary text (sizes, counts, hints).
    pub fn muted_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Selected list row: accent on a translucent highlight bg.
    pub fn highlight_style(&self) -> Style {
        Style::default()
            .fg(self.accent2)
            .bg(self.highlight_bg)
            .add_modifier(Modifier::BOLD)
    }
}

/// Deterministic hue for a directory tile from its name hash.
fn dir_hue(name: &str) -> usize {
    (fnv(name) % 8) as usize
}

const DIR_HUES: [Color; 8] = [
    Color::Rgb(67, 90, 170),
    Color::Rgb(80, 140, 200),
    Color::Rgb(45, 160, 150),
    Color::Rgb(60, 150, 90),
    Color::Rgb(170, 140, 60),
    Color::Rgb(190, 100, 90),
    Color::Rgb(140, 90, 180),
    Color::Rgb(110, 110, 120),
];

/// Tile color: dirs keep a name-hash hue but are shaded by relative size
/// (bigger = brighter). Files/symlinks use fixed dim colors.
pub fn tile_color(name: &str, kind: crate::scan::FileKind, size_pct: f64) -> Color {
    let base = match kind {
        crate::scan::FileKind::Dir => DIR_HUES[dir_hue(name)],
        crate::scan::FileKind::File => Color::Rgb(64, 70, 90),
        crate::scan::FileKind::Symlink => Color::Rgb(90, 64, 90),
    };
    shade(base, size_pct)
}

/// Brighten a color toward white by `t` (0..=1). Used for size shading.
fn shade(color: Color, t: f64) -> Color {
    let (r, g, b) = rgb(color);
    let t = t.clamp(0.0, 1.0);
    let f = |c: u8| (c as f64 + (255.0 - c as f64) * t) as u8;
    Color::Rgb(f(r), f(g), f(b))
}

/// Text color readable on `bg`: dark text on bright tiles, light on dark.
pub fn tile_label_fg(bg: Color) -> Color {
    let (r, g, b) = rgb(bg);
    // relative luminance (0..1)
    let lum = 0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64;
    if lum > 140.0 {
        Color::Rgb(18, 20, 26)
    } else {
        Color::Rgb(240, 242, 248)
    }
}

fn rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    }
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_contrast_switches() {
        // dark bg -> light text
        assert_eq!(
            tile_label_fg(Color::Rgb(10, 20, 30)),
            Color::Rgb(240, 242, 248)
        );
        // bright bg -> dark text
        assert_eq!(
            tile_label_fg(Color::Rgb(230, 220, 200)),
            Color::Rgb(18, 20, 26)
        );
    }

    #[test]
    fn shading_keeps_hue_direction() {
        let c = shade(Color::Rgb(67, 90, 170), 0.0);
        assert_eq!(c, Color::Rgb(67, 90, 170));
        let c2 = shade(Color::Rgb(67, 90, 170), 1.0);
        assert_eq!(c2, Color::Rgb(255, 255, 255));
    }
}
