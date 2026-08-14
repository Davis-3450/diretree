pub mod squarify;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect as TRect;
use ratatui::style::{Color, Modifier, Style};

use squarify::{Rect, squarify};

use crate::scan::Tree;
use crate::tui::theme::{Theme, tile_color, tile_label_fg};
use crate::units::human_bytes;

/// Render a treemap of `focus`'s children into the buffer. `selected` child
/// gets an accent border; `hovered` child is brightened. Tile color is the
/// dir's name-hue shaded by relative size; labels use auto-contrast text.
pub fn render_treemap(
    buf: &mut Buffer,
    area: TRect,
    tree: &Tree,
    focus: usize,
    selected: Option<usize>,
    hovered: Option<usize>,
) {
    let w = area.width as i64;
    let h = area.height as i64;
    if w <= 0 || h <= 0 {
        return;
    }
    let items: Vec<(usize, u64)> = tree
        .children(focus)
        .filter(|&c| tree.node(c).size > 0)
        .map(|c| (c, tree.node(c).size))
        .collect();
    let total: u64 = items.iter().map(|&(_, s)| s).sum::<u64>().max(1);
    let rect = Rect { x: 0, y: 0, w, h };
    let placed = squarify(rect, &items);

    for p in &placed {
        let (x, y, ww, hh) = (p.rect.x, p.rect.y, p.rect.w, p.rect.h);
        let n = tree.node(p.idx);
        let pct = n.size as f64 / total as f64;
        let mut color = tile_color(&n.name, n.kind, pct);
        if Some(p.idx) == hovered {
            color = Theme::dark().accent2;
        }
        for dy in 0..hh {
            for dx in 0..ww {
                let cell =
                    buf.cell_mut((area.x + x as u16 + dx as u16, area.y + y as u16 + dy as u16));
                if let Some(c) = cell {
                    c.set_char(' ');
                    c.set_bg(color);
                    c.set_fg(tile_label_fg(color));
                }
            }
        }
    }

    // labels on tiles big enough
    for p in &placed {
        if p.rect.w >= 5 && p.rect.h >= 1 {
            let n = tree.node(p.idx);
            let pct = n.size as f64 / total as f64;
            let mut label = format!("{}  {}  {:.1}%", n.name, human_bytes(n.size), pct * 100.0);
            let max_chars = p.rect.w as usize - 1;
            if label.chars().count() > max_chars {
                let cut: String = label.chars().take(max_chars.saturating_sub(1)).collect();
                label = format!("{cut}…");
            }
            let color = tile_color(&n.name, n.kind, pct);
            let bg = if Some(p.idx) == hovered {
                Theme::dark().accent2
            } else {
                color
            };
            let fg = tile_label_fg(bg);
            let cx = p.rect.x + 1;
            let cy = p.rect.y;
            for (k, ch) in label.chars().enumerate() {
                let cell = buf.cell_mut((area.x + cx as u16 + k as u16, area.y + cy as u16));
                if let Some(c) = cell {
                    c.set_char(ch);
                    c.set_bg(bg);
                    c.set_fg(fg);
                }
            }
        }
    }

    // selected tile border
    if let Some(p) = selected.and_then(|sel| placed.iter().find(|p| p.idx == sel)) {
        draw_border(buf, area, p.rect, Theme::dark().accent);
    }
    // hovered tile border (teal), only when not already selected
    if let Some(hv) = hovered.filter(|hv| Some(*hv) != selected)
        && let Some(p) = placed.iter().find(|q| q.idx == hv)
    {
        draw_border(buf, area, p.rect, Theme::dark().accent2);
    }
}

/// Map a terminal cell (0-based, relative to treemap area origin) to the
/// focused child whose tile contains it. Returns None if empty or outside.
pub fn hit_test(tree: &Tree, focus: usize, area: TRect, col: i64, row: i64) -> Option<usize> {
    let w = area.width as i64;
    let h = area.height as i64;
    if col < 0 || row < 0 || col >= w || row >= h {
        return None;
    }
    let items: Vec<(usize, u64)> = tree
        .children(focus)
        .filter(|&c| tree.node(c).size > 0)
        .map(|c| (c, tree.node(c).size))
        .collect();
    let placed = squarify(Rect { x: 0, y: 0, w, h }, &items);
    placed
        .into_iter()
        .find(|p| p.rect.contains(col, row))
        .map(|p| p.idx)
}

fn draw_border(buf: &mut Buffer, area: TRect, r: Rect, color: Color) {
    let (x0, y0) = (area.x + r.x as u16, area.y + r.y as u16);
    let (x1, y1) = (x0 + r.w as u16 - 1, y0 + r.h as u16 - 1);
    let style = Style::default()
        .bg(color)
        .add_modifier(Modifier::REVERSED | Modifier::BOLD);
    for x in x0..=x1 {
        if let Some(c) = buf.cell_mut((x, y0)) {
            c.set_style(style);
        }
        if let Some(c) = buf.cell_mut((x, y1)) {
            c.set_style(style);
        }
    }
    for y in y0..=y1 {
        if let Some(c) = buf.cell_mut((x0, y)) {
            c.set_style(style);
        }
        if let Some(c) = buf.cell_mut((x1, y)) {
            c.set_style(style);
        }
    }
}
