use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::types::{PanelState, TaskInfo, TaskStatus};

/// Calculate the height needed for the task panel.
pub fn task_panel_height(panel_state: &PanelState, tasks: &[TaskInfo]) -> u16 {
    if tasks.is_empty() {
        return 0;
    }
    if panel_state.tasks_expanded {
        // header + one line per task
        1 + tasks.len() as u16
    } else {
        1 // collapsed: just the header
    }
}

/// Render the task panel (Kiro-style with tree indicators).
pub fn render_task_panel(
    frame: &mut Frame,
    area: Rect,
    panel_state: &PanelState,
    tasks: &[TaskInfo],
) {
    if tasks.is_empty() || area.height == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Header line: "● Tasks (N)" on left, "ctrl+t to expand/collapse" on right
    let total = tasks.len();
    let toggle_hint = if panel_state.tasks_expanded {
        "ctrl+t to collapse"
    } else {
        "ctrl+t to expand"
    };

    // Calculate padding for right-aligned hint
    let header_left = format!("● Tasks ({})", total);
    let pad = (area.width as usize).saturating_sub(header_left.len() + toggle_hint.len() + 1);

    lines.push(Line::from(vec![
        Span::styled(&header_left, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw(" ".repeat(pad)),
        Span::styled(toggle_hint, Style::default().fg(Color::DarkGray)),
    ]));

    if panel_state.tasks_expanded {
        for (i, task) in tasks.iter().enumerate() {
            let is_last = i == tasks.len() - 1;
            let branch = if is_last { "└── " } else { "├── " };

            let (icon, icon_color) = match task.status {
                TaskStatus::Completed => ("●", Color::Green),
                TaskStatus::InProgress => ("◉", Color::Blue),
                TaskStatus::Pending => ("○", Color::DarkGray),
            };

            lines.push(Line::from(vec![
                Span::styled(branch, Style::default().fg(Color::DarkGray)),
                Span::styled(icon, Style::default().fg(icon_color)),
                Span::raw(" "),
                Span::styled(
                    format!("{}. {}", i + 1, task.name),
                    Style::default().fg(Color::White),
                ),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}
