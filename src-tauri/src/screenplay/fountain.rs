// Fountain export: ProseMirror JSON → Fountain plain text (.fountain)
//
// Fountain is a plain-text screenwriting format (https://fountain.io/syntax).
// This module converts the ProseMirror document JSON, screenplay metadata,
// and scene cards into a UTF-8 Fountain string that can be read by
// Highland, Slugline, Beat, Final Draft (via plugin), etc.
//
// Round-trip story (#188):
//   - meta.extra (#185) emits as additional title-page keys, sorted
//     alphabetically for diff-stable output
//   - scene_cards[i].description → `= synopsis` line(s) directly under
//     scene i's heading, so a Scriptty-edited file imported into Highland
//     surfaces the writer's notes in outline mode
//   - scene_cards[i].shoot_notes lines prefixed `[[#section depth=N]] ...`
//     emit as `# heading` (or `##`/`###` for nested depths) BEFORE the
//     scene heading. The marker syntax is internal — the importer mints
//     it; writers shouldn't author it by hand.
//   - shoot_notes lines without the section marker emit as inline
//     `[[ ... ]]` notes after the scene's body content
//
// Forcing rules (also #188 — preventing the next reader from misparsing
// a stored element):
//   - character cues that aren't pure all-caps Latin → `@` prefix
//     (Malayalam, mixed-case English, anything caseless)
//   - scene headings without a recognised slug prefix → `.` prefix
//   - transitions that aren't all-caps Latin ending in `TO:` → `>` prefix
//   - action lines that would auto-detect as something else → `!` prefix
//     (all-caps action like `BANG!`, all-caps action ending `TO:`,
//     action lines starting with `INT/EXT/EST/I/E`)
//
// Without the forcing layer, a Scriptty → Fountain → Scriptty round-trip
// would silently corrupt mixed-case content and any all-caps action.

use crate::screenplay::document::{SceneCard, ScreenplayMeta};
use serde_json::Value;
use std::collections::HashMap;

/// Generates a Fountain-formatted string from ProseMirror JSON content,
/// screenplay metadata, and per-scene cards (for synopses / sections /
/// notes).
///
/// # Arguments
/// * `content` — the ProseMirror document JSON
/// * `meta` — title-page metadata
/// * `scene_cards` — per-scene description / shoot_notes — emitted back
///   into Fountain as synopses, sections, and inline notes for round-trip
///
/// # Returns
/// A UTF-8 Fountain string ready to write to a `.fountain` file.
pub fn generate_fountain(
    content: &Value,
    meta: &ScreenplayMeta,
    scene_cards: &[SceneCard],
) -> String {
    let mut output = String::new();

    emit_title_page(&mut output, meta);

    // Pre-process scene cards into per-scene buckets so the body walker
    // can flush them at the right moments without doing the parsing work
    // mid-loop.
    let buckets = build_scene_buckets(scene_cards);

    let nodes = match content.get("content").and_then(|c| c.as_array()) {
        Some(arr) => arr,
        None => return output,
    };

    emit_body(&mut output, nodes, &buckets);

    output
}

// ─── Title page ──────────────────────────────────────────────────────────────

/// Emit the title-page block, including the standard keys from `meta` and
/// any non-standard keys preserved in `meta.extra` (sorted alphabetically
/// since `BTreeMap` already iterates that way).
fn emit_title_page(output: &mut String, meta: &ScreenplayMeta) {
    // The block is "interesting" if any first-class field OR any extra key
    // is set — without this guard, an empty doc with just `extra` keys
    // would skip the title-page entirely.
    let has_standard = !meta.title.is_empty()
        || !meta.author.is_empty()
        || !meta.draft_date.is_empty()
        || !meta.contact.is_empty()
        || !meta.registration_number.is_empty()
        || !meta.footnote.is_empty();
    let has_extra = !meta.extra.is_empty();
    if !has_standard && !has_extra {
        return;
    }

    if !meta.title.is_empty() {
        output.push_str(&format!("Title: {}\n", meta.title));
    }
    if !meta.author.is_empty() {
        output.push_str(&format!("Author: {}\n", meta.author));
    }
    if !meta.draft_date.is_empty() {
        output.push_str(&format!("Draft date: {}\n", meta.draft_date));
    }
    if !meta.contact.is_empty() {
        // Fountain title-page values can be multi-line: continuation
        // lines indented 3+ spaces or a tab. Use 4 spaces — readable in
        // monospaced viewers, and matches how Highland formats its own
        // exports.
        let lines: Vec<&str> = meta.contact.lines().collect();
        if lines.len() == 1 {
            output.push_str(&format!("Contact: {}\n", lines[0]));
        } else {
            output.push_str(&format!("Contact: {}\n", lines[0]));
            for line in &lines[1..] {
                output.push_str(&format!("    {}\n", line));
            }
        }
    }
    if !meta.registration_number.is_empty() {
        // Imported from `Copyright:` (when meta.registration_number was
        // empty at import time) — emit back to the same key so the
        // round-trip is symmetric. Writers who set this manually for
        // WGA/registration purposes get it labelled as Copyright in the
        // Fountain output, which is the closest standard key.
        output.push_str(&format!("Copyright: {}\n", meta.registration_number));
    }
    if !meta.footnote.is_empty() {
        // `meta.footnote` is the cover-bottom dedication / confidentiality
        // line. The matching Fountain key is `Notes:` — that's where
        // imports landed, so re-emit there.
        let lines: Vec<&str> = meta.footnote.lines().collect();
        if lines.len() == 1 {
            output.push_str(&format!("Notes: {}\n", lines[0]));
        } else {
            output.push_str(&format!("Notes: {}\n", lines[0]));
            for line in &lines[1..] {
                output.push_str(&format!("    {}\n", line));
            }
        }
    }

    // `meta.extra` (BTreeMap) iterates keys in sorted order — diff-stable
    // and predictable for round-trips. Original key spelling is preserved.
    for (key, value) in &meta.extra {
        let lines: Vec<&str> = value.lines().collect();
        if lines.len() <= 1 {
            output.push_str(&format!("{}: {}\n", key, value));
        } else {
            output.push_str(&format!("{}: {}\n", key, lines[0]));
            for line in &lines[1..] {
                output.push_str(&format!("    {}\n", line));
            }
        }
    }

    // Blank line separates title page from body.
    output.push('\n');
}

// ─── Scene-card buckets ──────────────────────────────────────────────────────

/// Per-scene buckets the body walker consults when emitting each scene.
/// `sections` and `notes` are derived from `shoot_notes` (depth-marker
/// prefix vs not); `synopsis_lines` is just `description` split on
/// newlines. Empty buckets are still inserted so a quick `.contains_key`
/// can disambiguate "no card" from "card with nothing to emit".
#[derive(Debug, Default)]
struct SceneBucket {
    /// `(depth, text)` pairs to emit as `#`/`##`/... before the scene.
    sections: Vec<(u32, String)>,
    /// Lines to emit as `= line` synopses after the scene heading.
    synopsis_lines: Vec<String>,
    /// Free-form lines to emit as inline `[[ line ]]` notes at scene end.
    notes: Vec<String>,
}

fn build_scene_buckets(cards: &[SceneCard]) -> HashMap<usize, SceneBucket> {
    let mut buckets: HashMap<usize, SceneBucket> = HashMap::new();
    for card in cards {
        let bucket = buckets.entry(card.scene_index).or_default();

        // Description → synopsis lines (one `= line` per non-empty entry).
        for line in card.description.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                bucket.synopsis_lines.push(trimmed.to_string());
            }
        }

        // shoot_notes — the importer joins multiple entries with `\n`,
        // so split and route per line.
        for line in card.shoot_notes.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((depth, text)) = parse_section_marker(trimmed) {
                bucket.sections.push((depth, text));
            } else {
                bucket.notes.push(trimmed.to_string());
            }
        }
    }
    buckets
}

/// Parse a `[[#section depth=N]] text` marker. Returns `Some((depth, text))`
/// when the input matches, `None` for any other shape — including the
/// many ways a writer might author similar-looking text by hand. The
/// strict format keeps the parser predictable and prevents accidental
/// re-promotion of inline notes that happen to mention sections.
fn parse_section_marker(line: &str) -> Option<(u32, String)> {
    let rest = line.strip_prefix("[[#section depth=")?;
    // Find the closing `]] `.
    let end = rest.find("]] ")?;
    let depth: u32 = rest[..end].parse().ok()?;
    if !(1..=6).contains(&depth) {
        return None;
    }
    let text = rest[end + 3..].trim();
    Some((depth, text.to_string()))
}

// ─── Body emission ───────────────────────────────────────────────────────────

fn emit_body(output: &mut String, nodes: &[Value], buckets: &HashMap<usize, SceneBucket>) {
    // Track the current scene index — incremented on each `scene_heading`
    // node, starting from 0. `-1` (using i64 for the sentinel) means
    // "haven't seen a scene heading yet".
    let mut current_scene: i64 = -1;
    let mut prev_kind: Option<String> = None;

    for node in nodes {
        let kind = match node.get("type").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => continue,
        };
        let text = extract_text(node);

        match kind {
            "scene_heading" => {
                // Flush the previous scene's pending notes before the new
                // scene header — the writer expects `[[ ... ]]` notes to
                // sit at the end of the scene they belong to.
                if current_scene >= 0 {
                    flush_scene_notes(output, buckets, current_scene as usize);
                }

                current_scene += 1;
                let scene_idx = current_scene as usize;

                // Sections attach to the scene that *follows* them.
                if prev_kind.is_some() {
                    output.push('\n');
                }
                if let Some(bucket) = buckets.get(&scene_idx) {
                    for (depth, txt) in &bucket.sections {
                        let pounds: String = "#".repeat(*depth as usize);
                        output.push_str(&format!("{} {}\n\n", pounds, txt));
                    }
                }

                output.push_str(&force_scene_heading(&text));
                output.push('\n');

                // Synopses go directly under the heading, one `=` per line.
                if let Some(bucket) = buckets.get(&scene_idx) {
                    for line in &bucket.synopsis_lines {
                        output.push_str(&format!("= {}\n", line));
                    }
                }
            }

            "action" => {
                if prev_kind.is_some() {
                    output.push('\n');
                }
                output.push_str(&force_action(&text));
                output.push('\n');
            }

            "character" => {
                if prev_kind.is_some() {
                    output.push('\n');
                }
                output.push_str(&force_character(&text));
                output.push('\n');
            }

            "dialogue" => {
                // Dialogue follows a character/parenthetical line with no
                // blank line between. The two-space-trailing convention
                // for keeping a dialogue block alive across paragraph-
                // looking breaks isn't relevant here because Scriptty's
                // schema doesn't model multi-paragraph dialogue blocks
                // (each Dialogue node is a single block).
                output.push_str(&text);
                output.push('\n');
            }

            "parenthetical" => {
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
                if prev_kind.is_some() {
                    output.push('\n');
                }
                output.push_str(&force_transition(&text));
                output.push('\n');
            }

            _ => {
                // Unknown element type (e.g. `episode_boundary` from
                // series exports — the PDF pipeline injects these).
                // Skip rather than emit garbage; series Fountain export
                // writes per-episode files (#187), so cross-episode
                // separators don't belong in the per-episode output.
                continue;
            }
        }

        prev_kind = Some(kind.to_string());
    }

    // Flush the final scene's notes at EOF.
    if current_scene >= 0 {
        flush_scene_notes(output, buckets, current_scene as usize);
    }
}

/// Emit any pending inline notes for `scene_idx` as `[[ ... ]]` blocks.
/// Each note gets its own paragraph (preceded by a blank line) so the
/// next reader's parser sees them as standalone notes rather than a
/// trailing annotation on whatever element ended the scene.
fn flush_scene_notes(output: &mut String, buckets: &HashMap<usize, SceneBucket>, scene_idx: usize) {
    if let Some(bucket) = buckets.get(&scene_idx) {
        for note in &bucket.notes {
            output.push_str(&format!("\n[[{}]]\n", note));
        }
    }
}

// ─── Forcing rules ───────────────────────────────────────────────────────────

/// Wrap a scene-heading text with a leading `.` if it wouldn't auto-detect.
/// Malayalam slugs (`അകത്ത്. വീട് - രാത്രി`) and any other heading that
/// doesn't begin with a known prefix needs the forced form, otherwise the
/// next reader treats it as action.
fn force_scene_heading(text: &str) -> String {
    if is_auto_detected_heading(text) {
        text.to_string()
    } else {
        format!(".{}", text)
    }
}

/// Wrap an action line with a leading `!` if it would auto-detect as a
/// different element. The three traps (per Fountain spec):
///   1. all-caps Latin (would become a Character cue)
///   2. all-caps Latin ending `TO:` (would become a Transition)
///   3. begins with `INT/EXT/EST/I/E` slug (would become a Scene Heading)
///
/// Multi-line action emits joined with spaces by the importer, so the
/// check applies to whatever line we're about to write.
fn force_action(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return text.to_string();
    }
    let needs_force = is_all_ascii_upper(trimmed) || is_auto_detected_heading(trimmed);
    if needs_force {
        format!("!{}", text)
    } else {
        text.to_string()
    }
}

/// Wrap a character cue with `@` if it wouldn't pass Fountain's all-caps
/// auto-detection. Caseless scripts (Malayalam, Tamil, Devanagari, etc.)
/// always need the prefix because they have no concept of case; mixed-
/// case Latin (e.g. `iPhone Voice`) also needs it. (#48 / #188)
fn force_character(text: &str) -> String {
    if is_all_ascii_upper(text) {
        text.to_string()
    } else {
        format!("@{}", text)
    }
}

/// Wrap a transition with `>` if it wouldn't auto-detect. The auto rule
/// is "all-caps Latin ending in `TO:`" — anything else (Malayalam, mixed
/// case, missing the colon) needs the forced prefix.
fn force_transition(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.to_uppercase().ends_with("TO:") && is_all_ascii_upper(trimmed) {
        trimmed.to_string()
    } else {
        format!(">{}", trimmed)
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Extract the text content from a ProseMirror node, preserving inline
/// marks as Fountain emphasis: `**bold**`, `*italic*`, `_underline_`.
/// When marks combine, they nest in the order bold → italic → underline,
/// matching what Highland and Fountain.js emit so a round-trip through
/// either parser doesn't reorder them.
fn extract_text(node: &Value) -> String {
    let mut text = String::new();

    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            if let Some(t) = child.get("text").and_then(|t| t.as_str()) {
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

                if is_bold {
                    text.push_str("**");
                }
                if is_italic {
                    text.push('*');
                }
                if is_underline {
                    text.push('_');
                }
                text.push_str(t);
                if is_underline {
                    text.push('_');
                }
                if is_italic {
                    text.push('*');
                }
                if is_bold {
                    text.push_str("**");
                }
            }
        }
    }

    text
}

/// True when the heading text would auto-detect as a Scene Heading per
/// the Fountain spec. Recognised prefixes: `INT`, `EXT`, `EST`,
/// `INT./EXT`, `INT/EXT`, `I/E` (each may be followed by a `.` or space).
fn is_auto_detected_heading(text: &str) -> bool {
    let upper = text.to_uppercase();
    upper.starts_with("INT.")
        || upper.starts_with("INT ")
        || upper.starts_with("EXT.")
        || upper.starts_with("EXT ")
        || upper.starts_with("INT./EXT.")
        || upper.starts_with("INT/EXT")
        || upper.starts_with("EST.")
        || upper.starts_with("EST ")
        || upper.starts_with("I/E.")
        || upper.starts_with("I/E ")
}

/// True when every character in the text is ASCII upper-case, digit, or
/// punctuation/whitespace, AND there's at least one ASCII letter. Used by
/// every forcing rule that has to decide "would this line satisfy
/// Fountain's all-caps Latin check?". Caseless scripts and mixed-case
/// Latin both fail this test (returning false), which is exactly what
/// drives the `@` / `!` / `>` prefix decisions. Empty input → false.
fn is_all_ascii_upper(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let mut has_ascii_letter = false;
    for c in text.chars() {
        if !c.is_ascii() {
            return false;
        }
        if c.is_ascii_lowercase() {
            return false;
        }
        if c.is_ascii_uppercase() {
            has_ascii_letter = true;
        }
    }
    has_ascii_letter
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screenplay::document::ScreenplayMeta;
    use crate::screenplay::fountain_import::parse_fountain;
    use serde_json::json;

    // ─── Title page ──────────────────────────────────────────────────

    #[test]
    fn empty_document_emits_empty_string() {
        let content = json!({"type": "doc", "content": []});
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert_eq!(result, "");
    }

    #[test]
    fn standard_title_page_keys_emit() {
        let content = json!({"type": "doc", "content": []});
        let meta = ScreenplayMeta {
            title: "My Script".to_string(),
            author: "Hrishi".to_string(),
            draft_date: "2026-03-14".to_string(),
            contact: "hello@example.com".to_string(),
            ..Default::default()
        };
        let result = generate_fountain(&content, &meta, &[]);
        assert!(result.contains("Title: My Script"));
        assert!(result.contains("Author: Hrishi"));
        assert!(result.contains("Draft date: 2026-03-14"));
        assert!(result.contains("Contact: hello@example.com"));
    }

    #[test]
    fn meta_extra_keys_emit_in_sorted_order() {
        let content = json!({"type": "doc", "content": []});
        let mut meta = ScreenplayMeta {
            title: "T".to_string(),
            ..Default::default()
        };
        meta.extra.insert("Zeta".into(), "z".into());
        meta.extra
            .insert("Source".into(), "Based on a true story".into());
        meta.extra.insert("Beta".into(), "b".into());
        let result = generate_fountain(&content, &meta, &[]);
        let beta = result.find("Beta:").expect("Beta present");
        let source = result.find("Source:").expect("Source present");
        let zeta = result.find("Zeta:").expect("Zeta present");
        assert!(
            beta < source && source < zeta,
            "expected sorted order, got: {result}"
        );
        assert!(result.contains("Source: Based on a true story"));
    }

    #[test]
    fn registration_number_emits_as_copyright() {
        let content = json!({"type": "doc", "content": []});
        let meta = ScreenplayMeta {
            title: "T".to_string(),
            registration_number: "© 2026 Stultus".to_string(),
            ..Default::default()
        };
        let result = generate_fountain(&content, &meta, &[]);
        assert!(result.contains("Copyright: © 2026 Stultus"));
    }

    #[test]
    fn footnote_emits_as_notes() {
        let content = json!({"type": "doc", "content": []});
        let meta = ScreenplayMeta {
            title: "T".to_string(),
            footnote: "Confidential. Do not distribute.".to_string(),
            ..Default::default()
        };
        let result = generate_fountain(&content, &meta, &[]);
        assert!(result.contains("Notes: Confidential. Do not distribute."));
    }

    // ─── Scene headings ──────────────────────────────────────────────

    #[test]
    fn scene_heading_auto_detect_emits_unforced() {
        let content = scene("INT. COFFEE SHOP - DAY");
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert_eq!(result.trim(), "INT. COFFEE SHOP - DAY");
    }

    #[test]
    fn scene_heading_unrecognised_prefix_force_with_period() {
        let content = scene("THE BEACH");
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert_eq!(result.trim(), ".THE BEACH");
    }

    #[test]
    fn malayalam_scene_heading_force_with_period() {
        // Caseless script — can't satisfy Fountain's slug-prefix rule.
        let content = scene("അകത്ത്. വീട് - രാത്രി");
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert!(
            result.contains(".അകത്ത്. വീട് - രാത്രി"),
            "expected forced prefix, got: {result}"
        );
    }

    // ─── Action ──────────────────────────────────────────────────────

    #[test]
    fn all_caps_action_force_with_bang() {
        // Without the `!` prefix, the next reader's parser would mistake
        // `BANG!` for a character cue if a non-empty line follows.
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "scene_heading", "content": [{ "type": "text", "text": "INT. HOUSE - DAY" }] },
                { "type": "action", "content": [{ "type": "text", "text": "BANG!" }] },
            ],
        });
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert!(
            result.contains("!BANG!"),
            "expected forced action, got: {result}"
        );
    }

    #[test]
    fn action_ending_to_force_with_bang() {
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "scene_heading", "content": [{ "type": "text", "text": "INT. HOUSE - DAY" }] },
                { "type": "action", "content": [{ "type": "text", "text": "HE WALKS UP TO:" }] },
            ],
        });
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert!(result.contains("!HE WALKS UP TO:"));
    }

    #[test]
    fn action_with_int_prefix_force_with_bang() {
        // An action paragraph starting with `INT` but not actually a
        // slug (e.g. talking about an "INTERVIEW") would auto-detect as
        // a scene heading without the `!` prefix.
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "scene_heading", "content": [{ "type": "text", "text": "INT. HOUSE - DAY" }] },
                { "type": "action", "content": [{ "type": "text", "text": "INT. AS INTERIORS" }] },
            ],
        });
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        // The action line is all-caps Latin, would also trigger the
        // is_all_ascii_upper branch — either path puts a `!` on it.
        assert!(result.contains("!INT. AS INTERIORS"));
    }

    #[test]
    fn mixed_case_action_emits_unforced() {
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "scene_heading", "content": [{ "type": "text", "text": "INT. HOUSE - DAY" }] },
                { "type": "action", "content": [{ "type": "text", "text": "John walks in." }] },
            ],
        });
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert!(result.contains("\nJohn walks in.\n"));
        assert!(!result.contains("!John"));
    }

    // ─── Character / dialogue / transition ───────────────────────────

    #[test]
    fn ascii_uppercase_character_no_force() {
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "character", "content": [{ "type": "text", "text": "JOHN" }] },
                { "type": "dialogue", "content": [{ "type": "text", "text": "Hello." }] },
            ],
        });
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert!(result.contains("JOHN\nHello."));
        assert!(!result.contains("@JOHN"));
    }

    #[test]
    fn malayalam_character_force_with_at() {
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "character", "content": [{ "type": "text", "text": "രമേശ്" }] },
                { "type": "dialogue", "content": [{ "type": "text", "text": "നമസ്കാരം" }] },
            ],
        });
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert!(result.contains("@രമേശ്"));
    }

    #[test]
    fn mixed_case_character_force_with_at() {
        // `iPhone Voice` has lowercase Latin → would parse as action
        // without the @ prefix.
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "character", "content": [{ "type": "text", "text": "iPhone Voice" }] },
                { "type": "dialogue", "content": [{ "type": "text", "text": "Battery low." }] },
            ],
        });
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert!(result.contains("@iPhone Voice"));
    }

    #[test]
    fn malayalam_transition_force_with_gt() {
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "transition", "content": [{ "type": "text", "text": "മായം" }] },
            ],
        });
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert!(result.contains(">മായം"));
    }

    #[test]
    fn ascii_transition_no_force() {
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "transition", "content": [{ "type": "text", "text": "CUT TO:" }] },
            ],
        });
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert_eq!(result.trim(), "CUT TO:");
    }

    #[test]
    fn parenthetical_wraps_in_parens() {
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "character", "content": [{ "type": "text", "text": "MARY" }] },
                { "type": "parenthetical", "content": [{ "type": "text", "text": "softly" }] },
                { "type": "dialogue", "content": [{ "type": "text", "text": "I know." }] },
            ],
        });
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        assert!(result.contains("MARY\n(softly)\nI know."));
    }

    // ─── Scene-card round-trip ───────────────────────────────────────

    #[test]
    fn scene_card_description_emits_as_synopsis() {
        let content = scene("INT. HOUSE - DAY");
        let cards = vec![SceneCard {
            scene_index: 0,
            description: "Hero meets villain".to_string(),
            shoot_notes: String::new(),
            extra_characters: String::new(),
            scheduled_date: String::new(),
            location_group: String::new(),
        }];
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &cards);
        assert!(result.contains("INT. HOUSE - DAY\n= Hero meets villain"));
    }

    #[test]
    fn section_marker_emits_as_pound_heading_before_scene() {
        let content = scene("INT. HOUSE - DAY");
        let cards = vec![SceneCard {
            scene_index: 0,
            description: String::new(),
            shoot_notes: "[[#section depth=1]] Act One".to_string(),
            extra_characters: String::new(),
            scheduled_date: String::new(),
            location_group: String::new(),
        }];
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &cards);
        // The `# Act One` heading must appear before the scene heading.
        let act_idx = result.find("# Act One").expect("Act One emitted");
        let scene_idx = result.find("INT. HOUSE - DAY").expect("scene emitted");
        assert!(act_idx < scene_idx);
    }

    #[test]
    fn section_marker_depth_two_emits_as_double_pound() {
        let content = scene("INT. HOUSE - DAY");
        let cards = vec![SceneCard {
            scene_index: 0,
            description: String::new(),
            shoot_notes: "[[#section depth=2]] Sequence A".to_string(),
            extra_characters: String::new(),
            scheduled_date: String::new(),
            location_group: String::new(),
        }];
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &cards);
        assert!(result.contains("## Sequence A"));
    }

    #[test]
    fn unmarked_shoot_notes_emit_as_inline_notes() {
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "scene_heading", "content": [{ "type": "text", "text": "INT. HOUSE - DAY" }] },
                { "type": "action", "content": [{ "type": "text", "text": "John walks in." }] },
            ],
        });
        let cards = vec![SceneCard {
            scene_index: 0,
            description: String::new(),
            shoot_notes: "check the lamp".to_string(),
            extra_characters: String::new(),
            scheduled_date: String::new(),
            location_group: String::new(),
        }];
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &cards);
        assert!(result.contains("[[check the lamp]]"));
        // The note must come AFTER the action in the same scene.
        let action_idx = result.find("John walks in.").expect("action present");
        let note_idx = result.find("[[check the lamp]]").expect("note present");
        assert!(action_idx < note_idx);
    }

    #[test]
    fn sections_synopses_notes_attach_to_correct_scenes() {
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "scene_heading", "content": [{ "type": "text", "text": "INT. HOUSE - DAY" }] },
                { "type": "action", "content": [{ "type": "text", "text": "Scene one action." }] },
                { "type": "scene_heading", "content": [{ "type": "text", "text": "EXT. PARK - NIGHT" }] },
                { "type": "action", "content": [{ "type": "text", "text": "Scene two action." }] },
            ],
        });
        let cards = vec![
            SceneCard {
                scene_index: 0,
                description: "Scene one synopsis".to_string(),
                shoot_notes: "scene one note".to_string(),
                extra_characters: String::new(),
                scheduled_date: String::new(),
                location_group: String::new(),
            },
            SceneCard {
                scene_index: 1,
                description: "Scene two synopsis".to_string(),
                shoot_notes: "[[#section depth=1]] Act Two".to_string(),
                extra_characters: String::new(),
                scheduled_date: String::new(),
                location_group: String::new(),
            },
        ];
        let result = generate_fountain(&content, &ScreenplayMeta::default(), &cards);

        let s1_synopsis = result.find("= Scene one synopsis").unwrap();
        let s1_note = result.find("[[scene one note]]").unwrap();
        let act_two = result.find("# Act Two").unwrap();
        let scene_two = result.find("EXT. PARK - NIGHT").unwrap();
        let s2_synopsis = result.find("= Scene two synopsis").unwrap();

        // Section attaches to the next scene, so `# Act Two` precedes
        // `EXT. PARK - NIGHT`.
        assert!(act_two < scene_two);
        // Scene 1 note flushes before scene 2's section.
        assert!(s1_synopsis < s1_note && s1_note < act_two);
        // Scene 2 synopsis sits under its own heading.
        assert!(scene_two < s2_synopsis);
    }

    // ─── Round-trip ──────────────────────────────────────────────────

    #[test]
    fn round_trip_meta_extra_preserves_keys() {
        let mut meta = ScreenplayMeta {
            title: "Round Trip".to_string(),
            ..Default::default()
        };
        meta.extra.insert("Source".into(), "An old letter".into());
        meta.extra
            .insert("Custom Key".into(), "carries over".into());

        let content = scene("INT. HOUSE - DAY");
        let exported = generate_fountain(&content, &meta, &[]);

        let (reparsed, _summary) = parse_fountain(&exported).unwrap();
        assert_eq!(reparsed.meta.title, "Round Trip");
        assert_eq!(
            reparsed.meta.extra.get("Source").map(String::as_str),
            Some("An old letter")
        );
        assert_eq!(
            reparsed.meta.extra.get("Custom Key").map(String::as_str),
            Some("carries over")
        );
    }

    #[test]
    fn round_trip_scene_card_synopsis_survives() {
        let content = scene("INT. HOUSE - DAY");
        let cards = vec![SceneCard {
            scene_index: 0,
            description: "Hero wakes up".to_string(),
            shoot_notes: String::new(),
            extra_characters: String::new(),
            scheduled_date: String::new(),
            location_group: String::new(),
        }];

        let exported = generate_fountain(&content, &ScreenplayMeta::default(), &cards);
        let (reparsed, _) = parse_fountain(&exported).unwrap();
        assert_eq!(reparsed.scene_cards.len(), 1);
        assert_eq!(reparsed.scene_cards[0].description, "Hero wakes up");
    }

    #[test]
    fn round_trip_section_marker_survives() {
        let content = scene("INT. HOUSE - DAY");
        let cards = vec![SceneCard {
            scene_index: 0,
            description: String::new(),
            shoot_notes: "[[#section depth=2]] Sequence A".to_string(),
            extra_characters: String::new(),
            scheduled_date: String::new(),
            location_group: String::new(),
        }];

        let exported = generate_fountain(&content, &ScreenplayMeta::default(), &cards);
        let (reparsed, _) = parse_fountain(&exported).unwrap();
        assert_eq!(reparsed.scene_cards.len(), 1);
        assert!(reparsed.scene_cards[0]
            .shoot_notes
            .contains("[[#section depth=2]] Sequence A"));
    }

    #[test]
    fn round_trip_malayalam_character_survives_via_at_prefix() {
        // The whole point of forcing: a Malayalam character cue must
        // re-import as a character, not as action.
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "scene_heading", "content": [{ "type": "text", "text": "INT. HOUSE - DAY" }] },
                { "type": "character", "content": [{ "type": "text", "text": "രമേശ്" }] },
                { "type": "dialogue", "content": [{ "type": "text", "text": "നമസ്കാരം" }] },
            ],
        });
        let exported = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        let (reparsed, _) = parse_fountain(&exported).unwrap();

        let nodes = reparsed
            .content
            .get("content")
            .and_then(|c| c.as_array())
            .unwrap();
        // Find the character node — must exist after round-trip.
        let cue = nodes
            .iter()
            .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("character"))
            .expect("character node round-trips");
        let cue_text = cue
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|n| n.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        assert_eq!(cue_text, "രമേശ്");
    }

    #[test]
    fn round_trip_all_caps_action_survives_via_bang_prefix() {
        // Without the `!` prefix on export, this would re-import as a
        // character cue + dialogue pair.
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "scene_heading", "content": [{ "type": "text", "text": "INT. HOUSE - DAY" }] },
                { "type": "action", "content": [{ "type": "text", "text": "BANG!" }] },
                { "type": "action", "content": [{ "type": "text", "text": "John reacts." }] },
            ],
        });
        let exported = generate_fountain(&content, &ScreenplayMeta::default(), &[]);
        let (reparsed, _) = parse_fountain(&exported).unwrap();

        let nodes = reparsed
            .content
            .get("content")
            .and_then(|c| c.as_array())
            .unwrap();
        // The first non-heading node should still be action with text "BANG!".
        let bang = nodes
            .iter()
            .find(|n| {
                n.get("type").and_then(|t| t.as_str()) == Some("action")
                    && node_text_field(n) == "BANG!"
            })
            .expect("BANG! round-trips as action");
        let _ = bang;
    }

    // ─── Smoke ───────────────────────────────────────────────────────

    #[test]
    fn full_screenplay_smoke() {
        let content = json!({
            "type": "doc",
            "content": [
                { "type": "scene_heading", "content": [{ "type": "text", "text": "INT. OFFICE - DAY" }] },
                { "type": "action", "content": [{ "type": "text", "text": "John walks in." }] },
                { "type": "character", "content": [{ "type": "text", "text": "JOHN" }] },
                { "type": "dialogue", "content": [{ "type": "text", "text": "Good morning." }] },
                { "type": "transition", "content": [{ "type": "text", "text": "CUT TO:" }] },
                { "type": "scene_heading", "content": [{ "type": "text", "text": "EXT. PARK - NIGHT" }] },
            ],
        });
        let meta = ScreenplayMeta {
            title: "Test Script".to_string(),
            author: "Writer".to_string(),
            ..Default::default()
        };
        let result = generate_fountain(&content, &meta, &[]);
        assert!(result.starts_with("Title: Test Script\n"));
        assert!(result.contains("INT. OFFICE - DAY"));
        assert!(result.contains("\nJohn walks in.\n"));
        assert!(result.contains("\nJOHN\nGood morning."));
        assert!(result.contains("CUT TO:"));
        assert!(result.contains("EXT. PARK - NIGHT"));
    }

    // ─── Test helpers ────────────────────────────────────────────────

    fn scene(text: &str) -> Value {
        json!({
            "type": "doc",
            "content": [
                { "type": "scene_heading", "content": [{ "type": "text", "text": text }] },
            ],
        })
    }

    fn node_text_field(node: &Value) -> String {
        node.get("content")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|n| n.get("text").and_then(|t| t.as_str()))
                    .collect::<String>()
            })
            .unwrap_or_default()
    }
}
