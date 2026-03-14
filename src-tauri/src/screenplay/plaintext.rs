// Plain text export: ProseMirror JSON → formatted plain text (.txt)
//
// Produces a readable screenplay in plain text with proper indentation
// following standard screenplay formatting conventions.

use crate::screenplay::document::ScreenplayMeta;
use serde_json::Value;

/// Characters per line for dialogue wrapping.
const DIALOGUE_WIDTH: usize = 35;

/// Left margin (spaces) for character names — centered at column 40.
const CHARACTER_INDENT: usize = 40;

/// Left margin (spaces) for parentheticals.
const PAREN_INDENT: usize = 35;

/// Left margin (spaces) for dialogue text.
const DIALOGUE_INDENT: usize = 25;

/// Total page width for right-aligning transitions.
const PAGE_WIDTH: usize = 60;

/// Generates a plain text screenplay from ProseMirror JSON content
/// and screenplay metadata.
///
/// # Arguments
/// * `content` — The ProseMirror document JSON (the `content` field of a .screenplay file)
/// * `meta` — Screenplay metadata for the header block
///
/// # Returns
/// A UTF-8 plain text string ready to write to a .txt file.
pub fn generate_plaintext(content: &Value, meta: &ScreenplayMeta) -> String {
    let mut output = String::new();

    // --- Metadata header block ---
    // Title, author, contact, and draft info centered or left-aligned at the top.
    let has_header = !meta.title.is_empty();
    if has_header {
        output.push_str(&meta.title.to_uppercase());
        output.push('\n');

        if !meta.author.is_empty() {
            output.push_str(&format!("by {}", meta.author));
            output.push('\n');
        }

        if !meta.contact.is_empty() {
            output.push_str(&meta.contact);
            output.push('\n');
        }

        if !meta.draft_date.is_empty() || meta.draft_number > 0 {
            let mut draft_line = String::new();
            if meta.draft_number > 0 {
                draft_line.push_str(&format!("Draft {}", meta.draft_number));
            }
            if !meta.draft_date.is_empty() {
                if !draft_line.is_empty() {
                    draft_line.push_str(" — ");
                }
                draft_line.push_str(&meta.draft_date);
            }
            output.push_str(&draft_line);
            output.push('\n');
        }

        // Separator between header and body
        output.push('\n');
        output.push('\n');
    }

    // --- Script body ---
    // Extract nodes from the ProseMirror JSON "content" array
    let nodes = match content.get("content").and_then(|c| c.as_array()) {
        Some(arr) => arr,
        None => return output,
    };

    // Track the previous element type for spacing decisions.
    let mut prev_type: Option<String> = None;

    for node in nodes {
        let element_type = match node.get("type").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => continue,
        };

        // Extract text by concatenating all child text nodes.
        let text = extract_text(node);

        match element_type {
            "scene_heading" => {
                // Scene headings: uppercase, full width, blank line before
                if prev_type.is_some() {
                    output.push('\n');
                }
                output.push_str(&text.to_uppercase());
                output.push('\n');
            }

            "action" => {
                // Action: full width, blank line before
                if prev_type.is_some() {
                    output.push('\n');
                }
                output.push_str(&text);
                output.push('\n');
            }

            "character" => {
                // Character names: indented to column 40, uppercase
                if prev_type.is_some() {
                    output.push('\n');
                }
                let name = text.trim().to_uppercase();
                output.push_str(&format!("{:>width$}", "", width = CHARACTER_INDENT));
                output.push_str(&name);
                output.push('\n');
            }

            "parenthetical" => {
                // Parentheticals: indented to column 35, wrapped in ()
                let trimmed = text.trim();
                let formatted = if trimmed.starts_with('(') && trimmed.ends_with(')') {
                    trimmed.to_string()
                } else {
                    format!("({})", trimmed)
                };
                output.push_str(&format!("{:>width$}", "", width = PAREN_INDENT));
                output.push_str(&formatted);
                output.push('\n');
            }

            "dialogue" => {
                // Dialogue: indented to column 25, wrapped at 35 characters
                let wrapped = word_wrap(&text, DIALOGUE_WIDTH);
                for line in wrapped.lines() {
                    output.push_str(&format!("{:>width$}", "", width = DIALOGUE_INDENT));
                    output.push_str(line);
                    output.push('\n');
                }
            }

            "transition" => {
                // Transitions: right-aligned to page width, blank line before
                if prev_type.is_some() {
                    output.push('\n');
                }
                let trimmed = text.trim();
                // Right-align: pad with spaces so the text ends at PAGE_WIDTH
                let padding = if trimmed.len() < PAGE_WIDTH {
                    PAGE_WIDTH - trimmed.len()
                } else {
                    0
                };
                output.push_str(&format!("{:>width$}", "", width = padding));
                output.push_str(trimmed);
                output.push('\n');
            }

            _ => {
                // Unknown element: write as plain text with blank line before
                if prev_type.is_some() {
                    output.push('\n');
                }
                output.push_str(&text);
                output.push('\n');
            }
        }

        prev_type = Some(element_type.to_string());
    }

    output
}

/// Extract the text content from a ProseMirror node by concatenating
/// all child text nodes.
fn extract_text(node: &Value) -> String {
    let mut text = String::new();

    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            if let Some(t) = child.get("text").and_then(|t| t.as_str()) {
                text.push_str(t);
            }
        }
    }

    text
}

/// Word-wrap text at the given width, breaking at word boundaries.
/// Handles Unicode text (Malayalam) by counting characters, not bytes.
fn word_wrap(text: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut current_line = String::new();
    // `char_count` tracks the number of characters in the current line.
    let mut char_count: usize = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        if char_count > 0 && char_count + 1 + word_len > max_width {
            // Current line is full — start a new line
            result.push_str(&current_line);
            result.push('\n');
            current_line = word.to_string();
            char_count = word_len;
        } else {
            if char_count > 0 {
                current_line.push(' ');
                char_count += 1;
            }
            current_line.push_str(word);
            char_count += word_len;
        }
    }

    // Don't forget the last line
    if !current_line.is_empty() {
        result.push_str(&current_line);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screenplay::document::ScreenplayMeta;
    use serde_json::json;

    #[test]
    fn test_empty_document() {
        let content = json!({"type": "doc", "content": []});
        let meta = ScreenplayMeta::default();
        let result = generate_plaintext(&content, &meta);
        assert_eq!(result, "");
    }

    #[test]
    fn test_header_block() {
        let content = json!({"type": "doc", "content": []});
        let meta = ScreenplayMeta {
            title: "My Script".to_string(),
            author: "Hrishi".to_string(),
            draft_number: 2,
            draft_date: "2026-03-14".to_string(),
            contact: "hello@example.com".to_string(),
            ..Default::default()
        };
        let result = generate_plaintext(&content, &meta);
        assert!(result.contains("MY SCRIPT"));
        assert!(result.contains("by Hrishi"));
        assert!(result.contains("hello@example.com"));
        assert!(result.contains("Draft 2 — 2026-03-14"));
    }

    #[test]
    fn test_scene_heading_uppercase() {
        let content = json!({
            "type": "doc",
            "content": [
                {
                    "type": "scene_heading",
                    "content": [{"type": "text", "text": "Int. Coffee Shop - Day"}]
                }
            ]
        });
        let meta = ScreenplayMeta::default();
        let result = generate_plaintext(&content, &meta);
        assert!(result.contains("INT. COFFEE SHOP - DAY"));
    }

    #[test]
    fn test_character_indented() {
        let content = json!({
            "type": "doc",
            "content": [
                {
                    "type": "character",
                    "content": [{"type": "text", "text": "JOHN"}]
                }
            ]
        });
        let meta = ScreenplayMeta::default();
        let result = generate_plaintext(&content, &meta);
        // Character should be indented 40 spaces
        let line = result.lines().next().unwrap();
        let leading_spaces = line.len() - line.trim_start().len();
        assert_eq!(leading_spaces, CHARACTER_INDENT);
        assert!(line.trim() == "JOHN");
    }

    #[test]
    fn test_dialogue_indented_and_wrapped() {
        let content = json!({
            "type": "doc",
            "content": [
                {
                    "type": "dialogue",
                    "content": [{"type": "text", "text": "This is a longer dialogue line that should be wrapped at the specified width for readability."}]
                }
            ]
        });
        let meta = ScreenplayMeta::default();
        let result = generate_plaintext(&content, &meta);
        // Each dialogue line should be indented 25 spaces
        for line in result.lines() {
            if !line.is_empty() {
                let leading_spaces = line.len() - line.trim_start().len();
                assert_eq!(leading_spaces, DIALOGUE_INDENT);
            }
        }
    }

    #[test]
    fn test_transition_right_aligned() {
        let content = json!({
            "type": "doc",
            "content": [
                {
                    "type": "transition",
                    "content": [{"type": "text", "text": "CUT TO:"}]
                }
            ]
        });
        let meta = ScreenplayMeta::default();
        let result = generate_plaintext(&content, &meta);
        let line = result.lines().next().unwrap();
        // "CUT TO:" is 7 chars, right-aligned to 60 = 53 spaces padding
        assert!(line.ends_with("CUT TO:"));
        let leading_spaces = line.len() - line.trim_start().len();
        assert_eq!(leading_spaces, PAGE_WIDTH - "CUT TO:".len());
    }

    #[test]
    fn test_parenthetical_with_parens() {
        let content = json!({
            "type": "doc",
            "content": [
                {
                    "type": "parenthetical",
                    "content": [{"type": "text", "text": "softly"}]
                }
            ]
        });
        let meta = ScreenplayMeta::default();
        let result = generate_plaintext(&content, &meta);
        let line = result.lines().next().unwrap();
        assert!(line.contains("(softly)"));
        let leading_spaces = line.len() - line.trim_start().len();
        assert_eq!(leading_spaces, PAREN_INDENT);
    }

    #[test]
    fn test_word_wrap() {
        let text = "Hello world this is a test of word wrapping";
        let wrapped = word_wrap(text, 15);
        for line in wrapped.lines() {
            // Each line should be at most 15 chars (unless a single word exceeds it)
            assert!(line.chars().count() <= 15, "Line too long: '{}'", line);
        }
    }

    #[test]
    fn test_full_screenplay() {
        let content = json!({
            "type": "doc",
            "content": [
                {
                    "type": "scene_heading",
                    "content": [{"type": "text", "text": "INT. OFFICE - DAY"}]
                },
                {
                    "type": "action",
                    "content": [{"type": "text", "text": "John walks in."}]
                },
                {
                    "type": "character",
                    "content": [{"type": "text", "text": "JOHN"}]
                },
                {
                    "type": "dialogue",
                    "content": [{"type": "text", "text": "Good morning."}]
                },
                {
                    "type": "transition",
                    "content": [{"type": "text", "text": "CUT TO:"}]
                }
            ]
        });
        let meta = ScreenplayMeta {
            title: "Test Script".to_string(),
            author: "Writer".to_string(),
            ..Default::default()
        };
        let result = generate_plaintext(&content, &meta);
        assert!(result.contains("TEST SCRIPT"));
        assert!(result.contains("by Writer"));
        assert!(result.contains("INT. OFFICE - DAY"));
        assert!(result.contains("John walks in."));
        assert!(result.contains("JOHN"));
        assert!(result.contains("Good morning."));
        assert!(result.contains("CUT TO:"));
    }
}
