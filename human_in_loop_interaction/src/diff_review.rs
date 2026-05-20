use std::path::Path;

use crate::config::HitlConfig;
use crate::types::{DiffHunk, HumanCorrection};

#[derive(Debug, thiserror::Error)]
pub enum DiffReviewError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("editor failed with code {0}")]
    EditorFailed(i32),
}

/// Compute a unified diff between old and new content.
pub fn compute_diff(file: &str, old: &str, new: &str) -> DiffHunk {
    let mut diff_lines = Vec::new();
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    // Simple line-by-line diff (in production, use a proper diff algorithm)
    let max = old_lines.len().max(new_lines.len());
    for i in 0..max {
        let ol = old_lines.get(i).copied().unwrap_or("");
        let nl = new_lines.get(i).copied().unwrap_or("");
        if ol != nl {
            if !ol.is_empty() { diff_lines.push(format!("-{ol}")); }
            if !nl.is_empty() { diff_lines.push(format!("+{nl}")); }
        } else {
            diff_lines.push(format!(" {ol}"));
        }
    }

    DiffHunk {
        file: file.to_string(),
        old_content: old.to_string(),
        new_content: new.to_string(),
        unified_diff: diff_lines.join("\n"),
    }
}

/// Format a diff hunk for terminal display (truncated to max_lines).
pub fn format_diff(hunk: &DiffHunk, max_lines: usize) -> String {
    let lines: Vec<&str> = hunk.unified_diff.lines().collect();
    let truncated = lines.len() > max_lines;
    let display: Vec<&str> = lines.iter().take(max_lines).copied().collect();

    let mut out = format!("--- {}\n+++ {} (proposed)\n", hunk.file, hunk.file);
    out.push_str(&display.join("\n"));
    if truncated {
        out.push_str(&format!("\n... ({} more lines, Ctrl+O to expand)", lines.len() - max_lines));
    }
    out
}

/// Open the user's editor on a file and wait for completion.
pub fn open_editor(config: &HitlConfig, file_path: &Path) -> Result<(), DiffReviewError> {
    let status = std::process::Command::new(&config.editor.command)
        .arg(file_path)
        .status()?;

    if !status.success() {
        return Err(DiffReviewError::EditorFailed(status.code().unwrap_or(-1)));
    }
    Ok(())
}

/// After human edits, compute the correction and generate context injection message.
pub fn compute_correction(file: &str, llm_content: &str, human_content: &str) -> Option<HumanCorrection> {
    if llm_content == human_content {
        return None; // No changes
    }

    let hunk = compute_diff(file, llm_content, human_content);

    Some(HumanCorrection {
        file: file.to_string(),
        llm_version: llm_content.to_string(),
        human_version: human_content.to_string(),
        diff: hunk.unified_diff,
    })
}

/// Generate the synthetic context message for the LLM after a human correction.
pub fn correction_to_context_message(correction: &HumanCorrection) -> String {
    format!(
        "User modified your proposed change to {}:\n--- Your version\n+++ User's version\n{}",
        correction.file, correction.diff
    )
}

/// Check if a diff is empty (no actual changes).
pub fn is_empty_diff(old: &str, new: &str) -> bool {
    old == new
}
