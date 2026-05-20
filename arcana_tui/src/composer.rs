use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::Theme;

/// The input composer at the bottom of the screen.
#[derive(Debug)]
pub struct Composer {
    /// Current input text
    pub input: String,
    /// Cursor position within the input
    pub cursor_pos: usize,
    /// History of sent messages (for recall with ↑)
    pub history: Vec<String>,
    /// Current history index (-1 = current input)
    pub history_index: Option<usize>,
    /// Whether the first-use hint should be shown
    pub show_hint: bool,
}

impl Default for Composer {
    fn default() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_index: None,
            show_hint: true,
        }
    }
}

impl Composer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.show_hint = false;
    }

    /// Insert a newline at the cursor position.
    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    /// Delete the character before the cursor.
    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            // Find the previous char boundary
            let prev = self.input[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.drain(prev..self.cursor_pos);
            self.cursor_pos = prev;
        }
    }

    /// Delete the character at the cursor.
    pub fn delete(&mut self) {
        if self.cursor_pos < self.input.len() {
            let next = self.input[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.input.len());
            self.input.drain(self.cursor_pos..next);
        }
    }

    /// Move cursor left.
    pub fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.input[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move cursor right.
    pub fn move_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos = self.input[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.input.len());
        }
    }

    /// Take the current input (consume it) and add to history.
    pub fn take_input(&mut self) -> String {
        let input = std::mem::take(&mut self.input);
        self.cursor_pos = 0;
        self.history_index = None;
        if !input.trim().is_empty() {
            self.history.push(input.clone());
        }
        input
    }

    /// Clear the input without adding to history.
    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
        self.history_index = None;
    }

    /// Recall previous message from history.
    pub fn recall_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_index {
            None => self.history.len() - 1,
            Some(i) => i.saturating_sub(1),
        };
        self.history_index = Some(idx);
        self.input = self.history[idx].clone();
        self.cursor_pos = self.input.len();
    }

    /// Check if the input is empty (ignoring whitespace).
    pub fn is_empty(&self) -> bool {
        self.input.trim().is_empty()
    }

    /// Get the number of lines in the input.
    pub fn line_count(&self) -> usize {
        self.input.lines().count().max(1)
    }

    /// Calculate the height needed for the composer.
    pub fn height(&self) -> u16 {
        let lines = self.line_count().min(10) as u16;
        lines + 2 // +2 for borders
    }

    /// Render the composer.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::TOP);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let in_slash_mode = self.input.starts_with('/');
        let prompt = if in_slash_mode { "/ " } else { "❯ " };

        // Build display content
        let display_content: &str = if self.input.is_empty() && self.show_hint {
            "[type / for commands, or enter message]"
        } else if in_slash_mode {
            &self.input[1..] // show without the leading /
        } else {
            &self.input
        };

        let content_style = if self.input.is_empty() && self.show_hint {
            theme.dim
        } else if in_slash_mode {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };

        let prompt_style = if in_slash_mode {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            theme.prompt_glyph
        };

        // Main input line
        let mut spans = vec![
            Span::styled(prompt, prompt_style),
            Span::styled(display_content.to_string(), content_style),
        ];

        // Slash command hints (shown inline when in slash mode and input is short)
        if in_slash_mode && self.input.len() <= 6 {
            let hint = match self.input.as_str() {
                "/" => " quit · help · model · mode · clear · status",
                "/q" | "/qu" | "/qui" | "/quit" => " ← exit session",
                "/h" | "/he" | "/hel" | "/help" => " ← show commands",
                "/mo" | "/mod" | "/mode" => " ← switch mode",
                "/m" | "/model" => " ← change model",
                "/c" | "/cl" | "/cle" | "/clea" | "/clear" => " ← clear viewport",
                "/s" | "/st" | "/sta" | "/stat" | "/statu" | "/status" => " ← show status",
                _ => "",
            };
            if !hint.is_empty() {
                spans.push(Span::styled(hint.to_string(), theme.dim));
            }
        }

        let paragraph = Paragraph::new(Line::from(spans));
        frame.render_widget(paragraph, inner);

        // Set cursor position
        let cursor_offset = if in_slash_mode {
            self.cursor_pos - 1 // account for hidden /
        } else {
            self.cursor_pos
        };
        let cursor_x = inner.x + prompt.len() as u16 + cursor_offset as u16;
        let cursor_y = inner.y;
        frame.set_cursor_position(Position::new(
            cursor_x.min(inner.x + inner.width - 1),
            cursor_y,
        ));
    }
}
