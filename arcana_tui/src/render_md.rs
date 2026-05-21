use ratatui::prelude::*;
use syntect::highlighting::{ThemeSet, Style as SynStyle};
use syntect::parsing::SyntaxSet;
use syntect::easy::HighlightLines;

/// Blue color for inline code
const CODE_BLUE: Color = Color::Rgb(0, 128, 255);

/// Render a markdown text block into styled Lines.
/// - Inline `code` → blue without backticks
/// - ```lang code blocks → syntax highlighted without fences
/// - Compact double+ newlines to single (except before # or ---)
pub fn render_markdown<'a>(text: &str, base_style: Style) -> Vec<Line<'a>> {
    let compacted = compact_newlines(text);
    let mut result: Vec<Line<'a>> = Vec::new();
    let mut lines_iter = compacted.lines().peekable();
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes["base16-ocean.dark"];

    while let Some(line) = lines_iter.next() {
        // Check for code block start
        if line.starts_with("```") {
            let lang = line.trim_start_matches('`').trim();
            let syntax = if lang.is_empty() {
                ss.find_syntax_plain_text()
            } else {
                ss.find_syntax_by_token(lang).unwrap_or_else(|| ss.find_syntax_plain_text())
            };
            let mut highlighter = HighlightLines::new(syntax, theme);

            // Collect code block lines until closing ```
            while let Some(code_line) = lines_iter.next() {
                if code_line.starts_with("```") {
                    break;
                }
                // Syntax highlight this line
                if let Ok(ranges) = highlighter.highlight_line(code_line, &ss) {
                    let spans: Vec<Span<'a>> = ranges.into_iter().map(|(style, text)| {
                        Span::styled(text.to_string(), syn_to_ratatui(style))
                    }).collect();
                    result.push(Line::from(spans));
                } else {
                    result.push(Line::from(Span::styled(code_line.to_string(), base_style)));
                }
            }
        } else {
            // Regular line — parse inline code
            result.push(parse_inline_code(line, base_style));
        }
    }
    result
}

/// Parse a single line for inline `code` spans.
/// Removes backticks, renders code content in blue.
fn parse_inline_code<'a>(text: &str, base_style: Style) -> Line<'a> {
    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find('`') {
        // Text before backtick
        if start > 0 {
            spans.push(Span::styled(rest[..start].to_string(), base_style));
        }
        let after = &rest[start + 1..];
        if let Some(end) = after.find('`') {
            // Render code content without backticks, in blue
            let code = &after[..end];
            spans.push(Span::styled(code.to_string(), Style::default().fg(CODE_BLUE)));
            rest = &after[end + 1..];
        } else {
            // No closing backtick
            spans.push(Span::styled(rest[start..].to_string(), base_style));
            rest = "";
            break;
        }
    }
    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_string(), base_style));
    }
    if spans.is_empty() {
        spans.push(Span::styled(String::new(), base_style));
    }
    Line::from(spans)
}

/// Compact multiple consecutive newlines to single newline,
/// except when the next line starts with # or ---
fn compact_newlines(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_empty = false;

    for line in text.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_was_empty {
                prev_was_empty = true;
                // Don't add yet — check next line
            }
            // Skip additional empty lines
        } else {
            if prev_was_empty {
                // Keep the blank line only if next line is # or ---
                if trimmed.starts_with('#') || trimmed.starts_with("---") {
                    result.push('\n');
                }
                prev_was_empty = false;
            }
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
        }
    }
    result
}

/// Convert syntect Style to ratatui Style
fn syn_to_ratatui(style: SynStyle) -> Style {
    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
    Style::default().fg(fg)
}
