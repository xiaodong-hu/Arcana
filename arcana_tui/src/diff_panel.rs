use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// Light gray for hints
const LIGHT_GRAY: Color = Color::Rgb(160, 160, 170);

/// A single line in a diff.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Header,
    Context,
    Added,
    Removed,
}

/// User action on the diff review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffAction {
    /// Accept the changes as-is
    Accept,
    /// Open in external editor for modification
    EditExternal,
    /// Reject the changes
    Reject,
}

/// Git diff review panel state.
#[derive(Debug, Clone)]
pub struct DiffPanel {
    /// Whether the panel is visible
    pub visible: bool,
    /// File path being changed
    pub file_path: String,
    /// Diff lines to display
    pub lines: Vec<DiffLine>,
    /// Scroll offset
    pub scroll: usize,
    /// User's final action
    pub action: Option<DiffAction>,
    /// Currently selected footer option (0=Accept, 1=Edit, 2=Reject)
    pub selected: usize,
}

impl Default for DiffPanel {
    fn default() -> Self {
        Self {
            visible: false,
            file_path: String::new(),
            lines: Vec::new(),
            scroll: 0,
            action: None,
            selected: 0,
        }
    }
}

impl DiffPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the diff panel with a unified diff string.
    pub fn show_diff(&mut self, file_path: String, diff_text: &str) {
        self.visible = true;
        self.file_path = file_path;
        self.scroll = 0;
        self.selected = 0;
        self.action = None;
        self.lines = diff_text.lines().map(|l| {
            let kind = if l.starts_with("+++") || l.starts_with("---") || l.starts_with("@@") {
                DiffKind::Header
            } else if l.starts_with('+') {
                DiffKind::Added
            } else if l.starts_with('-') {
                DiffKind::Removed
            } else {
                DiffKind::Context
            };
            DiffLine { kind, content: l.to_string() }
        }).collect();
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.lines.len().saturating_sub(5);
        self.scroll = (self.scroll + n).min(max);
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1).min(2);
    }

    pub fn confirm(&mut self) {
        self.action = Some(match self.selected {
            0 => DiffAction::Accept,
            1 => DiffAction::EditExternal,
            _ => DiffAction::Reject,
        });
        self.visible = false;
    }

    pub fn reject(&mut self) {
        self.action = Some(DiffAction::Reject);
        self.visible = false;
    }

    /// Render the diff review panel (full screen overlay).
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(" Diff Review: {} ", self.file_path))
            .title_alignment(Alignment::Left);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Reserve 2 lines for footer
        let diff_height = inner.height.saturating_sub(3) as usize;
        let footer_y = inner.y + inner.height.saturating_sub(2);

        // Render diff lines
        let visible_lines: Vec<Line> = self.lines.iter()
            .skip(self.scroll)
            .take(diff_height)
            .map(|dl| {
                let style = match dl.kind {
                    DiffKind::Header => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    DiffKind::Added => Style::default().fg(Color::Green),
                    DiffKind::Removed => Style::default().fg(Color::Red),
                    DiffKind::Context => Style::default().fg(Color::White),
                };
                Line::from(Span::styled(&dl.content, style))
            })
            .collect();

        let diff_area = Rect::new(inner.x, inner.y, inner.width, diff_height as u16);
        let paragraph = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, diff_area);

        // Scroll indicator
        let total = self.lines.len();
        let pct = if total > 0 { (self.scroll * 100) / total.max(1) } else { 0 };
        let scroll_info = format!(" {}/{} ({}%) ", self.scroll + 1, total, pct);

        // Footer with options
        let options = ["Accept", "Edit in $EDITOR", "Reject"];
        let mut footer_spans: Vec<Span> = Vec::new();
        for (i, opt) in options.iter().enumerate() {
            let (prefix, style) = if i == self.selected {
                ("❯ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                ("  ", Style::default().fg(LIGHT_GRAY))
            };
            footer_spans.push(Span::styled(prefix, style));
            footer_spans.push(Span::styled(*opt, style));
            if i < 2 {
                footer_spans.push(Span::styled("  │  ", Style::default().fg(LIGHT_GRAY)));
            }
        }
        footer_spans.push(Span::styled(scroll_info, Style::default().fg(LIGHT_GRAY)));

        let footer_area = Rect::new(inner.x, footer_y, inner.width, 1);
        frame.render_widget(Paragraph::new(Line::from(footer_spans)), footer_area);

        // Hint line
        let hint_area = Rect::new(inner.x, footer_y + 1, inner.width, 1);
        let hint = Line::from(Span::styled(
            "↑↓/j/k scroll │ ←→ select │ Enter confirm │ Esc reject │ Tab edit",
            Style::default().fg(LIGHT_GRAY),
        ));
        frame.render_widget(Paragraph::new(hint), hint_area);
    }
}
