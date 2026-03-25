// Fountain export: ProseMirror JSON → Fountain plain text (.fountain)
//
// Fountain is a plain-text screenwriting format (fountain.io).
// This module converts the ProseMirror document JSON and screenplay metadata
// into a UTF-8 Fountain string that can be read by Highland, Fade In, etc.

use crate::screenplay::document::ScreenplayMeta;
use serde_json::Value;

/// Generates a Fountain-formatted string from ProseMirror JSON content
/// and screenplay metadata.
///
/// # Arguments
/// * `content` — The ProseMirror document JSON (the `content` field of a .screenplay file)
/// * `meta` — Screenplay metadata for the title page block
///
/// # Returns
/// A UTF-8 Fountain string ready to write to a .fountain file.
pub fn generate_fountain(content: &Value, meta: &ScreenplayMeta) -> String {
    let mut output = String::new();

    // --- Title page block ---
    // Fountain title page is key-value pairs at the top of the file,
    // separated from the script body by a blank line.
    let has_title_page = !meta.title.is_empty();
    if has_title_page {
        output.push_str(&format!("Title: {}\n", meta.title));

        if !meta.author.is_empty() {
            output.push_str(&format!("Author: {}\n", meta.author));
        }
        if !meta.draft_date.is_empty() {
            output.push_str(&format!("Draft date: {}\n", meta.draft_date));
        }
        if !meta.contact.is_empty() {
            // Contact can be multi-line — Fountain supports indented continuation lines
            // for multi-line values. Each line after the first is indented with spaces.
            let lines: Vec<&str> = meta.contact.lines().collect();
            if lines.len() == 1 {
                output.push_str(&format!("Contact: {}\n", lines[0]));
            } else {
                // First line on the Contact: key line, subsequent lines indented
                output.push_str(&format!("Contact: {}\n", lines[0]));
                for line in &lines[1..] {
                    output.push_str(&format!("    {}\n", line));
                }
            }
        }

        // Blank line separates title page from script body
        output.push('\n');
    }

    // --- Script body ---
    // Extract nodes from the ProseMirror JSON "content" array
    let nodes = match content.get("content").and_then(|c| c.as_array()) {
        Some(arr) => arr,
        None => return output, // No content — return title page only
    };

    // Track the previous element type to determine blank line spacing.
    // Fountain uses blank lines to separate elements.
    let mut prev_type: Option<String> = None;

    for node in nodes {
        // Each node has a "type" field (e.g. "scene_heading", "action", etc.)
        let element_type = match node.get("type").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => continue,
        };

        // Extract text by concatenating all child text nodes.
        // ProseMirror stores text as: { "content": [{ "type": "text", "text": "..." }] }
        let text = extract_text(node);

        // Write the Fountain-formatted element
        match element_type {
            "scene_heading" => {
                // Fountain auto-detects scene headings starting with INT./EXT./etc.
                // For headings that don't match, force them with a leading period.
                // We always add a blank line before scene headings.
                if prev_type.is_some() {
                    output.push('\n');
                }

                if is_auto_detected_heading(&text) {
                    output.push_str(&text);
                } else {
                    // Force scene heading with a leading period
                    output.push('.');
                    output.push_str(&text);
                }
                output.push('\n');
            }

            "action" => {
                // Action is plain paragraph text. Blank line before if previous
                // element was not also action (consecutive actions need blank lines
                // in Fountain to be separate paragraphs).
                if prev_type.is_some() {
                    output.push('\n');
                }
                output.push_str(&text);
                output.push('\n');
            }

            "character" => {
                // Character names must be all-caps in Fountain for auto-detection.
                // Our schema already auto-uppercases Latin characters, so we write as-is.
                // For Malayalam character names, Fountain won't auto-detect them,
                // so we force with @ prefix.
                if prev_type.is_some() {
                    output.push('\n');
                }

                if is_all_ascii_upper(&text) {
                    // Standard all-caps character — Fountain auto-detects
                    output.push_str(&text);
                } else {
                    // Contains non-ASCII (Malayalam) or mixed case — force with @
                    output.push('@');
                    output.push_str(&text);
                }
                output.push('\n');
            }

            "dialogue" => {
                // Dialogue follows a character line with no blank line between.
                // Just the text on its own line.
                output.push_str(&text);
                output.push('\n');
            }

            "parenthetical" => {
                // Parentheticals follow a character or dialogue line.
                // Must be wrapped in parentheses. Our schema may or may not
                // include them, so ensure they're there.
                let trimmed = text.trim();
                if trimmed.starts_with('(') && trimmed.ends_with(')') {
                    output.push_str(trimmed);
                } else {
                    output.push('(');
                    output.push_str(trimmed);
                    output.push(')');
                }
                output.push('\n');
            }

            "transition" => {
                // Transitions in Fountain are detected when a line ends with "TO:"
                // and is all-caps. For others, force with a leading ">".
                if prev_type.is_some() {
                    output.push('\n');
                }

                let trimmed = text.trim();
                if trimmed.to_uppercase().ends_with("TO:") && is_all_ascii_upper(trimmed) {
                    output.push_str(trimmed);
                } else {
                    output.push('>');
                    output.push_str(trimmed);
                }
                output.push('\n');
            }

            _ => {
                // Unknown element type — write as plain text
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
/// all child text nodes. Preserves bold formatting using Fountain's
/// `**bold**` syntax.
///
/// ProseMirror stores inline formatting as "marks" on text nodes:
/// ```json
/// { "type": "text", "text": "bold word", "marks": [{ "type": "bold" }] }
/// ```
/// Bold text is wrapped in `**...**` for Fountain. Non-bold text is plain.
fn extract_text(node: &Value) -> String {
    let mut text = String::new();

    // `node["content"]` is an array of child nodes, each with type "text"
    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            if let Some(t) = child.get("text").and_then(|t| t.as_str()) {
                // Check which marks are applied to this text node.
                // Fountain supports: **bold**, *italic*, _underline_
                let marks_array = child.get("marks").and_then(|m| m.as_array());

                let has_mark = |mark_type: &str| -> bool {
                    marks_array
                        .map(|marks| {
                            marks.iter().any(|mark| {
                                mark.get("type").and_then(|t| t.as_str()) == Some(mark_type)
                            })
                        })
                        .unwrap_or(false)
                };

                let is_bold = has_mark("bold");
                let is_italic = has_mark("italic");
                let is_underline = has_mark("underline");

                // Build Fountain-formatted text with mark wrappers.
                // Fountain uses: **bold**, *italic*, _underline_
                // When combined, nest them: ***bold italic***, **_bold underline_**
                if is_bold { text.push_str("**"); }
                if is_italic { text.push('*'); }
                if is_underline { text.push('_'); }
                text.push_str(t);
                if is_underline { text.push('_'); }
                if is_italic { text.push('*'); }
                if is_bold { text.push_str("**"); }
            }
        }
    }

    text
}

/// Check if text starts with a Fountain auto-detected scene heading prefix.
/// Fountain auto-detects: INT., EXT., INT./EXT., EST., I/E.
fn is_auto_detected_heading(text: &str) -> bool {
    let upper = text.to_uppercase();
    upper.starts_with("INT.")
        || upper.starts_with("INT ")
        || upper.starts_with("EXT.")
        || upper.starts_with("EXT ")
        || upper.starts_with("INT./EXT.")
        || upper.starts_with("INT/EXT")
        || upper.starts_with("EST.")
        || upper.starts_with("I/E.")
}

/// Check if a string contains only uppercase ASCII letters, spaces, digits,
/// and common punctuation — i.e., no lowercase ASCII letters.
/// Returns false for empty strings.
fn is_all_ascii_upper(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    // Only check ASCII letters — non-ASCII (Malayalam) chars don't have
    // upper/lower distinction, so we consider them "not all ASCII upper".
    let has_ascii_letters = text.chars().any(|c| c.is_ascii_alphabetic());
    if !has_ascii_letters {
        return false;
    }
    !text.chars().any(|c| c.is_ascii_lowercase())
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
        let result = generate_fountain(&content, &meta);
        assert_eq!(result, "");
    }

    #[test]
    fn test_title_page() {
        let content = json!({"type": "doc", "content": []});
        let meta = ScreenplayMeta {
            title: "My Script".to_string(),
            author: "Hrishi".to_string(),
            draft_date: "2026-03-14".to_string(),
            contact: "hello@example.com".to_string(),
            ..Default::default()
        };
        let result = generate_fountain(&content, &meta);
        assert!(result.contains("Title: My Script"));
        assert!(result.contains("Author: Hrishi"));
        assert!(result.contains("Draft date: 2026-03-14"));
        assert!(result.contains("Contact: hello@example.com"));
    }

    #[test]
    fn test_scene_heading_auto_detect() {
        let content = json!({
            "type": "doc",
            "content": [
                {
                    "type": "scene_heading",
                    "content": [{"type": "text", "text": "INT. COFFEE SHOP - DAY"}]
                }
            ]
        });
        let meta = ScreenplayMeta::default();
        let result = generate_fountain(&content, &meta);
        assert_eq!(result.trim(), "INT. COFFEE SHOP - DAY");
    }

    #[test]
    fn test_scene_heading_forced() {
        let content = json!({
            "type": "doc",
            "content": [
                {
                    "type": "scene_heading",
                    "content": [{"type": "text", "text": "THE BEACH"}]
                }
            ]
        });
        let meta = ScreenplayMeta::default();
        let result = generate_fountain(&content, &meta);
        assert_eq!(result.trim(), ".THE BEACH");
    }

    #[test]
    fn test_character_dialogue_block() {
        let content = json!({
            "type": "doc",
            "content": [
                {
                    "type": "character",
                    "content": [{"type": "text", "text": "JOHN"}]
                },
                {
                    "type": "dialogue",
                    "content": [{"type": "text", "text": "Hello there."}]
                }
            ]
        });
        let meta = ScreenplayMeta::default();
        let result = generate_fountain(&content, &meta);
        assert!(result.contains("JOHN\nHello there.\n"));
    }

    #[test]
    fn test_parenthetical() {
        let content = json!({
            "type": "doc",
            "content": [
                {
                    "type": "character",
                    "content": [{"type": "text", "text": "MARY"}]
                },
                {
                    "type": "parenthetical",
                    "content": [{"type": "text", "text": "softly"}]
                },
                {
                    "type": "dialogue",
                    "content": [{"type": "text", "text": "I know."}]
                }
            ]
        });
        let meta = ScreenplayMeta::default();
        let result = generate_fountain(&content, &meta);
        assert!(result.contains("MARY\n(softly)\nI know.\n"));
    }

    #[test]
    fn test_transition() {
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
        let result = generate_fountain(&content, &meta);
        assert_eq!(result.trim(), "CUT TO:");
    }

    #[test]
    fn test_malayalam_character_forced() {
        let content = json!({
            "type": "doc",
            "content": [
                {
                    "type": "character",
                    "content": [{"type": "text", "text": "രമേഷ്"}]
                },
                {
                    "type": "dialogue",
                    "content": [{"type": "text", "text": "നമസ്കാരം"}]
                }
            ]
        });
        let meta = ScreenplayMeta::default();
        let result = generate_fountain(&content, &meta);
        // Malayalam character names get forced with @ prefix
        assert!(result.contains("@രമേഷ്\n"));
        assert!(result.contains("നമസ്കാരം\n"));
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
                },
                {
                    "type": "scene_heading",
                    "content": [{"type": "text", "text": "EXT. PARK - NIGHT"}]
                }
            ]
        });
        let meta = ScreenplayMeta {
            title: "Test Script".to_string(),
            author: "Writer".to_string(),
            ..Default::default()
        };
        let result = generate_fountain(&content, &meta);

        assert!(result.starts_with("Title: Test Script\n"));
        assert!(result.contains("INT. OFFICE - DAY\n"));
        assert!(result.contains("\nJohn walks in.\n"));
        assert!(result.contains("\nJOHN\nGood morning.\n"));
        assert!(result.contains("\nCUT TO:\n"));
        assert!(result.contains("\nEXT. PARK - NIGHT\n"));
    }
}
