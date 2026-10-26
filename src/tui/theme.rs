use clap::ValueEnum;
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum ThemeChoice {
    #[default]
    Default,
    Catppuccin,
    Monokai,
    Nord,
    TokyoNight,
}

impl ThemeChoice {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Default => "diretree",
            Self::Catppuccin => "catppuccin",
            Self::Monokai => "monokai",
            Self::Nord => "nord",
            Self::TokyoNight => "tokyo-night",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Default => Self::Catppuccin,
            Self::Catppuccin => Self::Monokai,
            Self::Monokai => Self::Nord,
            Self::Nord => Self::TokyoNight,
            Self::TokyoNight => Self::Default,
        }
    }

    pub fn to_theme(&self) -> Theme {
        match self {
            Self::Default => Theme::dark(),
            Self::Catppuccin => Theme::catppuccin(),
            Self::Monokai => Theme::monokai(),
            Self::Nord => Theme::nord(),
            Self::TokyoNight => Theme::tokyo_night(),
        }
    }
}

/// Palette structure for Diretree UI
#[derive(Clone, Debug)]
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
    pub dir_hues: [Color; 8],
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
            dir_hues: [
                Color::Rgb(67, 90, 170),
                Color::Rgb(80, 140, 200),
                Color::Rgb(45, 160, 150),
                Color::Rgb(60, 150, 90),
                Color::Rgb(170, 140, 60),
                Color::Rgb(190, 100, 90),
                Color::Rgb(140, 90, 180),
                Color::Rgb(110, 110, 120),
            ],
        }
    }

    pub fn catppuccin() -> Self {
        Self {
            base: Color::Rgb(30, 30, 46),       // Mocha base
            surface: Color::Rgb(24, 24, 37),    // Mantle
            accent: Color::Rgb(203, 166, 247),  // Mauve
            accent2: Color::Rgb(148, 226, 213), // Teal
            text: Color::Rgb(205, 214, 244),
            muted: Color::Rgb(147, 153, 178),   // Overlay0
            border: Color::Rgb(69, 71, 90),     // Surface1
            danger: Color::Rgb(243, 139, 168),  // Red
            highlight_bg: Color::Rgb(88, 91, 112), // Surface2
            dir_hues: [
                Color::Rgb(137, 180, 250), // Blue
                Color::Rgb(180, 190, 254), // Lavender
                Color::Rgb(148, 226, 213), // Teal
                Color::Rgb(166, 227, 161), // Green
                Color::Rgb(249, 226, 175), // Yellow
                Color::Rgb(250, 179, 135), // Peach
                Color::Rgb(245, 194, 231), // Pink
                Color::Rgb(203, 166, 247), // Mauve
            ],
        }
    }

    pub fn monokai() -> Self {
        Self {
            base: Color::Rgb(39, 40, 34),
            surface: Color::Rgb(30, 31, 26),
            accent: Color::Rgb(249, 38, 114),  // Pink
            accent2: Color::Rgb(102, 217, 239), // Cyan
            text: Color::Rgb(248, 248, 242),
            muted: Color::Rgb(117, 113, 94),
            border: Color::Rgb(73, 72, 62),
            danger: Color::Rgb(249, 38, 114),
            highlight_bg: Color::Rgb(62, 61, 50),
            dir_hues: [
                Color::Rgb(102, 217, 239), // Cyan
                Color::Rgb(166, 226, 46),  // Green
                Color::Rgb(253, 151, 31),  // Orange
                Color::Rgb(249, 38, 114),  // Magenta
                Color::Rgb(174, 129, 255), // Purple
                Color::Rgb(230, 219, 116), // Yellow
                Color::Rgb(102, 170, 239),
                Color::Rgb(210, 100, 120),
            ],
        }
    }

    pub fn nord() -> Self {
        Self {
            base: Color::Rgb(46, 52, 64),       // polar night 0
            surface: Color::Rgb(59, 66, 82),    // polar night 1
            accent: Color::Rgb(136, 192, 208),  // frost 8
            accent2: Color::Rgb(129, 161, 193), // frost 9
            text: Color::Rgb(236, 239, 244),    // snow storm 6
            muted: Color::Rgb(140, 153, 179),
            border: Color::Rgb(76, 86, 106),
            danger: Color::Rgb(191, 97, 106),   // aurora red
            highlight_bg: Color::Rgb(67, 76, 94),
            dir_hues: [
                Color::Rgb(143, 188, 187), // frost 7
                Color::Rgb(136, 192, 208), // frost 8
                Color::Rgb(129, 161, 193), // frost 9
                Color::Rgb(94, 129, 172),  // frost 10
                Color::Rgb(163, 190, 140), // aurora green
                Color::Rgb(235, 203, 139), // aurora yellow
                Color::Rgb(208, 135, 112), // aurora orange
                Color::Rgb(180, 142, 173), // aurora purple
            ],
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            base: Color::Rgb(26, 27, 38),
            surface: Color::Rgb(22, 22, 30),
            accent: Color::Rgb(122, 162, 247),  // Blue
            accent2: Color::Rgb(125, 207, 255), // Cyan
            text: Color::Rgb(192, 202, 245),
            muted: Color::Rgb(86, 95, 137),
            border: Color::Rgb(41, 46, 66),
            danger: Color::Rgb(247, 118, 142),  // Red
            highlight_bg: Color::Rgb(40, 52, 79),
            dir_hues: [
                Color::Rgb(122, 162, 247), // Blue
                Color::Rgb(125, 207, 255), // Cyan
                Color::Rgb(187, 154, 247), // Magenta/Purple
                Color::Rgb(158, 206, 106), // Green
                Color::Rgb(224, 175, 104), // Yellow
                Color::Rgb(255, 158, 100), // Orange
                Color::Rgb(247, 118, 142), // Red
                Color::Rgb(115, 218, 202), // Teal
            ],
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

/// Tile color: dirs keep a name-hash hue but are shaded by relative size
/// (bigger = brighter). Files/symlinks use fixed dim colors.
pub fn tile_color(name: &str, kind: crate::scan::FileKind, size_pct: f64) -> Color {
    tile_color_with_theme(name, kind, size_pct, &Theme::dark())
}

pub fn tile_color_with_theme(name: &str, kind: crate::scan::FileKind, size_pct: f64, theme: &Theme) -> Color {
    let base = match kind {
        crate::scan::FileKind::Dir => theme.dir_hues[dir_hue(name)],
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
