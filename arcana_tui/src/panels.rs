use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::types::{PanelState, TaskInfo, TaskStatus};

/// Light gray visible on transparent terminals
const LIGHT_GRAY: Color = Color::Rgb(160, 160, 170);

/// Calculate the height needed for the task panel.
pub fn task_panel_height(panel_state: &PanelState, tasks: &[TaskInfo]) -> u16 {
    if tasks.is_empty() {
        return 0;
    }
    if panel_state.tasks_expanded {
        1 + tasks.len() as u16
    } else {
        1
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

    let total = tasks.len();
    let toggle_hint = if panel_state.tasks_expanded {
        "ctrl+t to collapse"
    } else {
        "ctrl+t to expand"
    };

    let header_left = format!("● Tasks ({})", total);
    let pad = (area.width as usize).saturating_sub(header_left.len() + toggle_hint.len() + 1);

    lines.push(Line::from(vec![
        Span::styled(
            &header_left,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(pad)),
        Span::styled(toggle_hint, Style::default().fg(LIGHT_GRAY)),
    ]));

    if panel_state.tasks_expanded {
        for (i, task) in tasks.iter().enumerate() {
            let is_last = i == tasks.len() - 1;
            let branch = if is_last { "└── " } else { "├── " };

            // Green ● = done, Orange ● = blocked/failed, Empty ○ = pending
            let (icon, icon_color) = match task.status {
                TaskStatus::Completed => ("●", Color::Green),
                TaskStatus::InProgress => ("●", Color::Rgb(255, 165, 0)), // orange
                TaskStatus::Pending => ("○", LIGHT_GRAY),
            };

            lines.push(Line::from(vec![
                Span::styled(branch, Style::default().fg(LIGHT_GRAY)),
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
