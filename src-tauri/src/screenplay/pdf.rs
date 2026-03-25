// Typst-based PDF generation: ProseMirror JSON → Typst markup → PDF bytes in memory
//
// This module handles converting the ProseMirror document JSON into a Typst markup
// string formatted as a Hollywood single-column screenplay. The Typst markup is
// compiled to PDF in memory using the Typst compiler and typst-pdf crate.

use crate::screenplay::document::ScreenplayMeta;
use chrono::Datelike;
use serde_json::Value;
use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

/// Represents font data needed for PDF embedding.
///
/// Contains the raw bytes of regular and bold font weights,
/// which will be passed to the Typst compiler for embedding in the PDF.
pub struct FontData {
    /// Raw bytes of the regular weight font file
    pub regular: &'static [u8],
    /// Raw bytes of the bold weight font file
    pub bold: &'static [u8],
}

/// A single screenplay element extracted from ProseMirror JSON.
///
/// The ProseMirror document is a tree of nodes — each top-level node
/// corresponds to one screenplay element (scene heading, action, etc.).
struct ScreenplayElement {
    /// The type of element (scene_heading, action, character, etc.)
    element_type: String,
    /// The plain text content of the element (no formatting — used for .to_uppercase() etc.)
    text: String,
    /// Typst markup with inline formatting (bold spans wrapped in #strong[...], text escaped).
    /// Used in place of escape_typst(&text) when rendering elements that can have bold text.
    typst_inline: String,
}

/// A line within a character's dialogue block.
/// Parentheticals are stage directions like "(softly)", dialogue is the spoken text.
/// Each variant stores both the plain text and the Typst-formatted inline markup.
enum DialogueLine {
    /// (plain_text, typst_inline)
    Parenthetical(String, String),
    /// (plain_text, typst_inline)
    Dialogue(String, String),
}

/// A grouped screenplay element for page break control.
///
/// Groups ensure related elements stay together on the same page by wrapping
/// them in Typst's `#block(breakable: false)[...]`. This prevents orphaned
/// character names at page bottoms and lone scene headings without content.
enum ScreenplayGroup {
    /// A scene heading grouped with its first action paragraph.
    /// Rendered inside `#block(breakable: false)[...]` to prevent orphaning.
    SceneBlock {
        heading_text: String,
        scene_number: u32,
        /// Typst inline markup of the first action (preserves bold formatting)
        first_action_typst: Option<String>,
    },
    /// A character name grouped with following parentheticals and dialogue.
    /// Rendered inside `#block(breakable: false)[...]` to keep dialogue with its speaker.
    CharacterBlock {
        name: String,
        /// Sequence of parenthetical and dialogue lines following the character name
        lines: Vec<DialogueLine>,
    },
    /// A standalone element that doesn't need grouping (action, transition, etc.)
    Standalone(ScreenplayElement),
}

/// Groups a flat list of screenplay elements into page-break-safe groups.
///
/// This pass runs after `extract_elements()` and before Typst markup generation.
/// It ensures:
/// - Scene headings are grouped with the immediately following action (if any)
/// - Character names are grouped with all following parenthetical/dialogue lines
/// - Everything else remains standalone
///
/// Uses a manual index loop so we can "consume" (skip) elements that get absorbed
/// into a group, preventing them from being processed twice.
fn group_elements(elements: Vec<ScreenplayElement>) -> Vec<ScreenplayGroup> {
    let mut groups: Vec<ScreenplayGroup> = Vec::new();
    // Manual index so we can skip elements that get consumed into groups.
    // A for-each loop wouldn't let us advance past consumed elements.
    let mut i = 0;
    let mut scene_number: u32 = 0;

    // `elements.len()` returns the number of items. We use `while i < len`
    // instead of `for` so we can increment `i` by more than 1 when consuming.
    while i < elements.len() {
        match elements[i].element_type.as_str() {
            "scene_heading" => {
                scene_number += 1;
                let heading_text = elements[i].text.clone();

                // Peek at the next element — if it's an action, consume it
                // into the SceneBlock so they stay on the same page.
                let first_action_typst = if i + 1 < elements.len()
                    && elements[i + 1].element_type == "action"
                {
                    i += 1; // Skip the next element since we're consuming it
                    Some(elements[i].typst_inline.clone())
                } else {
                    None
                };

                groups.push(ScreenplayGroup::SceneBlock {
                    heading_text,
                    scene_number,
                    first_action_typst,
                });
            }
            "character" => {
                let name = elements[i].text.clone();
                let mut lines: Vec<DialogueLine> = Vec::new();

                // Collect only consecutive parenthetical and dialogue elements.
                // These form a single "character block" that must not break across pages.
                //
                // IMPORTANT: We only collect parenthetical and dialogue nodes here.
                // Any other element type (especially action) must NOT be absorbed
                // into the character block — it must remain standalone. Without this
                // explicit check, an action node sandwiched between two dialogue
                // nodes would incorrectly get pulled into the character block.
                while i + 1 < elements.len() {
                    match elements[i + 1].element_type.as_str() {
                        "parenthetical" => {
                            i += 1;
                            lines.push(DialogueLine::Parenthetical(
                                elements[i].text.clone(),
                                elements[i].typst_inline.clone(),
                            ));
                        }
                        "dialogue" => {
                            i += 1;
                            lines.push(DialogueLine::Dialogue(
                                elements[i].text.clone(),
                                elements[i].typst_inline.clone(),
                            ));
                        }
                        // Stop collecting on ANY non-dialogue/non-parenthetical element.
                        // This includes "action", "scene_heading", "transition", etc.
                        // Action elements must never be included in a character block.
                        _ => break,
                    }
                }

                groups.push(ScreenplayGroup::CharacterBlock { name, lines });
            }
            _ => {
                // Everything else (action, transition, unknown types) stays standalone.
                // We need to move the element out of the vector — but since we're
                // iterating by index over an owned Vec, we reconstruct it here.
                groups.push(ScreenplayGroup::Standalone(ScreenplayElement {
                    element_type: elements[i].element_type.clone(),
                    text: elements[i].text.clone(),
                    typst_inline: elements[i].typst_inline.clone(),
                }));
            }
        }

        i += 1;
    }

    groups
}

/// Extracts screenplay elements from ProseMirror JSON document content.
///
/// ProseMirror stores documents as a tree of typed nodes. The top-level
/// "content" array holds one node per screenplay element. Each node has
/// a "type" (like "scene_heading") and its own "content" array of text nodes.
///
/// Example ProseMirror JSON:
/// ```json
/// {
///   "type": "doc",
///   "content": [
///     { "type": "scene_heading", "content": [{ "type": "text", "text": "INT. OFFICE - DAY" }] },
///     { "type": "action", "content": [{ "type": "text", "text": "John walks in." }] }
///   ]
/// }
/// ```
fn extract_elements(content: &Value) -> Vec<ScreenplayElement> {
    // `Vec::new()` creates an empty growable list — we'll push elements into it
    let mut elements = Vec::new();

    // Navigate to the "content" array which holds all top-level nodes.
    // `and_then` chains two Option operations: if `get("content")` returns Some,
    // try `as_array()` on that value. If either step fails, we get None.
    let nodes = match content.get("content").and_then(|c| c.as_array()) {
        Some(arr) => arr,
        None => return elements, // No content array found — return empty list
    };

    for node in nodes {
        // Each node has a "type" field identifying what screenplay element it is.
        // `as_str()` converts a JSON string value to a Rust &str (string slice).
        let element_type = match node.get("type").and_then(|t| t.as_str()) {
            Some(t) => t.to_string(), // Convert &str to owned String so we can store it
            None => continue,         // Skip nodes without a type
        };

        // Extract plain text by concatenating all child text nodes.
        // A node's "content" array may contain multiple text nodes (e.g., when
        // the line has mixed formatting). We join them all into one string.
        let text = match node.get("content").and_then(|c| c.as_array()) {
            Some(children) => {
                // `filter_map` combines filter and map: skip None values, unwrap Some values.
                // `collect::<Vec<&str>>()` gathers the results into a vector of string slices.
                children
                    .iter()
                    .filter_map(|child| child.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<&str>>()
                    .join("") // Concatenate all text fragments into one string
            }
            None => String::new(), // Node with no text content (e.g., empty paragraph)
        };

        // Also extract Typst-formatted inline markup that preserves bold marks.
        // This is used in place of escape_typst(&text) for elements like action
        // and dialogue where inline bold formatting should appear in the PDF.
        let typst_inline = extract_inline_typst(node);

        elements.push(ScreenplayElement { element_type, text, typst_inline });
    }

    elements
}

/// Escapes special Typst characters in text content.
///
/// Typst uses characters like #, *, _, @ as markup syntax.
/// User-written screenplay text may contain these characters, so we need
/// to prefix them with backslashes so Typst treats them as literal text.
fn escape_typst(text: &str) -> String {
    // Each `.replace()` call creates a new String with that character escaped.
    // The chain processes all special characters in sequence.
    text.replace('\\', "\\\\") // Backslash must be escaped first (before we add more backslashes)
        .replace('#', "\\#")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('@', "\\@")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('$', "\\$")
}

/// Extracts inline content from a ProseMirror node and returns Typst markup
/// with bold formatting preserved.
///
/// ProseMirror stores inline formatting as "marks" on text nodes. For example,
/// a node with bold text looks like:
/// ```json
/// { "content": [
///   { "type": "text", "text": "normal " },
///   { "type": "text", "text": "bold", "marks": [{ "type": "bold" }] }
/// ]}
/// ```
///
/// This function walks the content array, escapes each text fragment for Typst,
/// and wraps bold fragments in `#strong[...]`.
fn extract_inline_typst(node: &Value) -> String {
    let children = match node.get("content").and_then(|c| c.as_array()) {
        Some(arr) => arr,
        None => return String::new(),
    };

    let mut result = String::new();
    for child in children {
        let text = match child.get("text").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => continue,
        };
        let escaped = escape_typst(text);

        // Check which marks are applied to this text node.
        // `marks` is an optional array on the text node; each mark has a "type" field.
        let marks_array = child.get("marks").and_then(|m| m.as_array());

        // Helper closure: checks if a specific mark type is present in the marks array
        let has_mark = |mark_type: &str| -> bool {
            marks_array
                .map(|marks| {
                    marks
                        .iter()
                        .any(|mark| mark.get("type").and_then(|t| t.as_str()) == Some(mark_type))
                })
                .unwrap_or(false)
        };

        let is_bold = has_mark("bold");
        let is_italic = has_mark("italic");
        let is_underline = has_mark("underline");

        // Build nested Typst markup for the text fragment.
        // Typst uses #strong[...] for bold, #emph[...] for italic,
        // and #underline[...] for underline. They can be nested.
        let mut formatted = escaped;
        if is_underline {
            formatted = format!("#underline[{}]", formatted);
        }
        if is_italic {
            formatted = format!("#emph[{}]", formatted);
        }
        if is_bold {
            formatted = format!("#strong[{}]", formatted);
        }

        result.push_str(&formatted);
    }

    result
}

/// Formats credit lines from author and director fields.
///
/// Returns a vector of credit lines (e.g. ["Written and Directed by", "Alice"])
/// based on what fields are filled in and whether writer and director are the same person.
///
/// # Rules
/// - Author only → "Written by" / author
/// - Director only → "Directed by" / director
/// - Same person → "Written and Directed by" / name
/// - Different people → "Written by" / author, "Directed by" / director
fn format_credit_lines(author: &str, director: &str) -> Vec<(String, String)> {
    let author = author.trim();
    let director = director.trim();

    // Both empty — no credits
    if author.is_empty() && director.is_empty() {
        return Vec::new();
    }

    // Only author
    if director.is_empty() {
        return vec![("Written by".to_string(), author.to_string())];
    }

    // Only director
    if author.is_empty() {
        return vec![("Directed by".to_string(), director.to_string())];
    }

    // Both present — check if they're the same person (case-insensitive comparison)
    if author.eq_ignore_ascii_case(director) {
        return vec![("Written and Directed by".to_string(), author.to_string())];
    }

    // Different people — separate credit lines
    vec![
        ("Written by".to_string(), author.to_string()),
        ("Directed by".to_string(), director.to_string()),
    ]
}

/// Generates Typst markup for a screenplay title page from document metadata.
///
/// The title page is only generated if `meta.title` is non-empty (after trimming).
/// It places the title and author centered on the page, with contact info and
/// draft details at the bottom left.
///
/// Returns an empty string if the title is blank, so it's safe to always call
/// this and prepend the result — it's a no-op when there's no title.
pub fn generate_title_page_markup(meta: &ScreenplayMeta) -> String {
    // `trim()` removes leading/trailing whitespace. `is_empty()` checks for "".
    // If the title is blank, skip the title page entirely.
    if meta.title.trim().is_empty() {
        return String::new();
    }

    let mut page = String::new();

    // Open a page block with title page margins (wider top/bottom for centering).
    page.push_str("#page(margin: (top: 3cm, bottom: 3cm, left: 3cm, right: 2.5cm))[\n");

    // --- Centered section: title + credit lines ---
    page.push_str("  #align(center)[\n");
    page.push_str("    #v(8cm)\n");
    // Escape the title so any Typst special characters (like # or $) are rendered literally.
    page.push_str(&format!(
        "    #text(size: 24pt, weight: \"bold\")[{}]\n",
        escape_typst(meta.title.trim())
    ));

    // Generate credit lines from author and director fields.
    // Uses format_credit_lines() to handle "Written by", "Directed by",
    // or combined "Written and Directed by" when they're the same person.
    let credits = format_credit_lines(&meta.author, &meta.director);
    if !credits.is_empty() {
        page.push_str("    #v(1cm)\n");
        for (i, (label, name)) in credits.iter().enumerate() {
            if i > 0 {
                page.push_str("    #v(0.6cm)\n");
            }
            // Label ("Written by") — small, light, understated
            page.push_str(&format!(
                "    #text(size: 11pt, fill: luma(100))[{}]\n",
                escape_typst(label)
            ));
            page.push_str("    #v(0.4cm)\n");
            // Name — larger, prominent
            page.push_str(&format!(
                "    #text(size: 16pt)[{}]\n",
                escape_typst(name)
            ));
        }
    }

    page.push_str("  ]\n"); // close #align(center)

    // --- Bottom-left section: contact info + draft line ---
    // Only show if at least one of contact or draft info is present.
    let has_contact = !meta.contact.trim().is_empty();
    let has_draft = meta.draft_number > 0 || !meta.draft_date.trim().is_empty();

    if has_contact || has_draft {
        page.push_str("  #align(left)[\n");
        // `#v(1fr)` pushes this content to the bottom of the page.
        // `1fr` is a Typst "fractional" unit — it expands to fill available space.
        page.push_str("    #v(1fr)\n");

        if has_contact {
            // Split multi-line contact info by newlines and join with Typst line breaks.
            // `\` at the end of a line in Typst creates a line break (like <br> in HTML).
            let contact_lines: Vec<String> = meta
                .contact
                .trim()
                .lines()
                .map(escape_typst)
                .collect();
            page.push_str(&format!(
                "    #text(size: 10pt)[{}]\n",
                contact_lines.join("\\\n")
            ));
        }

        if has_draft {
            page.push_str("    #v(0.3cm)\n");

            // Build the draft line: "Draft N" optionally followed by " — DATE"
            let mut draft_line = String::new();
            if meta.draft_number > 0 {
                draft_line.push_str(&format!("Draft {}", meta.draft_number));
            }
            if !meta.draft_date.trim().is_empty() {
                if !draft_line.is_empty() {
                    // " — " is an em dash separator between draft number and date
                    draft_line.push_str(" \\u{2014} ");
                }
                draft_line.push_str(&escape_typst(meta.draft_date.trim()));
            }
            page.push_str(&format!("    #text(size: 10pt)[{}]\n", draft_line));
        }

        page.push_str("  ]\n"); // close #align(left)
    }

    page.push_str("]\n"); // close #page(...)
    page.push_str("#pagebreak()\n\n");

    page
}

/// Generates a Typst markup string from ProseMirror JSON content.
///
/// This produces a complete Typst document with:
/// - A4 page setup with screenplay margins
/// - Font configuration using the specified font name
/// - All screenplay elements formatted in Hollywood single-column style
///
/// The returned string is valid Typst source that can be compiled to PDF.
pub fn generate_typst_markup(content: &Value, font_name: &str, meta: &ScreenplayMeta, page_break_after_scene: bool) -> String {
    let elements = extract_elements(content);

    // Group elements for page break control — this ensures scene headings
    // stay with their first action, and character names stay with dialogue.
    let groups = group_elements(elements);

    // `String::new()` creates an empty growable string — we'll append markup to it.
    let mut markup = String::new();

    // Document preamble — sets up page size, margins, font, and paragraph spacing.
    // `format!` is like printf — it substitutes {} placeholders with values.
    // The `r#"..."#` syntax is a raw string literal — backslashes and quotes
    // inside it are treated as plain text, which is handy for Typst markup.
    //
    // Note: Typst does not currently support `orphans` and `widows` parameters on
    // `#set par(...)`. These are CSS/TeX concepts that Typst handles differently
    // via its block-level `breakable` parameter. We use `#block(breakable: false)`
    // on grouped elements instead to prevent orphaned/widowed content.
    markup.push_str(&format!(
        r#"// Scriptty — Hollywood single-column screenplay format
// Generated by Scriptty. Do not edit manually.

#set page(paper: "a4", margin: (top: 2.5cm, bottom: 2.5cm, left: 3cm, right: 2.5cm))
#set text(font: "{}", size: 12pt)
#set par(leading: 0.65em)

"#,
        font_name
    ));

    // Prepend a title page if the screenplay has a title.
    // The title page uses its own page margins and layout, inserted after the
    // global page/font setup but before any screenplay content.
    markup.push_str(&generate_title_page_markup(meta));

    // Convert each group to Typst markup.
    // Hollywood format has specific indentation and spacing rules for each element type.
    // Groups are wrapped in `#block(breakable: false)` to prevent page breaks
    // between related elements (e.g., character name and their dialogue).
    for group in &groups {
        // `match` is Rust's pattern matching — like a switch statement but more powerful.
        // Each arm produces formatted Typst markup for that group type.
        let typst_element = match group {
            ScreenplayGroup::SceneBlock {
                heading_text,
                scene_number,
                first_action_typst,
            } => {
                let escaped_heading = escape_typst(heading_text);
                // If page-break-after-scene is enabled, insert a page break before
                // every scene except the first one.
                let mut block = String::new();
                if page_break_after_scene && *scene_number > 1 {
                    block.push_str("#pagebreak()\n");
                }
                // Wrap scene heading + first action in an unbreakable block so the
                // heading never appears orphaned at the bottom of a page.
                block.push_str(&format!(
                    "#block(breakable: false)[\n  #v(1.5em)\n  #text(weight: \"bold\", size: 12pt)[{}. {}]\n  #v(0.5em)\n",
                    scene_number,
                    escaped_heading.to_uppercase()
                ));
                if let Some(action_typst) = first_action_typst {
                    // Use typst_inline to preserve bold formatting in action text
                    block.push_str(&format!("  {}\n", action_typst));
                }
                block.push_str("]\n\n");
                block
            }
            ScreenplayGroup::CharacterBlock { name, lines } => {
                let escaped_name = escape_typst(name);
                // Wrap the entire character + dialogue sequence in an unbreakable block
                // so the character name is never separated from their lines.
                //
                // Hollywood alignment (A4 with 3cm left margin, 2.5cm right margin):
                // - Character cue: 9cm from page left edge → pad(left: 6cm) from text area
                // - Dialogue: 6.5cm–14.5cm from page left → pad(left: 3.5cm, right: 3cm)
                // - Parenthetical: 7.5cm from page left → pad(left: 4.5cm, right: 3.5cm)
                let mut block = format!(
                    "#block(breakable: false)[\n  #v(1em)\n  #pad(left: 6cm)[#text(weight: \"bold\")[{}]]\n",
                    escaped_name.to_uppercase()
                );
                for line in lines {
                    match line {
                        DialogueLine::Parenthetical(text, _typst_inline) => {
                            let escaped = escape_typst(text);
                            // Wrap in parentheses if not already present —
                            // the editor shows them via CSS pseudo-elements, not in content.
                            let display = if escaped.starts_with('(') && escaped.ends_with(')') {
                                escaped.clone()
                            } else {
                                format!("({})", escaped)
                            };
                            // Parentheticals: starts at 7.5cm from page left (4.5cm from text area left)
                            block.push_str(&format!(
                                "  #pad(left: 4.5cm, right: 3.5cm)[#emph[{}]]\n",
                                display
                            ));
                        }
                        DialogueLine::Dialogue(_text, typst_inline) => {
                            // Use typst_inline to preserve bold formatting in dialogue
                            block.push_str(&format!(
                                "  #pad(left: 3.5cm, right: 3cm)[{}]\n",
                                typst_inline
                            ));
                        }
                    }
                }
                block.push_str("]\n\n");
                block
            }
            ScreenplayGroup::Standalone(element) => {
                let escaped = escape_typst(&element.text);
                match element.element_type.as_str() {
                    "action" => {
                        // Action lines: use typst_inline to preserve bold formatting
                        format!("{}\n\n", element.typst_inline)
                    }
                    "transition" => {
                        // Transitions: right-aligned, uppercase (e.g., "CUT TO:")
                        // Bold is not meaningful here since transitions are always uppercase styled
                        format!(
                            "#v(1em)\n#align(right)[{}]\n",
                            escaped.to_uppercase()
                        )
                    }
                    // Fallback: scene_heading and character shouldn't appear as Standalone
                    // (they're consumed by groups), but handle them gracefully just in case.
                    "scene_heading" => {
                        format!(
                            "#v(1.5em)\n#text(weight: \"bold\", size: 12pt)[{}]\n#v(0.5em)\n",
                            escaped.to_uppercase()
                        )
                    }
                    "character" => {
                        // Character cue: 9cm from page left → 6cm pad from text area left
                        format!(
                            "#v(1em)\n#pad(left: 6cm)[#text(weight: \"bold\")[{}]]\n",
                            escaped.to_uppercase()
                        )
                    }
                    "dialogue" => {
                        // Dialogue: use typst_inline to preserve bold formatting
                        format!("#pad(left: 3.5cm, right: 3cm)[{}]\n", element.typst_inline)
                    }
                    "parenthetical" => {
                        // Wrap in parentheses if not already present
                        let display = if escaped.starts_with('(') && escaped.ends_with(')') {
                            escaped.clone()
                        } else {
                            format!("({})", escaped)
                        };
                        // Parenthetical: 7.5cm from page left → pad(left: 4.5cm, right: 3.5cm)
                        format!(
                            "#pad(left: 4.5cm, right: 3.5cm)[#emph[{}]]\n",
                            display
                        )
                    }
                    _ => continue, // Skip unknown node types
                }
            }
        };

        // `push_str` appends a string slice to our growing markup string
        markup.push_str(&typst_element);
    }

    markup
}

/// The Typst "World" provides all resources the compiler needs:
/// source code, fonts, and file access. Our implementation is minimal
/// because we only compile a single in-memory source with bundled fonts.
///
/// The `World` trait is the core interface between our app and the Typst
/// compiler. It tells Typst where to find the source code to compile,
/// what fonts are available, and how to access any external files.
/// Since we only generate PDFs from in-memory markup with bundled fonts,
/// our implementation is intentionally simple — no file system access needed.
struct ScreenplayWorld {
    /// The Typst source code (our generated markup).
    /// `Source` is Typst's representation of a single source file.
    source: Source,

    /// Font metadata index — tells Typst what fonts are available
    /// and their properties (family name, weight, style, etc.).
    /// Wrapped in `LazyHash` because the `World` trait requires it
    /// for efficient caching during compilation.
    font_book: LazyHash<FontBook>,

    /// The actual parsed font objects, indexed by position.
    /// The index here matches the index in `font_book`.
    fonts: Vec<Font>,

    /// Shared standard library instance containing all of Typst's
    /// built-in functions, types, and constants.
    /// Also wrapped in `LazyHash` for caching.
    library: LazyHash<Library>,
}

impl ScreenplayWorld {
    /// Creates a new ScreenplayWorld from Typst markup source and font bytes.
    ///
    /// # Arguments
    ///
    /// * `markup` — The Typst source code string to compile
    /// * `font_bytes` — Slice of raw font file bytes (each entry is one font file)
    ///
    /// # Returns
    ///
    /// * `Ok(ScreenplayWorld)` — Ready for compilation
    /// * `Err(String)` — If no valid fonts could be parsed from the provided bytes
    fn new(markup: String, font_bytes: &[&'static [u8]]) -> Result<Self, String> {
        // Parse all fonts from the raw byte slices.
        // Each byte slice may contain one or more fonts (e.g., a .ttc collection).
        // `Font::iter()` returns an iterator over all fonts in the data.
        // `flat_map` flattens multiple iterators into one — so if we have 2 font files
        // each containing 1 font, we get a single iterator of 2 Font objects.
        let mut fonts = Vec::new();
        for bytes in font_bytes {
            // `Bytes::new()` wraps the static byte slice without copying.
            // The `&'static [u8]` lifetime means the data lives forever (it's
            // compiled into the binary), so no allocation is needed.
            let typst_bytes = Bytes::new(*bytes);
            // `Font::iter()` parses all fonts from the byte buffer.
            for font in Font::iter(typst_bytes) {
                fonts.push(font);
            }
        }

        if fonts.is_empty() {
            return Err("No valid fonts found in the provided font data".to_string());
        }

        // Build the font book (metadata index) from our parsed fonts.
        // `FontBook::from_fonts` scans each Font and extracts metadata like
        // family name, weight, and style into a searchable index.
        // The `iter()` call borrows each font rather than consuming it.
        let font_book = FontBook::from_fonts(fonts.iter());

        // Create the source file with a "fake" FileId (not tied to a real file path).
        // `FileId::new_fake()` generates a unique ID for virtual/in-memory files.
        // `VirtualPath::new()` creates a virtual path — not a real filesystem path,
        // just an identifier Typst uses internally.
        let source = Source::new(FileId::new_fake(VirtualPath::new("/main.typ")), markup);

        Ok(Self {
            source,
            font_book: LazyHash::new(font_book),
            fonts,
            // `Library::builder().build()` creates the standard Typst library
            // containing all built-in functions like #text(), #page(), etc.
            library: LazyHash::new(Library::builder().build()),
        })
    }
}

// The `World` trait tells the Typst compiler how to access everything it needs:
// source files, fonts, and the standard library. Since our screenplay world is
// fully in-memory with no external file dependencies, most methods are simple lookups.
//
// `Send + Sync` are required supertraits — they tell Rust this type is safe to
// share across threads. Our struct qualifies because all its fields are thread-safe.
impl World for ScreenplayWorld {
    /// Returns the Typst standard library (built-in functions, types, etc.).
    /// The `&LazyHash<Library>` return type uses lazy hashing for efficient
    /// memoization during compilation.
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    /// Returns the font metadata book so Typst can look up fonts by name/properties.
    fn book(&self) -> &LazyHash<FontBook> {
        &self.font_book
    }

    /// Returns the FileId of the main source file (the entry point for compilation).
    /// `source.id()` retrieves the FileId we assigned when creating the Source.
    fn main(&self) -> FileId {
        self.source.id()
    }

    /// Looks up a source file by its FileId.
    /// We only have one source file (our generated markup), so we check
    /// if the requested ID matches and return it, or an error otherwise.
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            // `FileError::NotFound` tells Typst this file doesn't exist.
            // We wrap it in `Err` because `FileResult` is `Result<T, FileError>`.
            // `FileError::NotFound` takes a PathBuf indicating what file was missing.
            // We use an empty PathBuf since there's no real path to report.
            Err(typst::diag::FileError::NotFound(std::path::PathBuf::new()))
        }
    }

    /// Looks up a raw file by its FileId (for images, data files, etc.).
    /// Our screenplay world has no external files, so we always return
    /// the source text as bytes if it matches, or NotFound otherwise.
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.source.id() {
            // Convert the source text to bytes for raw file access
            Ok(Bytes::new(self.source.text().as_bytes().to_vec()))
        } else {
            // `FileError::NotFound` takes a PathBuf indicating what file was missing.
            // We use an empty PathBuf since there's no real path to report.
            Err(typst::diag::FileError::NotFound(std::path::PathBuf::new()))
        }
    }

    /// Returns a font by its index in the font book.
    /// The index corresponds to the position in our `fonts` vector.
    /// `get()` returns `Option<&Font>`, and `cloned()` converts the
    /// borrowed `&Font` to an owned `Font` (Font is cheap to clone).
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    /// Returns the current date for Typst's `#datetime.today()` function.
    /// The `offset` parameter is a UTC offset in hours — if None, we use local time.
    /// We use the `chrono` crate for date handling.
    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let now = if let Some(offset_hours) = offset {
            // `FixedOffset::east_opt` creates a timezone with the given offset in seconds.
            // We multiply hours by 3600 to convert to seconds.
            let offset = chrono::FixedOffset::east_opt(offset_hours as i32 * 3600)?;
            chrono::Utc::now().with_timezone(&offset).date_naive()
        } else {
            // Use the local timezone if no offset specified
            chrono::Local::now().date_naive()
        };

        // `Datetime::from_ymd` creates a Typst date from year, month, day.
        // `Datelike` trait (imported from chrono) provides `.year()`, `.month()`, `.day()`.
        Datetime::from_ymd(now.year(), now.month().try_into().ok()?, now.day().try_into().ok()?)
    }
}

/// Generates Typst markup for Indian two-column screenplay format.
///
/// Indian format uses a two-column layout: visuals/action on the left (58%),
/// audio/dialogue on the right (42%). Character names are right-aligned in the
/// left column, and dialogue is left-aligned in the right column. Scene headings
/// span the full width.
///
/// # Arguments
///
/// * `content` — The ProseMirror JSON document content (the `"content"` field)
/// * `font_name` — The human-readable font family name (e.g., "Noto Sans Malayalam")
///
/// # Returns
///
/// A complete Typst markup string ready for compilation to PDF.
pub fn generate_indian_markup(content: &Value, font_name: &str, meta: &ScreenplayMeta, page_break_after_scene: bool) -> String {
    // Reuse the same element extraction as Hollywood format.
    // `extract_elements` parses ProseMirror JSON into a flat list of ScreenplayElements.
    let elements = extract_elements(content);

    let mut markup = String::new();

    // Document preamble — A4 page with narrower margins than Hollywood format.
    // Indian two-column format uses the full page width more efficiently since
    // content is split into two columns rather than centered.
    markup.push_str(&format!(
        r#"// Scriptty — Indian two-column screenplay format
// Generated by Scriptty. Do not edit manually.

#set page(paper: "a4", margin: (top: 2cm, bottom: 2cm, left: 1.5cm, right: 1.5cm))
#set text(font: "{}", size: 11pt)
#set par(leading: 0.6em)

"#,
        font_name
    ));

    // Prepend a title page if the screenplay has a title.
    // Same title page layout as Hollywood format — it appears before the
    // two-column content begins.
    markup.push_str(&generate_title_page_markup(meta));

    // Split elements into scenes. Each scene starts with a `scene_heading` and
    // includes all elements until the next `scene_heading`.
    // `scenes` is a Vec of (heading_text, body_elements) tuples.
    // Elements before the first scene heading are treated as a scene with no heading.
    let mut scenes: Vec<(Option<String>, Vec<&ScreenplayElement>)> = Vec::new();

    for element in &elements {
        if element.element_type == "scene_heading" {
            // Start a new scene with this heading. The body starts empty.
            scenes.push((Some(element.text.clone()), Vec::new()));
        } else if scenes.is_empty() {
            // Elements before the first scene heading — create a "no heading" scene.
            scenes.push((None, vec![element]));
        } else {
            // Add this element to the current (last) scene's body.
            // `last_mut()` returns `Option<&mut T>` — a mutable reference to the last item.
            // `unwrap()` is safe here because we checked `scenes.is_empty()` above.
            scenes.last_mut().unwrap().1.push(element);
        }
    }

    // Track scene numbers for the heading labels.
    let mut scene_number: u32 = 0;

    for (heading, body) in &scenes {
        // --- Scene heading rendering ---
        // We may wrap the heading + first action in an unbreakable block so
        // the heading doesn't get orphaned at the bottom of a page.
        if let Some(heading_text) = heading {
            scene_number += 1;

            // If page-break-after-scene is enabled, insert a page break before
            // every scene except the first one.
            if page_break_after_scene && scene_number > 1 {
                markup.push_str("#pagebreak()\n");
            }

            let escaped_heading = escape_typst(heading_text);

            // Check if the first body element is an action — if so, we'll wrap
            // heading + first action together in an unbreakable block.
            let first_is_action = body
                .first() // `first()` returns `Option<&&ScreenplayElement>` — the first element if any
                .map(|el| el.element_type == "action") // convert to `Option<bool>`
                .unwrap_or(false); // default to false if body is empty

            if first_is_action {
                // Wrap scene heading + first action in `#block(breakable: false)` to
                // prevent a page break between them (no orphaned headings).
                // Use typst_inline to preserve bold formatting in the first action.
                let first_action_typst = &body[0].typst_inline;
                markup.push_str(&format!(
                    "#v(1.5em)\n#block(breakable: false)[\n#text(weight: \"bold\")[{}. {}]\n\n#grid(\n  columns: (50%, 50%),\n  column-gutter: 1em,\n  align(left)[{}],\n  []\n)\n]\n",
                    scene_number,
                    escaped_heading.to_uppercase(),
                    first_action_typst
                ));
            } else {
                // No first action to pair with — just render the heading.
                // `#v(1.5em)` adds vertical space before the heading.
                markup.push_str(&format!(
                    "#v(1.5em)\n#text(weight: \"bold\")[{}. {}]\n\n",
                    scene_number,
                    escaped_heading.to_uppercase()
                ));
            }
        }

        // Determine the starting index for the body loop. If the first element
        // was an action that got consumed by the scene heading block above,
        // skip it (start at index 1). Otherwise start at 0.
        let first_is_action = heading.is_some()
            && body
                .first()
                .map(|el| el.element_type == "action")
                .unwrap_or(false);
        let start_index = if first_is_action { 1 } else { 0 };

        // --- Character block buffering ---
        // Instead of emitting each character/dialogue/parenthetical row immediately,
        // we collect grid rows into this buffer. When we encounter a non-dialogue/
        // non-parenthetical element (or reach the end of the scene), we flush the
        // buffer wrapped in `#block(breakable: false)[...]` with vertical spacing.
        //
        // `Vec<String>` — each entry is one `#grid(...)` call as a string.
        let mut char_block_rows: Vec<String> = Vec::new();

        // Track the pending character name for two-column grid rendering.
        // When we encounter a `character` element, we store its name here and
        // don't render it yet — we wait for the following dialogue or parenthetical
        // to pair them together in a two-column grid row.
        //
        // `Option<String>` means this is either `Some("CHARACTER NAME")` or `None`.
        let mut pending_character: Option<String> = None;

        /// Helper: flush collected character block rows into the markup string,
        /// wrapped in an unbreakable block with vertical spacing before and after.
        /// This is a closure (anonymous function) that captures `markup` and
        /// `char_block_rows` by mutable reference.
        ///
        /// We define it as a macro instead of a closure because Rust doesn't allow
        /// a closure to mutably borrow two fields when both are local variables
        /// in the same scope and we also need to borrow `pending_character`.
        macro_rules! flush_char_block {
            ($markup:expr, $rows:expr) => {
                if !$rows.is_empty() {
                    // Add vertical space before the character block for separation
                    // from preceding action text.
                    $markup.push_str("#v(0.5em)\n");
                    // Wrap all rows in an unbreakable block so the character name
                    // and their dialogue are never split across pages.
                    $markup.push_str("#block(breakable: false)[\n");
                    for row in $rows.iter() {
                        $markup.push_str(row);
                    }
                    $markup.push_str("]\n");
                    // Add vertical space after the character block.
                    $markup.push_str("#v(0.3em)\n");
                    $rows.clear();
                }
            };
        }

        for (i, element) in body.iter().enumerate() {
            // Skip the first element if it was already consumed by the scene heading block.
            if i < start_index {
                continue;
            }

            match element.element_type.as_str() {
                "scene_heading" => {
                    // Skip — already rendered above. This shouldn't happen since
                    // we split by scene_heading, but handle it gracefully.
                    continue;
                }
                "action" => {
                    // Action breaks a character block — flush any buffered rows first.
                    // Also flush any pending character that had no dialogue.
                    if let Some(char_name) = pending_character.take() {
                        // `.take()` moves the value out of the Option, leaving it as `None`.
                        // This is how we "consume" the pending character.
                        let escaped_name = escape_typst(&char_name);
                        char_block_rows.push(format!(
                            "#grid(\n  columns: (50%, 50%),\n  column-gutter: 1em,\n  align(right)[#pad(right: 0.5em)[*{}*]],\n  [#pad(left: 0.5em)[]]\n)\n",
                            escaped_name.to_uppercase()
                        ));
                    }
                    flush_char_block!(markup, char_block_rows);

                    // Action text: rendered in the left column of a grid row,
                    // with an empty right column. This keeps action text aligned
                    // with the two-column layout instead of spanning full width.
                    // Use typst_inline to preserve bold formatting.
                    markup.push_str(&format!(
                        "#grid(\n  columns: (50%, 50%),\n  column-gutter: 1em,\n  align(left)[{}],\n  []\n)\n",
                        element.typst_inline
                    ));
                }
                "character" => {
                    // A new character element starts a new character block.
                    // First, flush any previous pending character (e.g., two character
                    // elements in a row without dialogue).
                    if let Some(char_name) = pending_character.take() {
                        let escaped_name = escape_typst(&char_name);
                        char_block_rows.push(format!(
                            "#grid(\n  columns: (50%, 50%),\n  column-gutter: 1em,\n  align(right)[#pad(right: 0.5em)[*{}*]],\n  [#pad(left: 0.5em)[]]\n)\n",
                            escaped_name.to_uppercase()
                        ));
                    }
                    // Flush the previous character block (if any) before starting a new one.
                    flush_char_block!(markup, char_block_rows);

                    // Store this character name — don't render yet.
                    // We'll pair it with the next dialogue or parenthetical.
                    pending_character = Some(element.text.clone());
                }
                "parenthetical" => {
                    let escaped = escape_typst(&element.text);
                    // Wrap in parentheses if not already wrapped (parens are visual-only in the editor)
                    let display = if escaped.starts_with('(') && escaped.ends_with(')') {
                        escaped.clone()
                    } else {
                        format!("({})", escaped)
                    };
                    if let Some(char_name) = pending_character.take() {
                        // Parenthetical right after a character name:
                        // Left column = character name (right-aligned, bold)
                        // Right column = parenthetical (italic)
                        let escaped_name = escape_typst(&char_name);
                        char_block_rows.push(format!(
                            "#grid(\n  columns: (50%, 50%),\n  column-gutter: 1em,\n  align(right)[#pad(right: 0.5em)[*{}*]],\n  align(left)[#pad(left: 0.5em)[#emph[{}]]]\n)\n",
                            escaped_name.to_uppercase(),
                            display
                        ));
                        // Character is consumed — next dialogue will have empty left column.
                    } else {
                        // Parenthetical without a pending character (e.g., between dialogue lines):
                        // Left column = empty, right column = parenthetical (italic)
                        char_block_rows.push(format!(
                            "#grid(\n  columns: (50%, 50%),\n  column-gutter: 1em,\n  [#pad(right: 0.5em)[]],\n  align(left)[#pad(left: 0.5em)[#emph[{}]]]\n)\n",
                            display
                        ));
                    }
                }
                "dialogue" => {
                    // Use typst_inline to preserve bold formatting in dialogue
                    if let Some(char_name) = pending_character.take() {
                        // Dialogue right after a character name (no parenthetical in between):
                        // Left column = character name (right-aligned, bold)
                        // Right column = dialogue text (left-aligned)
                        let escaped_name = escape_typst(&char_name);
                        char_block_rows.push(format!(
                            "#grid(\n  columns: (50%, 50%),\n  column-gutter: 1em,\n  align(right)[#pad(right: 0.5em)[*{}*]],\n  align(left)[#pad(left: 0.5em)[{}]]\n)\n",
                            escaped_name.to_uppercase(),
                            element.typst_inline
                        ));
                    } else {
                        // Dialogue after parenthetical already consumed the character:
                        // Left column = empty, right column = dialogue text
                        char_block_rows.push(format!(
                            "#grid(\n  columns: (50%, 50%),\n  column-gutter: 1em,\n  [#pad(right: 0.5em)[]],\n  align(left)[#pad(left: 0.5em)[{}]]\n)\n",
                            element.typst_inline
                        ));
                    }
                }
                "transition" => {
                    // Transition breaks a character block — flush any buffered rows.
                    if let Some(char_name) = pending_character.take() {
                        let escaped_name = escape_typst(&char_name);
                        char_block_rows.push(format!(
                            "#grid(\n  columns: (50%, 50%),\n  column-gutter: 1em,\n  align(right)[#pad(right: 0.5em)[*{}*]],\n  [#pad(left: 0.5em)[]]\n)\n",
                            escaped_name.to_uppercase()
                        ));
                    }
                    flush_char_block!(markup, char_block_rows);

                    // Transition: right-aligned, full width (e.g., "CUT TO:")
                    let escaped = escape_typst(&element.text);
                    markup.push_str(&format!(
                        "#align(right)[{}]\n\n",
                        escaped.to_uppercase()
                    ));
                }
                _ => {
                    // Unknown element types are skipped silently.
                    continue;
                }
            }
        }

        // Flush any trailing pending character at the end of a scene.
        // This handles the edge case where a scene ends with just a character name.
        if let Some(char_name) = pending_character.take() {
            let escaped_name = escape_typst(&char_name);
            char_block_rows.push(format!(
                "#grid(\n  columns: (50%, 50%),\n  column-gutter: 1em,\n  align(right)[#pad(right: 0.5em)[*{}*]],\n  [#pad(left: 0.5em)[]]\n)\n",
                escaped_name.to_uppercase()
            ));
        }
        // Flush any remaining character block rows at the end of the scene.
        flush_char_block!(markup, char_block_rows);
    }

    markup
}

/// Generates PDF bytes from a screenplay in Indian two-column format.
///
/// Takes the ProseMirror JSON content, a font name, and the font's raw bytes.
/// Generates Indian two-column Typst markup, then compiles it to PDF using
/// the same Typst compilation pipeline as the Hollywood format.
///
/// # Errors
///
/// Returns an error string if:
/// - No valid fonts could be loaded from the provided font data
/// - The Typst markup fails to compile
/// - PDF rendering fails
pub fn generate_pdf_indian(
    content: &Value,
    font_name: &str,
    font_data: &FontData,
    meta: &ScreenplayMeta,
) -> Result<Vec<u8>, String> {
    // Generate Indian two-column Typst markup instead of Hollywood format.
    // `meta` is passed through to include the title page in the PDF.
    // Standalone Indian PDF export doesn't have the page-break-per-scene option — pass false.
    let markup = generate_indian_markup(content, font_name, meta, false);

    // From here, the compilation pipeline is identical to `generate_pdf()`:
    // create a ScreenplayWorld, compile the Typst source, render to PDF bytes.

    // Collect font bytes — both regular and bold weights for embedding.
    let font_bytes: Vec<&'static [u8]> = vec![font_data.regular, font_data.bold];

    // Create the Typst "World" — provides source code, fonts, and library to the compiler.
    let world = ScreenplayWorld::new(markup, &font_bytes)
        .map_err(|e| format!("Failed to initialize Typst world: {}", e))?;

    // Compile the Typst source into a paged document layout.
    // `::<PagedDocument>` is a "turbofish" — it tells the compiler which output
    // type we want (a page-based document for PDF output).
    let document = typst::compile::<PagedDocument>(&world)
        .output
        .map_err(|diagnostics| {
            let messages: Vec<String> = diagnostics
                .iter()
                .map(|d| format!("{:?}", d))
                .collect();
            format!("Typst compilation errors: {}", messages.join("; "))
        })?;

    // Render the compiled document to PDF bytes in memory.
    // No temp files are written — everything stays in memory.
    let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|diagnostics| {
            let messages: Vec<String> = diagnostics
                .iter()
                .map(|d| format!("{:?}", d))
                .collect();
            format!("PDF rendering errors: {}", messages.join("; "))
        })?;

    Ok(pdf_bytes)
}

/// Generates PDF bytes from a screenplay document.
///
/// Takes the ProseMirror JSON content, a font name, and the font's raw bytes.
/// Generates Typst markup from the document content, then compiles it to PDF
/// using the Typst compiler with the bundled fonts embedded.
///
/// # Errors
///
/// Returns an error string if:
/// - No valid fonts could be loaded from the provided font data
/// - The Typst markup fails to compile (e.g., syntax errors in generated markup)
/// - PDF rendering fails
pub fn generate_pdf(
    content: &Value,
    font_name: &str,
    font_data: &FontData,
    meta: &ScreenplayMeta,
) -> Result<Vec<u8>, String> {
    // Generate the Typst markup from the ProseMirror JSON.
    // `meta` is passed through to include the title page in the PDF.
    // Standalone PDF export doesn't have the page-break-per-scene option — pass false.
    let markup = generate_typst_markup(content, font_name, meta, false);

    // Collect all font bytes — pass both regular and bold weights.
    // These are `&'static [u8]` slices embedded in the binary at compile time.
    let font_bytes: Vec<&'static [u8]> = vec![font_data.regular, font_data.bold];

    // Create the Typst "World" — the environment the compiler needs to do its work.
    let world = ScreenplayWorld::new(markup, &font_bytes)
        .map_err(|e| format!("Failed to initialize Typst world: {}", e))?;

    // Compile the Typst source into a paged document.
    // `typst::compile::<PagedDocument>` tells Typst we want a page-based layout
    // (as opposed to, say, an HTML document). The turbofish `::<PagedDocument>`
    // specifies the generic type parameter explicitly.
    // `Warned { output, warnings }` — we ignore warnings and focus on the result.
    let document = typst::compile::<PagedDocument>(&world)
        .output
        .map_err(|diagnostics| {
            // `diagnostics` is a Vec of errors — format them all into one string
            let messages: Vec<String> = diagnostics
                .iter()
                .map(|d| format!("{:?}", d))
                .collect();
            format!("Typst compilation errors: {}", messages.join("; "))
        })?;

    // Render the compiled document to PDF bytes in memory.
    // `PdfOptions::default()` uses standard PDF settings.
    // No temp files are written — everything stays in memory.
    let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|diagnostics| {
            let messages: Vec<String> = diagnostics
                .iter()
                .map(|d| format!("{:?}", d))
                .collect();
            format!("PDF rendering errors: {}", messages.join("; "))
        })?;

    Ok(pdf_bytes)
}

/// Generates a Typst markup section for a prose text block (synopsis, treatment, or narrative).
///
/// Creates a properly typeset prose section with:
/// - Page break and margin reset to symmetric prose layout
/// - Justified text with first-line paragraph indentation
/// - Comfortable line spacing for reading (1.5× leading)
/// - Project title as main heading, section name as subtitle, author byline
///
/// # Arguments
/// * `section_name` — The section label (e.g. "Synopsis", "Treatment", "Narrative")
/// * `body` — The prose text content
/// * `font_name` — The font family name for rendering
/// * `title` — The project/screenplay title (shown as the main heading)
/// * `author` — Writer name(s)
/// * `director` — Director name
/// * `needs_pagebreak` — whether to emit a `#pagebreak()` before the section
pub fn generate_prose_section_markup(section_name: &str, body: &str, font_name: &str, title: &str, author: &str, director: &str, needs_pagebreak: bool) -> String {
    let escaped_section = escape_typst(section_name);
    let escaped_body = escape_typst(body);
    let escaped_title = escape_typst(title);

    let mut markup = String::new();

    // Page break and reset page/paragraph settings for prose layout.
    // Screenplay uses asymmetric margins (left: 3cm) and tight leading (0.65em);
    // prose needs symmetric margins, justified text, and relaxed leading.
    // Only emit a page break if there's preceding content — avoids a blank first page.
    if needs_pagebreak {
        markup.push_str("#pagebreak()\n\n");
    }

    markup.push_str(&format!(
        r#"// Prose section: reset layout for readable prose typography
#set page(margin: (top: 2.5cm, bottom: 2.5cm, left: 3cm, right: 3cm))
#set text(font: "{}", size: 12pt)
#set par(justify: true, leading: 0.8em, first-line-indent: 1cm)

"#,
        font_name
    ));

    // Project title as the main heading (large, bold)
    if !title.is_empty() {
        markup.push_str(&format!(
            r#"#par(first-line-indent: 0cm)[
  #align(center)[
    #v(2cm)
    #text(size: 20pt, weight: "bold")[{}]
  ]
]
"#,
            escaped_title
        ));
    }

    // Section name as a subtitle (smaller, tracked, muted)
    markup.push_str(&format!(
        r#"#par(first-line-indent: 0cm)[
  #align(center)[
    {}#text(size: 12pt, tracking: 0.15em, fill: luma(100))[{}]
  ]
]
"#,
        // Add spacing: more if title is present, less if it's the first element
        if title.is_empty() { "#v(2cm)\n    " } else { "#v(0.6cm)\n    " },
        escaped_section.to_uppercase()
    ));

    // Credit lines below the section name — compact single-line format.
    // Label is italic and muted, name is normal weight for visual hierarchy.
    let credits = format_credit_lines(author, director);
    for (label, name) in &credits {
        markup.push_str(&format!(
            r#"#par(first-line-indent: 0cm)[
  #align(center)[
    #v(0.3cm)
    #text(size: 11pt)[#text(fill: luma(120))[#emph[{}]] {}]
  ]
]
"#,
            escape_typst(label),
            escape_typst(name)
        ));
    }

    markup.push_str("\n#v(1.5cm)\n\n");

    // Body text — split by newlines to preserve paragraph breaks.
    // Each non-empty line becomes a Typst paragraph. Empty lines add spacing.
    // The first paragraph after the heading should not have first-line indent
    // (standard typographic convention).
    let mut is_first_paragraph = true;
    for paragraph in escaped_body.split('\n') {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            // Blank line = paragraph break with spacing
            markup.push_str("#v(0.4em)\n");
            continue;
        }
        if is_first_paragraph {
            // First paragraph: no indent (typographic convention after headings)
            markup.push_str(&format!("#par(first-line-indent: 0cm)[{}]\n\n", trimmed));
            is_first_paragraph = false;
        } else {
            // Subsequent paragraphs: inherit the 1cm first-line indent from #set par()
            markup.push_str(&format!("{}\n\n", trimmed));
        }
    }

    markup
}

/// Generates a Typst markup section for the scene cards breakdown.
///
/// Creates a formatted table of scene information for set use.
///
/// # Arguments
/// * `cards_data` — JSON array of scene card objects with auto-populated and manual fields
/// * `font_name` — The font family name for rendering
/// * `meta` — Document metadata for title and credits
/// * `needs_pagebreak` — whether to emit a `#pagebreak()` before the section.
pub fn generate_scene_cards_markup(cards_data: &Value, font_name: &str, meta: &ScreenplayMeta, needs_pagebreak: bool) -> String {
    let mut markup = String::new();

    // Only emit a page break if there's preceding content.
    if needs_pagebreak {
        markup.push_str("#pagebreak()\n\n");
    }

    // Reset to symmetric prose-style margins for the scene breakdown section
    markup.push_str(&format!(
        r#"// Scene breakdown: reset layout
#set page(margin: (top: 2.5cm, bottom: 2.5cm, left: 2.5cm, right: 2.5cm))
#set text(font: "{}", size: 11pt)
#set par(justify: false, first-line-indent: 0cm, leading: 0.65em)

"#,
        font_name
    ));

    // Project title as main heading (if present)
    if !meta.title.trim().is_empty() {
        markup.push_str(&format!(
            r#"#par(first-line-indent: 0cm)[
  #align(center)[
    #v(1cm)
    #text(size: 20pt, weight: "bold")[{}]
  ]
]
"#,
            escape_typst(meta.title.trim())
        ));
    }

    // "Scene Breakdown" as section subtitle
    markup.push_str(&format!(
        r#"#par(first-line-indent: 0cm)[
  #align(center)[
    {}#text(size: 12pt, tracking: 0.15em, fill: luma(100))[SCENE BREAKDOWN]
  ]
]
"#,
        if meta.title.trim().is_empty() { "#v(1cm)\n    " } else { "#v(0.6cm)\n    " }
    ));

    // Credit lines — label italic and muted, name normal weight
    let credits = format_credit_lines(&meta.author, &meta.director);
    for (label, name) in &credits {
        markup.push_str(&format!(
            r#"#par(first-line-indent: 0cm)[
  #align(center)[
    #v(0.3cm)
    #text(size: 11pt)[#text(fill: luma(120))[#emph[{}]] {}]
  ]
]
"#,
            escape_typst(label),
            escape_typst(name)
        ));
    }

    markup.push_str("\n#v(1.5cm)\n\n");

    // cards_data is expected to be a JSON array of objects:
    // [{ scene_number, heading, location, time, characters, page_estimate, description, shoot_notes }]
    if let Some(cards) = cards_data.as_array() {
        for card in cards {
            let scene_num = card.get("scene_number").and_then(|v| v.as_u64()).unwrap_or(0);
            let heading = card.get("heading").and_then(|v| v.as_str()).unwrap_or("");
            let location = card.get("location").and_then(|v| v.as_str()).unwrap_or("");
            let time = card.get("time").and_then(|v| v.as_str()).unwrap_or("");
            let characters = card.get("characters").and_then(|v| v.as_str()).unwrap_or("");
            let description = card.get("description").and_then(|v| v.as_str()).unwrap_or("");
            let shoot_notes = card.get("shoot_notes").and_then(|v| v.as_str()).unwrap_or("");

            markup.push_str(&format!(
                r#"#block(stroke: 0.5pt + luma(180), radius: 4pt, inset: 12pt, width: 100%)[
  #text(weight: "bold")[{}. {}]
  #v(4pt)
  *Location:* {} #h(1cm) *Time:* {}
  #linebreak()
  *Characters:* {}
"#,
                scene_num,
                escape_typst(heading),
                escape_typst(location),
                escape_typst(time),
                escape_typst(characters),
            ));

            if !description.is_empty() {
                markup.push_str(&format!(
                    "  #v(4pt)\n  _{}_\n",
                    escape_typst(description)
                ));
            }
            if !shoot_notes.is_empty() {
                markup.push_str(&format!(
                    "  #v(4pt)\n  #text(size: 10pt, fill: luma(100))[Notes: {}]\n",
                    escape_typst(shoot_notes)
                ));
            }

            markup.push_str("]\n#v(8pt)\n\n");
        }
    }

    markup
}

/// Compiles a complete Typst markup string (with preamble already included)
/// into PDF bytes. Shared helper used by the combined export command.
pub fn compile_markup_to_pdf(markup: &str, font_data: &FontData) -> Result<Vec<u8>, String> {
    let font_bytes: Vec<&'static [u8]> = vec![font_data.regular, font_data.bold];
    let world = ScreenplayWorld::new(markup.to_string(), &font_bytes)
        .map_err(|e| format!("Failed to initialize Typst world: {}", e))?;

    let document = typst::compile::<PagedDocument>(&world)
        .output
        .map_err(|diagnostics| {
            let messages: Vec<String> = diagnostics
                .iter()
                .map(|d| format!("{:?}", d))
                .collect();
            format!("Typst compilation errors: {}", messages.join("; "))
        })?;

    let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|diagnostics| {
            let messages: Vec<String> = diagnostics
                .iter()
                .map(|d| format!("{:?}", d))
                .collect();
            format!("PDF rendering errors: {}", messages.join("; "))
        })?;

    Ok(pdf_bytes)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // `use super::*` brings everything from the parent module (pdf.rs) into scope
    // so we can call its functions in our tests.
    use super::*;
    use crate::screenplay::document::ScreenplayMeta;
    use serde_json::json;

    /// Creates a default (empty) ScreenplayMeta for tests that don't need title page data.
    /// Using `ScreenplayMeta::default()` gives us blank fields, so no title page is generated.
    fn empty_meta() -> ScreenplayMeta {
        ScreenplayMeta::default()
    }

    #[test]
    fn test_escape_typst_special_characters() {
        assert_eq!(escape_typst("hello #world"), "hello \\#world");
        assert_eq!(escape_typst("price: $5"), "price: \\$5");
        assert_eq!(escape_typst("a < b > c"), "a \\< b \\> c");
        assert_eq!(escape_typst("no specials"), "no specials");
    }

    #[test]
    fn test_extract_elements_basic() {
        // `json!` macro creates a serde_json::Value from JSON-like syntax
        let doc = json!({
            "type": "doc",
            "content": [
                {
                    "type": "scene_heading",
                    "content": [{ "type": "text", "text": "INT. OFFICE - DAY" }]
                },
                {
                    "type": "action",
                    "content": [{ "type": "text", "text": "John walks in." }]
                }
            ]
        });

        let elements = extract_elements(&doc);
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].element_type, "scene_heading");
        assert_eq!(elements[0].text, "INT. OFFICE - DAY");
        assert_eq!(elements[1].element_type, "action");
        assert_eq!(elements[1].text, "John walks in.");
    }

    #[test]
    fn test_extract_elements_empty_content() {
        let doc = json!({ "type": "doc" });
        let elements = extract_elements(&doc);
        assert_eq!(elements.len(), 0);
    }

    #[test]
    fn test_extract_elements_multi_text_nodes() {
        // ProseMirror may split a line into multiple text nodes (e.g., mixed marks)
        let doc = json!({
            "type": "doc",
            "content": [
                {
                    "type": "action",
                    "content": [
                        { "type": "text", "text": "Hello " },
                        { "type": "text", "text": "world" }
                    ]
                }
            ]
        });

        let elements = extract_elements(&doc);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].text, "Hello world");
    }

    #[test]
    fn test_generate_typst_markup_contains_scene_heading() {
        let doc = json!({
            "type": "doc",
            "content": [
                {
                    "type": "scene_heading",
                    "content": [{ "type": "text", "text": "INT. OFFICE - DAY" }]
                }
            ]
        });

        let markup = generate_typst_markup(&doc, "Noto Sans Malayalam", &empty_meta());
        // Should contain the font setting
        assert!(markup.contains("Noto Sans Malayalam"));
        // Scene heading text should be uppercased
        assert!(markup.contains("INT. OFFICE - DAY"));
        // Should include scene number
        assert!(markup.contains("1. INT. OFFICE - DAY"));
        // Should be bold
        assert!(markup.contains("weight: \"bold\""));
        // Should be wrapped in an unbreakable block for page break control
        assert!(markup.contains("block(breakable: false)"));
    }

    #[test]
    fn test_generate_typst_markup_dialogue_block() {
        let doc = json!({
            "type": "doc",
            "content": [
                {
                    "type": "character",
                    "content": [{ "type": "text", "text": "John" }]
                },
                {
                    "type": "parenthetical",
                    "content": [{ "type": "text", "text": "(softly)" }]
                },
                {
                    "type": "dialogue",
                    "content": [{ "type": "text", "text": "I need to go." }]
                }
            ]
        });

        let markup = generate_typst_markup(&doc, "Manjari", &empty_meta());
        // Character name should be uppercase and left-padded to Hollywood spec position
        assert!(markup.contains("JOHN"));
        assert!(markup.contains("pad(left: 6cm)"));
        // Parenthetical should be italic with correct padding
        assert!(markup.contains("emph"));
        assert!(markup.contains("pad(left: 4.5cm, right: 3.5cm)"));
        // Dialogue should be padded to Hollywood spec
        assert!(markup.contains("pad(left: 3.5cm, right: 3cm)"));
        assert!(markup.contains("I need to go."));
        // Entire character block should be wrapped in an unbreakable block
        assert!(markup.contains("block(breakable: false)"));
    }

    #[test]
    fn test_generate_typst_markup_malayalam_text() {
        let doc = json!({
            "type": "doc",
            "content": [
                {
                    "type": "dialogue",
                    "content": [{ "type": "text", "text": "രമേഷ് Flat ലേക്ക് നടന്നു" }]
                }
            ]
        });

        let markup = generate_typst_markup(&doc, "Noto Sans Malayalam", &empty_meta());
        // Malayalam text should pass through unmodified (no special chars to escape)
        assert!(markup.contains("രമേഷ് Flat ലേക്ക് നടന്നു"));
    }

    // ─── Grouping logic tests ────────────────────────────────────────────────

    #[test]
    fn test_group_elements_scene_heading_with_action() {
        // A scene heading followed by an action should be grouped into a SceneBlock
        let elements = vec![
            ScreenplayElement {
                element_type: "scene_heading".to_string(),
                text: "INT. OFFICE - DAY".to_string(),
            },
            ScreenplayElement {
                element_type: "action".to_string(),
                text: "John walks in.".to_string(),
            },
        ];

        let groups = group_elements(elements);
        assert_eq!(groups.len(), 1);

        // `matches!` is a macro that checks if a value matches a pattern.
        // Returns true/false — useful for checking enum variants without destructuring.
        match &groups[0] {
            ScreenplayGroup::SceneBlock {
                heading_text,
                scene_number,
                first_action,
            } => {
                assert_eq!(heading_text, "INT. OFFICE - DAY");
                assert_eq!(*scene_number, 1);
                assert_eq!(first_action.as_deref(), Some("John walks in."));
            }
            _ => panic!("Expected SceneBlock"),
        }
    }

    #[test]
    fn test_group_elements_scene_heading_without_action() {
        // A scene heading followed by a non-action element should have first_action = None
        let elements = vec![
            ScreenplayElement {
                element_type: "scene_heading".to_string(),
                text: "INT. OFFICE - DAY".to_string(),
            },
            ScreenplayElement {
                element_type: "character".to_string(),
                text: "John".to_string(),
            },
        ];

        let groups = group_elements(elements);
        assert_eq!(groups.len(), 2);

        match &groups[0] {
            ScreenplayGroup::SceneBlock {
                first_action, ..
            } => {
                assert!(first_action.is_none());
            }
            _ => panic!("Expected SceneBlock"),
        }
    }

    #[test]
    fn test_group_elements_character_block() {
        // A character followed by parenthetical and dialogue should be grouped
        let elements = vec![
            ScreenplayElement {
                element_type: "character".to_string(),
                text: "John".to_string(),
            },
            ScreenplayElement {
                element_type: "parenthetical".to_string(),
                text: "(softly)".to_string(),
            },
            ScreenplayElement {
                element_type: "dialogue".to_string(),
                text: "I need to go.".to_string(),
            },
        ];

        let groups = group_elements(elements);
        assert_eq!(groups.len(), 1);

        match &groups[0] {
            ScreenplayGroup::CharacterBlock { name, lines } => {
                assert_eq!(name, "John");
                assert_eq!(lines.len(), 2);
                // Verify the first line is a parenthetical
                match &lines[0] {
                    DialogueLine::Parenthetical(text) => assert_eq!(text, "(softly)"),
                    _ => panic!("Expected Parenthetical"),
                }
                // Verify the second line is dialogue
                match &lines[1] {
                    DialogueLine::Dialogue(text) => assert_eq!(text, "I need to go."),
                    _ => panic!("Expected Dialogue"),
                }
            }
            _ => panic!("Expected CharacterBlock"),
        }
    }

    #[test]
    fn test_group_elements_standalone_action() {
        // An action not preceded by a scene heading should be standalone
        let elements = vec![ScreenplayElement {
            element_type: "action".to_string(),
            text: "The door opens.".to_string(),
        }];

        let groups = group_elements(elements);
        assert_eq!(groups.len(), 1);

        match &groups[0] {
            ScreenplayGroup::Standalone(el) => {
                assert_eq!(el.element_type, "action");
                assert_eq!(el.text, "The door opens.");
            }
            _ => panic!("Expected Standalone"),
        }
    }

    #[test]
    fn test_group_elements_scene_numbering() {
        // Multiple scene headings should be numbered sequentially
        let elements = vec![
            ScreenplayElement {
                element_type: "scene_heading".to_string(),
                text: "INT. OFFICE - DAY".to_string(),
            },
            ScreenplayElement {
                element_type: "action".to_string(),
                text: "First action.".to_string(),
            },
            ScreenplayElement {
                element_type: "scene_heading".to_string(),
                text: "EXT. PARK - NIGHT".to_string(),
            },
            ScreenplayElement {
                element_type: "action".to_string(),
                text: "Second action.".to_string(),
            },
        ];

        let groups = group_elements(elements);
        assert_eq!(groups.len(), 2);

        match &groups[0] {
            ScreenplayGroup::SceneBlock { scene_number, .. } => {
                assert_eq!(*scene_number, 1);
            }
            _ => panic!("Expected SceneBlock"),
        }
        match &groups[1] {
            ScreenplayGroup::SceneBlock { scene_number, .. } => {
                assert_eq!(*scene_number, 2);
            }
            _ => panic!("Expected SceneBlock"),
        }
    }

    #[test]
    fn test_group_elements_character_with_multiple_dialogue_lines() {
        // A character with multiple consecutive dialogue lines should all be grouped
        let elements = vec![
            ScreenplayElement {
                element_type: "character".to_string(),
                text: "Mary".to_string(),
            },
            ScreenplayElement {
                element_type: "dialogue".to_string(),
                text: "First line.".to_string(),
            },
            ScreenplayElement {
                element_type: "parenthetical".to_string(),
                text: "(beat)".to_string(),
            },
            ScreenplayElement {
                element_type: "dialogue".to_string(),
                text: "Second line.".to_string(),
            },
        ];

        let groups = group_elements(elements);
        assert_eq!(groups.len(), 1);

        match &groups[0] {
            ScreenplayGroup::CharacterBlock { name, lines } => {
                assert_eq!(name, "Mary");
                assert_eq!(lines.len(), 3);
            }
            _ => panic!("Expected CharacterBlock"),
        }
    }

    #[test]
    fn test_group_elements_action_breaks_character_block() {
        // An action element between two dialogue blocks must NOT be absorbed
        // into the character block. The character block should end before the
        // action, and the action should be standalone.
        let elements = vec![
            ScreenplayElement {
                element_type: "character".to_string(),
                text: "John".to_string(),
            },
            ScreenplayElement {
                element_type: "dialogue".to_string(),
                text: "First line.".to_string(),
            },
            ScreenplayElement {
                element_type: "action".to_string(),
                text: "He pauses.".to_string(),
            },
            ScreenplayElement {
                element_type: "dialogue".to_string(),
                text: "Second line.".to_string(),
            },
        ];

        let groups = group_elements(elements);
        // Should be 3 groups: CharacterBlock (John + first dialogue),
        // Standalone (action), Standalone (dialogue)
        assert_eq!(groups.len(), 3);

        match &groups[0] {
            ScreenplayGroup::CharacterBlock { name, lines } => {
                assert_eq!(name, "John");
                // Only the first dialogue should be in the block — NOT the action or second dialogue
                assert_eq!(lines.len(), 1);
                match &lines[0] {
                    DialogueLine::Dialogue(text) => assert_eq!(text, "First line."),
                    _ => panic!("Expected Dialogue"),
                }
            }
            _ => panic!("Expected CharacterBlock"),
        }

        // The action should be standalone
        match &groups[1] {
            ScreenplayGroup::Standalone(el) => {
                assert_eq!(el.element_type, "action");
                assert_eq!(el.text, "He pauses.");
            }
            _ => panic!("Expected Standalone action"),
        }

        // The orphaned dialogue should be standalone
        match &groups[2] {
            ScreenplayGroup::Standalone(el) => {
                assert_eq!(el.element_type, "dialogue");
                assert_eq!(el.text, "Second line.");
            }
            _ => panic!("Expected Standalone dialogue"),
        }
    }

    #[test]
    fn test_group_elements_mixed_sequence() {
        // Full screenplay sequence: scene heading + action, then character block,
        // then standalone transition
        let elements = vec![
            ScreenplayElement {
                element_type: "scene_heading".to_string(),
                text: "INT. OFFICE - DAY".to_string(),
            },
            ScreenplayElement {
                element_type: "action".to_string(),
                text: "The room is empty.".to_string(),
            },
            ScreenplayElement {
                element_type: "character".to_string(),
                text: "John".to_string(),
            },
            ScreenplayElement {
                element_type: "dialogue".to_string(),
                text: "Hello.".to_string(),
            },
            ScreenplayElement {
                element_type: "transition".to_string(),
                text: "CUT TO:".to_string(),
            },
        ];

        let groups = group_elements(elements);
        assert_eq!(groups.len(), 3); // SceneBlock, CharacterBlock, Standalone(transition)

        assert!(matches!(&groups[0], ScreenplayGroup::SceneBlock { .. }));
        assert!(matches!(&groups[1], ScreenplayGroup::CharacterBlock { .. }));
        assert!(matches!(&groups[2], ScreenplayGroup::Standalone(_)));
    }

    #[test]
    fn test_scene_block_markup_contains_unbreakable_block() {
        // Verify that a scene heading + action generates Typst with breakable: false
        let doc = json!({
            "type": "doc",
            "content": [
                {
                    "type": "scene_heading",
                    "content": [{ "type": "text", "text": "INT. OFFICE - DAY" }]
                },
                {
                    "type": "action",
                    "content": [{ "type": "text", "text": "John walks in." }]
                }
            ]
        });

        let markup = generate_typst_markup(&doc, "Noto Sans Malayalam", &empty_meta());
        // The scene heading and first action should be inside a single unbreakable block
        assert!(markup.contains("block(breakable: false)"));
        assert!(markup.contains("1. INT. OFFICE - DAY"));
        assert!(markup.contains("John walks in."));
    }

    #[test]
    fn test_scene_heading_only_consumes_first_action() {
        // A scene heading should only consume the FIRST action — subsequent
        // actions should be standalone
        let doc = json!({
            "type": "doc",
            "content": [
                {
                    "type": "scene_heading",
                    "content": [{ "type": "text", "text": "INT. OFFICE - DAY" }]
                },
                {
                    "type": "action",
                    "content": [{ "type": "text", "text": "First action." }]
                },
                {
                    "type": "action",
                    "content": [{ "type": "text", "text": "Second action." }]
                }
            ]
        });

        let elements = extract_elements(&doc);
        let groups = group_elements(elements);
        // Should be: SceneBlock (heading + first action), Standalone (second action)
        assert_eq!(groups.len(), 2);

        match &groups[0] {
            ScreenplayGroup::SceneBlock { first_action, .. } => {
                assert_eq!(first_action.as_deref(), Some("First action."));
            }
            _ => panic!("Expected SceneBlock"),
        }
        match &groups[1] {
            ScreenplayGroup::Standalone(el) => {
                assert_eq!(el.text, "Second action.");
            }
            _ => panic!("Expected Standalone"),
        }
    }
}
