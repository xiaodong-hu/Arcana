use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

/// Light gray for hints
const LIGHT_GRAY: Color = Color::Rgb(160, 160, 170);

/// The four approval options for authority requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalChoice {
    /// Approve this single file mutation only
    SingleApprove,
    /// Approve all steps in this task (dangerous)
    ApproveAll,
    /// Pause and wait for human to finish editing/reviewing
    HumanInterrupt,
    /// Reject and abort the current operation
    Reject,
}

/// Authority approval panel state.
#[derive(Debug, Clone)]
pub struct ApprovalPanel {
    /// Whether the panel is visible
    pub visible: bool,
    /// Description of what requires approval
    pub description: String,
    /// File path being mutated (if applicable)
    pub file_path: Option<String>,
    /// Currently selected option (0-3)
    pub selected: usize,
    /// The user's final choice (set when they confirm)
    pub choice: Option<ApprovalChoice>,
}

impl Default for ApprovalPanel {
    fn default() -> Self {
        Self {
            visible: false,
            description: String::new(),
            file_path: None,
            selected: 0,
            choice: None,
        }
    }
}

impl ApprovalPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the approval panel for a mutation request.
    pub fn request(&mut self, description: String, file_path: Option<String>) {
        self.visible = true;
        self.description = description;
        self.file_path = file_path;
        self.selected = 0;
        self.choice = None;
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1).min(3);
    }

    /// Confirm the current selection.
    pub fn confirm(&mut self) {
        self.choice = Some(match self.selected {
            0 => ApprovalChoice::SingleApprove,
            1 => ApprovalChoice::ApproveAll,
            2 => ApprovalChoice::HumanInterrupt,
            _ => ApprovalChoice::Reject,
        });
        self.visible = false;
    }

    /// Reject (shortcut via Esc).
    pub fn reject(&mut self) {
        self.choice = Some(ApprovalChoice::Reject);
        self.visible = false;
    }

    /// Render the approval panel.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Panel dimensions
        let width = 60u16.min(area.width.saturating_sub(4));
        let height = 9u16;
        let x = (area.width.saturating_sub(width)) / 2;
        let y = area.height.saturating_sub(height + 2);

        let panel_area = Rect::new(area.x + x, area.y + y, width, height);
        frame.render_widget(Clear, panel_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Authority ")
            .title_alignment(Alignment::Left);

        let inner = block.inner(panel_area);
        frame.render_widget(block, panel_area);

        let mut lines: Vec<Line> = Vec::new();

        // Description
        let desc = if let Some(ref path) = self.file_path {
            format!("write requires approval: {}", path)
        } else {
            self.description.clone()
        };
        lines.push(Line::from(Span::styled(
            desc,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Options
        let options = [
            ("Yes, single permission", Color::Green),
            ("Trust, always allow in this session", Color::Yellow),
            ("Human interrupt (wait for edit)", Color::Cyan),
            ("Reject and abort", Color::Red),
        ];

        for (i, (label, color)) in options.iter().enumerate() {
            let cursor = if i == self.selected { "❯ " } else { "  " };
            let style = if i == self.selected {
                Style::default().fg(*color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(LIGHT_GRAY)
            };
            lines.push(Line::from(vec![
                Span::styled(cursor, Style::default().fg(*color)),
                Span::styled(*label, style),
            ]));
        }

        // Footer hint
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "↑↓ select │ Enter confirm │ Esc reject",
            Style::default().fg(LIGHT_GRAY),
        )));

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}
