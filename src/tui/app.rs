use std::collections::HashSet;
use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

use crossterm::ExecutableCommand;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseButton, MouseEventKind,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::cli::report::top_entries;
use crate::scan::{FileKind, Tree};
use crate::tui::theme::Theme;
use crate::units::human_bytes;

use super::treemap;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Treemap,
    Tree,
    Top,
}

struct App {
    tree: Tree,
    tab: Tab,
    // treemap
    focus: usize,
    selected: Option<usize>,
    hovered: Option<usize>,
    // tree view
    expanded: HashSet<usize>,
    tree_cursor: usize,
    // top view
    top_cursor: usize,
    // last-frame layout areas (for mouse hit-testing)
    treemap_inner: Rect,
    tree_inner: Rect,
    top_inner: Rect,
    footer_area: Rect,
    back_area: Rect,
    // list state (for scroll-offset-aware mouse hit-testing)
    tree_state: ListState,
    top_state: ListState,
    // double-click tracking
    last_click: Option<(u16, u16, Instant)>,
}

impl App {
    fn new(tree: Tree) -> Self {
        let mut expanded = HashSet::new();
        expanded.insert(tree.root());
        Self {
            tree,
            tab: Tab::Treemap,
            focus: 0,
            selected: None,
            hovered: None,
            expanded,
            tree_cursor: 0,
            top_cursor: 0,
            treemap_inner: Rect::default(),
            tree_inner: Rect::default(),
            top_inner: Rect::default(),
            footer_area: Rect::default(),
            back_area: Rect::default(),
            tree_state: ListState::default(),
            top_state: ListState::default(),
            last_click: None,
        }
    }

    fn run(
        &mut self,
        term: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    ) -> io::Result<bool> {
        loop {
            term.draw(|f| self.draw(f))?;
            if self.handle_events()? {
                return Ok(true);
            }
        }
    }

    fn draw(&mut self, f: &mut ratatui::Frame) {
        let theme = Theme::dark();
        let area = f.area();
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        self.draw_header(f, rows[0], &theme);
        self.draw_footer(f, rows[2], &theme);
        self.footer_area = rows[2];
        match self.tab {
            Tab::Treemap => self.draw_treemap(f, rows[1], &theme),
            Tab::Tree => self.draw_tree(f, rows[1], &theme),
            Tab::Top => self.draw_top(f, rows[1], &theme),
        }
    }

    fn draw_header(&mut self, f: &mut ratatui::Frame, area: Rect, theme: &Theme) {
        let back_text = " ◀ back ";
        let back_w = back_text.chars().count() as u16;
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(back_w), Constraint::Min(0)])
            .split(area);
        self.back_area = cols[0];
        let btn_style = if self.can_back() {
            Style::default()
                .fg(theme.text)
                .bg(theme.accent)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted).bg(theme.surface)
        };
        f.render_widget(Paragraph::new(back_text).style(btn_style), cols[0]);

        let root = self.tree.node(self.tree.root());
        let mut left = format!(" diretree · {}", root.path.display());
        if let Some(n) = self.current_node() {
            left.push_str(&format!(" › {}", self.tree.node(n).path.display()));
        }
        let right = format!(
            " {}  {} files  {} dirs ",
            human_bytes(root.size),
            root.file_count,
            root.dir_count
        );
        let width = cols[1].width as usize;
        let right_c = right.chars().count();
        let max_left = width.saturating_sub(right_c).saturating_sub(1);
        let left_c: String = left.chars().take(max_left).collect();
        let pad = " ".repeat(width.saturating_sub(left_c.chars().count() + right_c));

        let mut spans = vec![Span::styled(left_c, theme.base_style())];
        spans.push(Span::styled(pad + &right, theme.muted_style()));
        f.render_widget(Paragraph::new(Line::from(spans)), cols[1]);
    }

    fn can_back(&self) -> bool {
        match self.tab {
            Tab::Treemap => self.tree.node(self.focus).parent.is_some(),
            Tab::Tree => {
                let v = self.visible_nodes();
                v.get(self.tree_cursor)
                    .map(|&i| self.expanded.contains(&i) || self.tree.node(i).parent.is_some())
                    .unwrap_or(false)
            }
            Tab::Top => false,
        }
    }

    fn footer_text(&self) -> String {
        let help = match self.tab {
            Tab::Treemap => "  ⏎/dbl drill  u up  ←→ select  o open  q quit ",
            Tab::Tree => "  j/k move  l enter  h collapse  o open  q quit ",
            Tab::Top => "  j/k move  ⏎ open  o open  q quit ",
        };
        format!(" [1] treemap  [2] tree  [3] top   {help}")
    }

    fn draw_footer(&self, f: &mut ratatui::Frame, area: Rect, theme: &Theme) {
        let text = self.footer_text();
        let mut spans = Vec::new();
        for ch in text.chars() {
            let style = match ch {
                '1' if self.tab == Tab::Treemap => theme.accent_style(),
                '2' if self.tab == Tab::Tree => theme.accent_style(),
                '3' if self.tab == Tab::Top => theme.accent_style(),
                '1' | '2' | '3' => theme.base_style(),
                _ => theme.muted_style(),
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn draw_treemap(&mut self, f: &mut ratatui::Frame, area: Rect, theme: &Theme) {
        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));
        let inner = block.inner(body[0]);
        f.render_widget(block, body[0]);

        // tiles below the caption row
        let tiles = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);
        let focus_caption = format!(" {}", self.tree.node(self.focus).path.display());
        f.render_widget(
            Paragraph::new(focus_caption).style(theme.accent_style()),
            tiles[0],
        );
        let treemap_area = tiles[1];
        self.treemap_inner = treemap_area;

        let buf = f.buffer_mut();
        treemap::render_treemap(
            buf,
            treemap_area,
            &self.tree,
            self.focus,
            self.selected,
            self.hovered,
        );

        // status line: node info
        let status_area = body[1];
        let text = match self.current_node() {
            Some(n) => {
                let node = self.tree.node(n);
                let focus_size = self.tree.node(self.focus).size.max(1);
                let pct = node.size as f64 / focus_size as f64 * 100.0;
                let sel = if Some(n) == self.selected {
                    "  ●"
                } else {
                    ""
                };
                format!(
                    " {}  {}  {:.1}% of {}  {} files  {} dirs{}",
                    node.path.display(),
                    human_bytes(node.size),
                    pct,
                    human_bytes(focus_size),
                    node.file_count,
                    node.dir_count,
                    sel
                )
            }
            None => " hover: tiles    click: select    double-click: drill ".to_string(),
        };
        f.render_widget(
            Paragraph::new(text).style(Style::default().fg(theme.text).bg(theme.surface)),
            status_area,
        );
    }

    fn draw_tree(&mut self, f: &mut ratatui::Frame, area: Rect, theme: &Theme) {
        let root_size = self.tree.node(self.tree.root()).size.max(1);
        let visible = self.visible_nodes();
        let items: Vec<ListItem> = visible
            .iter()
            .map(|&idx| {
                let n = self.tree.node(idx);
                let depth = self.depth_of(idx);
                let pct = n.size as f64 / root_size as f64 * 100.0;
                let bar_len = (n.size as f64 / root_size as f64 * 14.0).round() as usize;

                let glyph = match n.kind {
                    FileKind::Dir => {
                        if self.expanded.contains(&idx) {
                            "▼"
                        } else {
                            "▶"
                        }
                    }
                    FileKind::File => "•",
                    FileKind::Symlink => "→",
                };
                let (glyph_color, name_color) = match n.kind {
                    FileKind::Dir => (theme.accent2, theme.text),
                    FileKind::File => (theme.muted, theme.text),
                    FileKind::Symlink => (theme.danger, theme.text),
                };

                let mut spans = vec![Span::raw("  ".repeat(depth))];
                spans.push(Span::styled(glyph, Style::default().fg(glyph_color)));
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    n.name.clone(),
                    Style::default().fg(name_color),
                ));
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    "█".repeat(bar_len),
                    Style::default().fg(if bar_len > 0 {
                        theme.accent2
                    } else {
                        theme.border
                    }),
                ));
                spans.push(Span::styled(
                    format!(" {:>9}  {:>5.1}%", human_bytes(n.size), pct),
                    theme.muted_style(),
                ));
                ListItem::new(Line::from(spans))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" files ");
        let list = List::new(items)
            .block(block)
            .highlight_style(theme.highlight_style())
            .highlight_symbol("▍ ");
        let inner = area;
        self.tree_state.select(Some(self.tree_cursor));
        f.render_stateful_widget(list, area, &mut self.tree_state);
        self.tree_inner = Rect {
            x: inner.x + 1,
            y: inner.y + 1,
            width: inner.width.saturating_sub(2),
            height: inner.height.saturating_sub(2),
        };
    }

    fn draw_top(&mut self, f: &mut ratatui::Frame, area: Rect, theme: &Theme) {
        let root_size = self.tree.node(self.tree.root()).size.max(1);
        let entries = top_entries(&self.tree, 100, 0);
        let items: Vec<ListItem> = entries
            .iter()
            .enumerate()
            .map(|(row, &idx)| {
                let n = self.tree.node(idx);
                let pct = n.size as f64 / root_size as f64;
                let bar_len = (pct * 20.0).round() as usize;
                let (color, label) = match n.kind {
                    FileKind::Dir => (theme.accent, "DIR"),
                    FileKind::File => (theme.accent2, "FILE"),
                    FileKind::Symlink => (theme.danger, "LINK"),
                };
                let mut spans = vec![Span::styled(
                    format!("{:>4} ", row + 1),
                    theme.muted_style(),
                )];
                spans.push(Span::styled(
                    format!("{label:<4} "),
                    Style::default().fg(color),
                ));
                spans.push(Span::styled(
                    "█".repeat(bar_len),
                    Style::default().fg(color),
                ));
                spans.push(Span::styled(
                    format!(" {:>9}  {:>5.1}%  ", human_bytes(n.size), pct * 100.0),
                    theme.muted_style(),
                ));
                spans.push(Span::styled(
                    n.path.display().to_string(),
                    theme.base_style(),
                ));
                ListItem::new(Line::from(spans))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(" top files & dirs ");
        let list = List::new(items)
            .block(block)
            .highlight_style(theme.highlight_style())
            .highlight_symbol("▍ ");
        self.top_state.select(Some(self.top_cursor));
        f.render_stateful_widget(list, area, &mut self.top_state);
        self.top_inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };
    }

    fn current_node(&self) -> Option<usize> {
        match self.tab {
            Tab::Treemap => self.selected.or(self.hovered).or(Some(self.focus)),
            Tab::Tree => {
                let v = self.visible_nodes();
                v.get(self.tree_cursor).copied()
            }
            Tab::Top => {
                let e = top_entries(&self.tree, 100, 0);
                e.get(self.top_cursor).copied()
            }
        }
    }

    fn visible_nodes(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack: Vec<usize> = vec![self.tree.root()];
        while let Some(i) = stack.pop() {
            out.push(i);
            if self.expanded.contains(&i) {
                let mut kids: Vec<_> = self.tree.children(i).collect();
                kids.reverse();
                stack.extend(kids);
            }
        }
        out
    }

    fn depth_of(&self, idx: usize) -> usize {
        let mut d = 0;
        let mut cur = self.tree.node(idx).parent;
        while let Some(p) = cur {
            d += 1;
            cur = self.tree.node(p).parent;
        }
        d
    }

    fn handle_events(&mut self) -> io::Result<bool> {
        if !event::poll(Duration::from_millis(80))? {
            return Ok(false);
        }
        match event::read()? {
            Event::Key(k) => self.on_key(k),
            Event::Mouse(m) => {
                self.on_mouse(m);
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn on_mouse(&mut self, m: crossterm::event::MouseEvent) {
        // crossterm coords are 1-based; ratatui is 0-based
        let (col, row) = (m.column.saturating_sub(1), m.row.saturating_sub(1));
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // back button
                if self.can_back() && self.back_area.contains(Position::new(col, row)) {
                    self.go_up();
                    return;
                }
                // footer tab clicks
                if self.footer_area.contains(Position::new(col, row)) {
                    let i = (col - self.footer_area.x) as usize;
                    match self.footer_text().chars().nth(i) {
                        Some('1') => {
                            self.tab = Tab::Treemap;
                            self.hovered = None;
                        }
                        Some('2') => {
                            self.tab = Tab::Tree;
                            self.hovered = None;
                        }
                        Some('3') => {
                            self.tab = Tab::Top;
                            self.hovered = None;
                        }
                        _ => {}
                    }
                    return;
                }
                match self.tab {
                    Tab::Treemap => {
                        if self.treemap_inner.contains(Position::new(col, row)) {
                            let lx = col as i64 - self.treemap_inner.x as i64;
                            let ly = row as i64 - self.treemap_inner.y as i64;
                            if let Some(idx) = treemap::hit_test(
                                &self.tree,
                                self.focus,
                                self.treemap_inner,
                                lx,
                                ly,
                            ) {
                                // double-click on same spot -> drill
                                let now = Instant::now();
                                let is_double = self
                                    .last_click
                                    .map(|(c, r, t)| {
                                        c == col
                                            && r == row
                                            && now.duration_since(t) < Duration::from_millis(400)
                                    })
                                    .unwrap_or(false);
                                self.last_click = Some((col, row, now));
                                self.selected = Some(idx);
                                if is_double {
                                    let n = self.tree.node(idx);
                                    if n.kind == FileKind::Dir && self.tree.has_children(idx) {
                                        self.focus = idx;
                                        self.selected = None;
                                        self.hovered = None;
                                    }
                                }
                            }
                        }
                    }
                    Tab::Tree => {
                        if self.tree_inner.contains(Position::new(col, row)) {
                            let idx = (row - self.tree_inner.y) as usize + self.tree_state.offset();
                            let v = self.visible_nodes();
                            if idx < v.len() {
                                self.tree_cursor = idx;
                            }
                        }
                    }
                    Tab::Top => {
                        if self.top_inner.contains(Position::new(col, row)) {
                            let idx = (row - self.top_inner.y) as usize + self.top_state.offset();
                            let e = top_entries(&self.tree, 100, 0);
                            if idx < e.len() {
                                self.top_cursor = idx;
                            }
                        }
                    }
                }
            }
            MouseEventKind::Moved => {
                if self.tab == Tab::Treemap && self.treemap_inner.contains(Position::new(col, row))
                {
                    let lx = col as i64 - self.treemap_inner.x as i64;
                    let ly = row as i64 - self.treemap_inner.y as i64;
                    self.hovered =
                        treemap::hit_test(&self.tree, self.focus, self.treemap_inner, lx, ly);
                } else {
                    self.hovered = None;
                }
            }
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                let dir = if m.kind == MouseEventKind::ScrollDown {
                    1
                } else {
                    -1
                };
                match self.tab {
                    Tab::Tree => {
                        let v = self.visible_nodes();
                        let next = (self.tree_cursor as i64 + dir)
                            .clamp(0, v.len().saturating_sub(1) as i64)
                            as usize;
                        self.tree_cursor = next;
                    }
                    Tab::Top => {
                        let e = top_entries(&self.tree, 100, 0);
                        let next = (self.top_cursor as i64 + dir)
                            .clamp(0, e.len().saturating_sub(1) as i64)
                            as usize;
                        self.top_cursor = next;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn on_key(&mut self, k: KeyEvent) -> io::Result<bool> {
        if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c')) {
            return Ok(true);
        }
        match k.code {
            KeyCode::Char('q') => Ok(true),
            KeyCode::Esc => {
                self.go_up();
                Ok(false)
            }
            KeyCode::Char('1') => self.switch(Tab::Treemap),
            KeyCode::Char('2') => self.switch(Tab::Tree),
            KeyCode::Char('3') => self.switch(Tab::Top),
            KeyCode::Char('o') => self.open_selected(),
            KeyCode::Char('u') | KeyCode::Backspace => {
                self.go_up();
                Ok(false)
            }
            other => match self.tab {
                Tab::Treemap => self.treemap_key(other),
                Tab::Tree => self.tree_key(other),
                Tab::Top => self.top_key(other),
            },
        }
    }

    fn go_up(&mut self) {
        match self.tab {
            Tab::Treemap => {
                if let Some(p) = self.tree.node(self.focus).parent {
                    self.focus = p;
                    self.selected = None;
                    self.hovered = None;
                }
            }
            Tab::Tree => {
                let v = self.visible_nodes();
                if let Some(&idx) = v.get(self.tree_cursor) {
                    if self.expanded.contains(&idx) {
                        self.expanded.remove(&idx);
                    } else if let Some(p) = self.tree.node(idx).parent
                        && let Some(pos) = v.iter().position(|&x| x == p)
                    {
                        self.tree_cursor = pos;
                    }
                }
            }
            Tab::Top => {}
        }
    }

    fn switch(&mut self, tab: Tab) -> io::Result<bool> {
        self.tab = tab;
        self.hovered = None;
        Ok(false)
    }

    fn treemap_key(&mut self, k: KeyCode) -> io::Result<bool> {
        let kids: Vec<usize> = self
            .tree
            .children(self.focus)
            .filter(|&c| self.tree.node(c).size > 0)
            .collect();
        let cur_pos = self
            .selected
            .and_then(|s| kids.iter().position(|&c| c == s));
        match k {
            KeyCode::Enter => {
                if let Some(s) = self.selected {
                    // drill into a dir
                    if self.tree.node(s).kind == FileKind::Dir && self.tree.has_children(s) {
                        self.focus = s;
                        self.selected = None;
                    }
                } else {
                    // already drilled in: collapse back to parent
                    self.go_up();
                }
            }
            KeyCode::Right => {
                let pos = cur_pos.unwrap_or(0);
                if !kids.is_empty() {
                    self.selected = Some(kids[(pos + 1) % kids.len()]);
                }
            }
            KeyCode::Left => {
                let pos = cur_pos.unwrap_or(0);
                if !kids.is_empty() {
                    self.selected = Some(kids[(pos + kids.len() - 1) % kids.len()]);
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn tree_key(&mut self, k: KeyCode) -> io::Result<bool> {
        let visible = self.visible_nodes();
        if visible.is_empty() {
            return Ok(false);
        }
        match k {
            KeyCode::Down | KeyCode::Char('j') if self.tree_cursor + 1 < visible.len() => {
                self.tree_cursor += 1;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.tree_cursor = self.tree_cursor.saturating_sub(1);
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                let idx = visible[self.tree_cursor];
                if self.tree.node(idx).kind == FileKind::Dir {
                    self.expanded.insert(idx);
                }
            }
            KeyCode::Char('h') => {
                let idx = visible[self.tree_cursor];
                if self.expanded.contains(&idx) {
                    self.expanded.remove(&idx);
                } else if let Some(p) = self.tree.node(idx).parent
                    && let Some(pos) = visible.iter().position(|&v| v == p)
                {
                    self.tree_cursor = pos;
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn top_key(&mut self, k: KeyCode) -> io::Result<bool> {
        let entries = top_entries(&self.tree, 100, 0);
        match k {
            KeyCode::Down | KeyCode::Char('j') if self.top_cursor + 1 < entries.len() => {
                self.top_cursor += 1;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.top_cursor = self.top_cursor.saturating_sub(1);
            }
            KeyCode::Enter if self.top_cursor < entries.len() => {
                let _ = open::that(self.tree.node(entries[self.top_cursor]).path.clone());
            }
            _ => {}
        }
        Ok(false)
    }

    fn open_selected(&self) -> io::Result<bool> {
        let path = self.current_node().map(|i| self.tree.node(i).path.clone());
        if let Some(p) = path {
            let _ = open::that(p);
        }
        Ok(false)
    }
}

pub fn run(tree: Tree) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(tree);
    let result = app.run(&mut terminal);
    terminal.show_cursor()?;
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(DisableMouseCapture)?;
    stdout.execute(LeaveAlternateScreen)?;
    result?;
    Ok(())
}
