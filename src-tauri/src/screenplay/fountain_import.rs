// Fountain import: .fountain plain text → ProseMirror JSON + meta + scene_cards.
//
// Implements a hand-rolled parser following the Fountain spec at
// https://fountain.io/syntax/ and matching the precedence used by the
// canonical Objective-C reference parser (`nyousefi/Fountain`).
//
// Pipeline (per-stage notes inline below):
//   1. strip_boneyard       — global pass; `/* ... */` is the only Fountain
//                             syntax allowed to cross double line breaks
//   2. extract_notes        — pulls out `[[ ... ]]` (may also span lines)
//                             so line-based tokenisation can't fragment them
//   3. parse_title_page     — top-of-file `Key: value` pairs ending at the
//                             first blank line followed by a non-key line
//   4. tokenize_body        — line-by-line state machine with lookahead
//   5. fold_tokens          — folds tokens into ProseMirror JSON nodes and
//                             a parallel `scene_cards` list, attaching
//                             synopses/sections/notes to the right scenes
//
// Out of scope for v1 (per #186 / umbrella #184):
//   - mapping inline emphasis (`*`/`**`/`***`/`_`) to ProseMirror marks —
//     markers are stripped, content is preserved
//   - lyrics as a dedicated node (folded into action)
//   - custom scene numbers (`#1A#`) round-trip
//   - dual dialogue marking (collapsed to sequential pairs)
//   - boneyard preservation (silently dropped, counted)

use crate::screenplay::document::{
    ProjectType, SceneCard, ScreenplayDocument, ScreenplayMeta, ScreenplaySettings, ScreenplayStory,
};
use serde_json::{json, Value};

/// Counts and warnings reported back to the frontend so the import-summary
/// toast can tell the writer what was lost or transformed during import.
/// Mirrors the user-visible language of the toast — keep field names stable
/// across versions to avoid breaking downstream consumers.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ImportSummary {
    pub boneyards_dropped: usize,
    pub notes_count: usize,
    pub synopses_count: usize,
    pub sections_count: usize,
    pub dual_dialogue_count: usize,
    pub scene_numbers_dropped: usize,
    pub emphasis_stripped: usize,
    /// Free-form warnings — e.g. "file appears to contain Malayalam but no
    /// @-prefixed character cues". Surfaced verbatim by the toast.
    pub warnings: Vec<String>,
}

/// Parses a `.fountain` file string into a complete `ScreenplayDocument`
/// (Film shape) plus an `ImportSummary` describing what was transformed
/// or dropped.
///
/// Returns `Err` only on truly malformed input. Most edge cases (boneyard
/// drops, dual dialogue collapse, etc.) succeed and emit summary entries
/// instead of erroring.
pub fn parse_fountain(input: &str) -> Result<(ScreenplayDocument, ImportSummary), String> {
    let mut summary = ImportSummary::default();

    // Stage 1: boneyard global pre-pass.
    let (no_boneyard, boneyards) = strip_boneyard(input);
    summary.boneyards_dropped = boneyards;

    // Stage 2: notes extraction. Notes may span multiple lines; pulling
    // them out as a side-channel keeps the line-based body tokeniser
    // simple. Each replaced span becomes a `\u{FFFD}note:N\u{FFFD}` marker
    // (using U+FFFD REPLACEMENT CHARACTER as a delimiter writers will
    // never type) so the tokeniser can re-attach notes to their host
    // scene during folding.
    let (no_notes, notes) = extract_notes(&no_boneyard);
    summary.notes_count = notes.len();

    // Stage 3: title page (top of file only).
    let (title_page, body_text) = parse_title_page(&no_notes);
    let mut meta = ScreenplayMeta::default();
    apply_title_page(&title_page, &mut meta);

    // Stage 4: tokenise body lines into a flat token stream.
    let tokens = tokenize_body(body_text, &mut summary);

    // Stage 5: fold tokens into ProseMirror nodes and scene_cards.
    let (content, scene_cards) = fold_tokens(tokens, &notes, &mut summary);

    // Malayalam without @-forced cues is a recoverable but noisy case —
    // surface a warning so the writer sees why their characters all
    // imported as action.
    if has_substantial_malayalam(body_text)
        && !body_text.contains("\n@")
        && !body_text.starts_with('@')
    {
        summary.warnings.push(
            "Fountain file appears to contain Malayalam but no @-prefixed character cues — \
             characters may have imported as action. The sender should re-export with forced \
             character prefixes."
                .to_string(),
        );
    }

    let document = ScreenplayDocument {
        project_type: ProjectType::Film,
        series: None,
        content,
        meta,
        settings: ScreenplaySettings::default(),
        story: ScreenplayStory::default(),
        scene_cards,
    };

    Ok((document, summary))
}

// ─── Stage 1: boneyard ───────────────────────────────────────────────────────

/// Strip every `/* ... */` block (including multi-line ones). Returns the
/// cleaned text plus the count of boneyard blocks removed.
///
/// Boneyard is the only Fountain syntax that crosses double line breaks —
/// the spec is explicit. We have to scan with byte-level discipline here
/// because the markers themselves are unrelated to the line structure we
/// rely on later.
fn strip_boneyard(input: &str) -> (String, usize) {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut count = 0;
    while i < bytes.len() {
        // Lookahead for the opening `/*` — checking two bytes at a time.
        // `/` and `*` are both single-byte ASCII so byte-level matching is
        // safe; for any non-ASCII run we copy through as a UTF-8 slice
        // (casting a `u8` to `char` would mis-decode multi-byte chars and
        // corrupt Malayalam content).
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            if let Some(end) = find_subsequence(&bytes[i + 2..], b"*/") {
                i += 2 + end + 2;
                count += 1;
            } else {
                // Unterminated: drop everything from `/*` to EOF.
                count += 1;
                break;
            }
        } else if bytes[i].is_ascii() {
            out.push(bytes[i] as char);
            i += 1;
        } else {
            // Multi-byte UTF-8 char — find its trailing continuation
            // bytes (`10xx_xxxx`) and copy the whole codepoint as a slice.
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i] & 0b1100_0000) == 0b1000_0000 {
                i += 1;
            }
            out.push_str(&input[start..i]);
        }
    }
    (out, count)
}

/// Locate the first occurrence of `needle` inside `haystack`. Returns the
/// byte offset of the match, or `None`. We use this instead of `str::find`
/// so we can operate on byte slices (the boneyard scanner has already
/// committed to byte-level work).
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

// ─── Stage 2: notes ──────────────────────────────────────────────────────────

/// One inline Fountain note (`[[ ... ]]`). Position is recorded so the
/// folder can attach it to the scene that contains the surrounding text.
#[derive(Debug, Clone)]
struct Note {
    text: String,
}

/// Pull every `[[ ... ]]` out of the input, replacing each occurrence
/// with a sentinel marker the body tokeniser can recognise (`\u{FFFD}n:N\u{FFFD}`).
/// Notes may span lines, but a fully blank line terminates them per spec.
fn extract_notes(input: &str) -> (String, Vec<Note>) {
    let mut out = String::with_capacity(input.len());
    let mut notes: Vec<Note> = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'[' && bytes[i + 1] == b'[' {
            // Find the closing `]]`, but stop at a blank line (spec rule).
            if let Some(rel_end) = find_note_end(&bytes[i + 2..]) {
                let raw = &input[i + 2..i + 2 + rel_end];
                notes.push(Note {
                    text: raw.trim().to_string(),
                });
                // Inline marker — we keep notes in-stream so that we can
                // tell which scene contains them when folding. Format:
                // `\u{FFFD}n:<index>\u{FFFD}`.
                out.push_str(&format!("\u{FFFD}n:{}\u{FFFD}", notes.len() - 1));
                i += 2 + rel_end + 2;
            } else {
                // Unterminated note — emit the `[[` as literal text.
                out.push('[');
                out.push('[');
                i += 2;
            }
        } else {
            // ASCII fast path — fall back to char-by-char for non-ASCII.
            if bytes[i].is_ascii() {
                out.push(bytes[i] as char);
                i += 1;
            } else {
                // Find the next char boundary so the slice we copy is valid UTF-8.
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i] & 0b1100_0000) == 0b1000_0000 {
                    i += 1;
                }
                out.push_str(&input[start..i]);
            }
        }
    }
    (out, notes)
}

/// Find the closing `]]` for a note starting at the current position.
/// Per spec, a fully-blank line (zero whitespace) terminates the note
/// even if `]]` never appears.
fn find_note_end(slice: &[u8]) -> Option<usize> {
    let mut i = 0;
    let mut at_line_start = false;
    while i < slice.len() {
        if i + 1 < slice.len() && slice[i] == b']' && slice[i + 1] == b']' {
            return Some(i);
        }
        if slice[i] == b'\n' {
            // Was the previous char also `\n`? Then this is a blank line
            // (two consecutive newlines = blank in between) — terminate.
            if at_line_start {
                return None;
            }
            at_line_start = true;
        } else if slice[i] == b' ' || slice[i] == b'\t' {
            // Whitespace doesn't reset `at_line_start` — a line of pure
            // whitespace is also "blank" per Fountain's rules. (We use
            // strict spec semantics here even though the surface code
            // path is simple.)
        } else {
            at_line_start = false;
        }
        i += 1;
    }
    None
}

// ─── Stage 3: title page ─────────────────────────────────────────────────────

/// Title page key/value pairs, with original key spelling preserved.
/// Stored as a `Vec` (rather than a map) so duplicate keys in malformed
/// input are still visible to the caller.
type TitlePageEntries = Vec<(String, String)>;

/// Parse the title page off the top of the input. Returns the parsed
/// entries plus the remainder of the file (body content).
///
/// Per spec, the title page is recognised only at the start of the file
/// and ends at the first blank line followed by a non-`Key:` line. If
/// the first non-blank line of the file isn't a `Key: value` form, the
/// title page is empty and the entire input is body.
fn parse_title_page(input: &str) -> (TitlePageEntries, &str) {
    // Quick reject: if the first non-blank line doesn't look like a *known*
    // title-page key, the file has no title page. Without this gate, lines
    // like `FADE IN:` would be misread as a title-page entry with key
    // "FADE IN" and consumed before the body parser sees them.
    let first_meaningful = input.lines().find(|l| !l.trim().is_empty());
    let Some(first) = first_meaningful else {
        return (TitlePageEntries::new(), input);
    };
    if !is_known_title_page_key_line(first) {
        return (TitlePageEntries::new(), input);
    }

    let mut entries: TitlePageEntries = Vec::new();
    let mut current_key: Option<String> = None;
    let mut current_value = String::new();
    let mut consumed_bytes = 0usize;
    let mut byte_cursor = 0usize;

    for line in input.split_inclusive('\n') {
        let line_len = line.len();
        let line_no_newline = line.trim_end_matches('\n').trim_end_matches('\r');

        if line_no_newline.trim().is_empty() {
            // Blank line ends the title page. Consume the blank line itself.
            byte_cursor += line_len;
            consumed_bytes = byte_cursor;
            // Flush any in-progress key.
            if let Some(k) = current_key.take() {
                entries.push((k, std::mem::take(&mut current_value).trim().to_string()));
            }
            break;
        }

        if is_title_page_key_line(line_no_newline) {
            // New key — flush the previous one.
            if let Some(k) = current_key.take() {
                entries.push((k, std::mem::take(&mut current_value).trim().to_string()));
            }
            // Split on the first colon.
            if let Some((key, val)) = line_no_newline.split_once(':') {
                current_key = Some(key.trim().to_string());
                current_value = val.trim().to_string();
            }
        } else if is_indented_continuation(line_no_newline) && current_key.is_some() {
            // Continuation of a multi-line value (3+ leading spaces or tab).
            if !current_value.is_empty() {
                current_value.push('\n');
            }
            current_value.push_str(line_no_newline.trim_start());
        } else {
            // Non-blank, non-key, non-continuation line — title page ends
            // here without consuming this line. The body parser sees it.
            if let Some(k) = current_key.take() {
                entries.push((k, std::mem::take(&mut current_value).trim().to_string()));
            }
            consumed_bytes = byte_cursor;
            break;
        }

        byte_cursor += line_len;
        consumed_bytes = byte_cursor;
    }

    // Flush trailing key if EOF arrived inside the title page.
    if let Some(k) = current_key.take() {
        entries.push((k, current_value.trim().to_string()));
    }

    let body = if consumed_bytes <= input.len() {
        &input[consumed_bytes..]
    } else {
        ""
    };
    (entries, body)
}

/// True when the line looks like a `Key: value` title-page entry — used
/// inside the title-page parsing loop to detect *additional* keys after
/// the first one is confirmed. Permissive about which key names are
/// allowed (any letters/spaces) so non-standard custom keys still
/// round-trip through `meta.extra`.
fn is_title_page_key_line(line: &str) -> bool {
    let Some(colon_idx) = line.find(':') else {
        return false;
    };
    let key = &line[..colon_idx];
    if key.is_empty() || key.len() > 40 {
        return false;
    }
    // The key portion should be predominantly letters with optional spaces.
    // Allowing digits keeps things like `Episode 1:` accidentally working
    // even though that's not standard. We disallow anything that looks
    // remotely like a sentence (contains punctuation other than spaces).
    key.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '\t')
        && key.chars().any(|c| c.is_ascii_alphabetic())
}

/// True when a line is indented enough (3+ spaces or a tab) to count as a
/// continuation of the previous title-page value.
fn is_indented_continuation(line: &str) -> bool {
    line.starts_with('\t') || line.starts_with("   ")
}

/// True when the line begins with a *recognised* title-page key. Used as
/// the gate for entering title-page mode in the first place — if the file
/// opens with `FADE IN:` or any other action-shaped line, we want the
/// body parser to handle it, not the title-page parser. Only after this
/// check passes do we accept additional non-standard keys per spec.
fn is_known_title_page_key_line(line: &str) -> bool {
    let Some(colon_idx) = line.find(':') else {
        return false;
    };
    let key = line[..colon_idx].trim().to_ascii_lowercase();
    matches!(
        key.as_str(),
        "title"
            | "credit"
            | "author"
            | "authors"
            | "source"
            | "notes"
            | "draft date"
            | "date"
            | "contact"
            | "copyright"
            | "revision"
    )
}

/// Map title-page entries onto the document's `meta` struct. Standard keys
/// land in their first-class fields; unrecognised keys round-trip via
/// `meta.extra` so a co-writer's custom title-page metadata is preserved.
fn apply_title_page(entries: &TitlePageEntries, meta: &mut ScreenplayMeta) {
    for (key, value) in entries {
        // Key matching is case-insensitive per the de-facto convention
        // across Highland, Slugline, Beat, and Fountain.js.
        let lower = key.to_ascii_lowercase();
        match lower.as_str() {
            "title" => meta.title = value.clone(),
            "author" | "authors" => meta.author = value.clone(),
            "credit" => {
                // Credit is the prefix phrase ("Written by") on Fountain
                // title pages. Scriptty doesn't model a separate credit
                // field, so when author is empty we fall back to using
                // the credit value directly; otherwise we append (so the
                // data is preserved even if the rendering isn't ideal).
                if meta.author.is_empty() {
                    meta.author = value.clone();
                } else {
                    meta.author.push('\n');
                    meta.author.push_str(value);
                }
            }
            "draft date" | "date" => meta.draft_date = value.clone(),
            "contact" => meta.contact = value.clone(),
            "copyright" => {
                if meta.registration_number.is_empty() {
                    meta.registration_number = value.clone();
                } else {
                    meta.extra.insert(key.clone(), value.clone());
                }
            }
            "notes" => {
                if meta.footnote.is_empty() {
                    meta.footnote = value.clone();
                } else {
                    meta.extra.insert(key.clone(), value.clone());
                }
            }
            _ => {
                // Unknown / non-standard key — preserve verbatim.
                meta.extra.insert(key.clone(), value.clone());
            }
        }
    }
}

// ─── Stage 4: body tokeniser ─────────────────────────────────────────────────

/// One Fountain body element after lexing. The folder turns these into
/// ProseMirror nodes + scene_cards.
#[derive(Debug, Clone)]
enum Token {
    SceneHeading(String),
    Action(String),
    Character(String),
    Parenthetical(String),
    Dialogue(String),
    Transition(String),
    Synopsis(String),
    Section {
        depth: u32,
        text: String,
    },
    /// References a `Note` by index. Inserted at the position the original
    /// `[[ ... ]]` appeared (between body lines) so the folder can attach
    /// it to the right scene.
    NoteAnchor(usize),
}

/// What kind of element we just emitted — drives parenthetical/dialogue
/// context detection on the *next* line.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PrevKind {
    None,
    Character,
    Dialogue,
    Parenthetical,
    Other,
}

fn tokenize_body(input: &str, summary: &mut ImportSummary) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut prev = PrevKind::None;
    let mut at_top_of_body = true;

    // Materialise lines once — we need lookahead to determine character
    // cues (next line non-empty) and scene heading auto-detect (blank
    // before/after).
    let lines: Vec<&str> = input.split('\n').collect();
    let mut i = 0;
    while i < lines.len() {
        let raw = lines[i];
        // Strip trailing CR (CRLF inputs) but keep trailing spaces — the
        // two-space-trailing convention is load-bearing for dialogue
        // continuity.
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed = line.trim();
        let prev_blank = i == 0 || lines[i - 1].trim().is_empty();
        let next_blank = i + 1 >= lines.len() || lines[i + 1].trim().is_empty();

        // Note anchor — emitted as a stand-alone token when the line is
        // *only* a note marker (this happens when the original was a
        // between-line `[[ ... ]]`). Inline notes inside other elements
        // are handled when we strip them from the surrounding text.
        if trimmed.starts_with('\u{FFFD}') && trimmed.ends_with('\u{FFFD}') {
            if let Some(idx) = parse_note_marker(trimmed) {
                tokens.push(Token::NoteAnchor(idx));
                prev = PrevKind::Other;
                i += 1;
                continue;
            }
        }

        if trimmed.is_empty() {
            prev = PrevKind::None;
            i += 1;
            continue;
        }

        // ─── 1. Forced prefixes (Win over auto-detection). ─────────────
        // `..` is NOT a forced scene heading per spec — must be a single
        // dot not followed by another dot.
        if trimmed.starts_with('.') && !trimmed.starts_with("..") && trimmed.len() > 1 {
            let heading = drop_scene_number(trimmed[1..].trim());
            if heading.1 {
                summary.scene_numbers_dropped += 1;
            }
            tokens.push(Token::SceneHeading(strip_emphasis_count(
                heading.0, summary,
            )));
            prev = PrevKind::Other;
            at_top_of_body = false;
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('!') {
            tokens.push(Token::Action(strip_emphasis_count(rest.trim(), summary)));
            prev = PrevKind::Other;
            at_top_of_body = false;
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('@') {
            // Forced character cue — supports Malayalam / mixed-case names.
            // Caret-suffix means dual dialogue (`@MARY ^`). We collapse
            // dual to sequential pairs (per #184 scope) but still count
            // the occurrence so the import-summary toast can report it.
            let (name, dual) = parse_dual_marker(rest.trim());
            if dual {
                summary.dual_dialogue_count += 1;
            }
            tokens.push(Token::Character(strip_emphasis_count(name, summary)));
            prev = PrevKind::Character;
            at_top_of_body = false;
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('~') {
            // Lyrics → action for v1.
            tokens.push(Token::Action(strip_emphasis_count(rest.trim(), summary)));
            prev = PrevKind::Other;
            at_top_of_body = false;
            i += 1;
            continue;
        }
        // Section: `# ...` up to `######`. Count the `#`s.
        if trimmed.starts_with('#') {
            let depth = trimmed.chars().take_while(|c| *c == '#').count() as u32;
            if (1..=6).contains(&depth) {
                let text = trimmed.trim_start_matches('#').trim();
                tokens.push(Token::Section {
                    depth,
                    text: strip_emphasis_count(text, summary),
                });
                summary.sections_count += 1;
                prev = PrevKind::Other;
                at_top_of_body = false;
                i += 1;
                continue;
            }
        }
        // Synopsis: `= text`. NOT a page break (which is `=` × 3+).
        if let Some(rest) = trimmed.strip_prefix('=') {
            // Disambiguate from page break: page break is all `=`s.
            let all_equals = trimmed.chars().all(|c| c == '=');
            if !all_equals {
                tokens.push(Token::Synopsis(strip_emphasis_count(rest.trim(), summary)));
                summary.synopses_count += 1;
                prev = PrevKind::Other;
                at_top_of_body = false;
                i += 1;
                continue;
            }
        }
        // Forced transition: leading `>` (without trailing `<` — that's
        // centred text, handled below).
        if trimmed.starts_with('>') && !trimmed.ends_with('<') {
            let rest = trimmed.trim_start_matches('>').trim();
            tokens.push(Token::Transition(strip_emphasis_count(rest, summary)));
            prev = PrevKind::Other;
            at_top_of_body = false;
            i += 1;
            continue;
        }
        // Centred text → action for v1 (no centred node in our schema).
        if trimmed.starts_with('>') && trimmed.ends_with('<') {
            let inner = trimmed.trim_start_matches('>').trim_end_matches('<').trim();
            tokens.push(Token::Action(strip_emphasis_count(inner, summary)));
            prev = PrevKind::Other;
            at_top_of_body = false;
            i += 1;
            continue;
        }

        // ─── 2. Page break: `=` × 3+, nothing else. ────────────────────
        if trimmed.len() >= 3 && trimmed.chars().all(|c| c == '=') {
            // Dropped — Scriptty has continuous-page editor.
            prev = PrevKind::Other;
            at_top_of_body = false;
            i += 1;
            continue;
        }

        // ─── 3. Scene heading auto-detect. ─────────────────────────────
        // Spec strictly requires a blank line BEFORE and AFTER the slug,
        // but real-world Fountain files routinely place a synopsis (`= ...`)
        // or section (`# ...`) immediately under the heading with no
        // blank in between. Reference parsers (Highland, Slugline,
        // nyousefi/Fountain's FastFountainParser) handle this via
        // paragraph-based scanning. Our line-based scan emulates that
        // tolerance by relying solely on the slug prefix once the
        // previous line is blank — the prefix list is unique enough that
        // false positives are vanishingly rare.
        if prev_blank && is_scene_heading_prefix(trimmed) {
            let (text, dropped_num) = drop_scene_number(trimmed);
            if dropped_num {
                summary.scene_numbers_dropped += 1;
            }
            tokens.push(Token::SceneHeading(strip_emphasis_count(text, summary)));
            prev = PrevKind::Other;
            at_top_of_body = false;
            i += 1;
            continue;
        }

        // ─── 4. Transition auto-detect (incl. `FADE IN:` at top). ──────
        if prev_blank && next_blank && is_uppercase_latin_line(trimmed) && trimmed.ends_with("TO:")
        {
            tokens.push(Token::Transition(strip_emphasis_count(trimmed, summary)));
            prev = PrevKind::Other;
            at_top_of_body = false;
            i += 1;
            continue;
        }
        if at_top_of_body && trimmed.eq_ignore_ascii_case("FADE IN:") {
            tokens.push(Token::Transition(trimmed.to_string()));
            prev = PrevKind::Other;
            at_top_of_body = false;
            i += 1;
            continue;
        }

        // ─── 5. Character cue auto-detect. ─────────────────────────────
        // Requires: blank previous line, non-empty next line (the dialogue).
        if prev_blank && !next_blank && is_character_cue(trimmed) {
            let (name, dual) = parse_dual_marker(trimmed);
            if dual {
                summary.dual_dialogue_count += 1;
            }
            tokens.push(Token::Character(strip_emphasis_count(name, summary)));
            prev = PrevKind::Character;
            at_top_of_body = false;
            i += 1;
            continue;
        }

        // ─── 6. Parenthetical (context: prev was Character/Dialogue). ──
        if matches!(prev, PrevKind::Character | PrevKind::Dialogue)
            && trimmed.starts_with('(')
            && trimmed.ends_with(')')
        {
            tokens.push(Token::Parenthetical(strip_emphasis_count(trimmed, summary)));
            prev = PrevKind::Parenthetical;
            at_top_of_body = false;
            i += 1;
            continue;
        }

        // ─── 7. Dialogue continuation. ─────────────────────────────────
        if matches!(
            prev,
            PrevKind::Character | PrevKind::Parenthetical | PrevKind::Dialogue
        ) {
            // Collect this line plus any consecutive lines that are still
            // dialogue (the two-space-trailing convention keeps blank-
            // looking lines alive).
            let mut combined = strip_emphasis_count(trimmed, summary);
            i += 1;
            while i < lines.len() {
                let raw_next = lines[i].strip_suffix('\r').unwrap_or(lines[i]);
                let next_trimmed = raw_next.trim();
                // Fully blank line (no whitespace) ends the dialogue block.
                if raw_next.is_empty() {
                    break;
                }
                // A line of pure whitespace, including the two-space-trailing
                // convention, keeps the dialogue alive but contributes
                // a paragraph-break-ish signal — we render it as a space.
                if next_trimmed.is_empty() {
                    combined.push(' ');
                    i += 1;
                    continue;
                }
                // Don't cross into another character cue or transition.
                if is_character_cue(next_trimmed) || is_scene_heading_prefix(next_trimmed) {
                    break;
                }
                // Parenthetical mid-dialogue ends this dialogue chunk so
                // the folder can emit a separate parenthetical token.
                if next_trimmed.starts_with('(') && next_trimmed.ends_with(')') {
                    break;
                }
                combined.push(' ');
                combined.push_str(&strip_emphasis_count(next_trimmed, summary));
                i += 1;
            }
            tokens.push(Token::Dialogue(combined));
            prev = PrevKind::Dialogue;
            at_top_of_body = false;
            continue;
        }

        // ─── 8. Action fallback — collect consecutive non-blank lines. ─
        let mut combined = strip_emphasis_count(trimmed, summary);
        i += 1;
        while i < lines.len() {
            let raw_next = lines[i].strip_suffix('\r').unwrap_or(lines[i]);
            let next_trimmed = raw_next.trim();
            if next_trimmed.is_empty() {
                break;
            }
            // Don't absorb a scene heading or character cue into the
            // action block.
            if is_scene_heading_prefix(next_trimmed) {
                break;
            }
            // For mid-action character lookahead we need the line *after*
            // next. If `next` looks like a character cue AND the line
            // after that is non-empty, stop here so the cue is parsed
            // separately. However, since action only ends at a blank
            // line, this case requires a blank line between us and the
            // cue — handled by the `is_empty` check above.
            // Section / synopsis / forced markers also break the run.
            if next_trimmed.starts_with('#')
                || next_trimmed.starts_with('=')
                || next_trimmed.starts_with('!')
                || next_trimmed.starts_with('@')
                || next_trimmed.starts_with('~')
                || (next_trimmed.starts_with('.') && !next_trimmed.starts_with(".."))
            {
                break;
            }
            combined.push(' ');
            combined.push_str(&strip_emphasis_count(next_trimmed, summary));
            i += 1;
        }
        tokens.push(Token::Action(combined));
        prev = PrevKind::Other;
        at_top_of_body = false;
    }

    tokens
}

/// Parse a `\u{FFFD}n:N\u{FFFD}` marker emitted by `extract_notes`. Returns
/// `None` if the marker is malformed (shouldn't happen in practice).
fn parse_note_marker(s: &str) -> Option<usize> {
    let inner = s
        .trim_start_matches('\u{FFFD}')
        .trim_end_matches('\u{FFFD}');
    let idx_str = inner.strip_prefix("n:")?;
    idx_str.parse().ok()
}

/// True when a line begins with a recognised scene-heading slug. The spec
/// lists `INT`, `EXT`, `EST`, `INT./EXT`, `INT/EXT`, and `I/E`. Match is
/// case-insensitive in practice across implementations.
fn is_scene_heading_prefix(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    // Order longest-first so `INT./EXT.` beats `INT.` for a line that
    // genuinely starts with the dual-prefix form.
    const PREFIXES: &[&str] = &[
        "INT./EXT.",
        "INT./EXT ",
        "INT/EXT.",
        "INT/EXT ",
        "INT/EXT",
        "I/E.",
        "I/E ",
        "INT.",
        "INT ",
        "EXT.",
        "EXT ",
        "EST.",
        "EST ",
    ];
    PREFIXES.iter().any(|p| upper.starts_with(p))
}

/// True when the line is uppercase Latin with at least one alphabetical
/// character. Lowercase characters disqualify it; non-ASCII characters
/// also disqualify (Malayalam/Tamil/Devanagari are caseless and can never
/// satisfy this rule — that's why the `@` forced-character prefix exists).
fn is_uppercase_latin_line(line: &str) -> bool {
    let mut has_alpha = false;
    for c in line.chars() {
        if c.is_ascii_lowercase() {
            return false;
        }
        if !c.is_ascii() {
            return false;
        }
        if c.is_ascii_uppercase() {
            has_alpha = true;
        }
    }
    has_alpha
}

/// True when the line could be a character cue. Spec rule: all uppercase
/// Latin letters with at least one alpha. Lowercase is allowed *inside
/// parens only* (extension forms like `MOM (V.O.)` or `HANS (on the
/// radio)`).
fn is_character_cue(line: &str) -> bool {
    // Strip trailing `^` (dual marker) and any whitespace before it.
    let line = line.trim_end_matches('^').trim();
    let mut depth: u32 = 0;
    let mut has_alpha = false;
    for c in line.chars() {
        if c == '(' {
            depth += 1;
            continue;
        }
        if c == ')' {
            depth = depth.saturating_sub(1);
            continue;
        }
        if depth > 0 {
            // Inside parens: anything goes.
            continue;
        }
        if c.is_ascii_lowercase() {
            return false;
        }
        if !c.is_ascii() {
            // Non-ASCII (Malayalam etc.) defeats auto-detection. Writers
            // must use `@` for those — handled at the forced-prefix
            // branch above.
            return false;
        }
        if c.is_ascii_uppercase() {
            has_alpha = true;
        }
    }
    has_alpha
}

/// Extract the `^` dual-dialogue marker from a character cue. Returns the
/// cue name without the marker plus a flag indicating whether the marker
/// was present.
fn parse_dual_marker(line: &str) -> (&str, bool) {
    if line.ends_with('^') {
        (line.trim_end_matches('^').trim_end(), true)
    } else {
        (line, false)
    }
}

/// Strip an optional Fountain scene-number suffix (`#1A#`, `#42#`, etc.).
/// Returns the cleaned heading plus a flag indicating whether a number was
/// dropped (so the caller can update the import summary).
fn drop_scene_number(heading: &str) -> (&str, bool) {
    let trimmed = heading.trim_end();
    // Match `... #ANY#` at end. Find the last `#` and check there's a
    // matching one earlier on the line.
    if !trimmed.ends_with('#') {
        return (heading, false);
    }
    let body = &trimmed[..trimmed.len() - 1];
    let Some(open_idx) = body.rfind('#') else {
        return (heading, false);
    };
    // Ensure there's at least one space before the opening `#` — otherwise
    // we'd eat trailing punctuation that happens to include `#`.
    if open_idx == 0 || !body[..open_idx].ends_with(' ') {
        return (heading, false);
    }
    (body[..open_idx].trim_end(), true)
}

// ─── Emphasis stripping ──────────────────────────────────────────────────────

/// Wrapper that runs `strip_emphasis` and bumps the import-summary counter
/// when any change occurs. We count the *number of lines that had emphasis
/// stripped*, not the number of markers, because that's the actionable
/// signal for the writer (how many places they may want to manually re-add
/// formatting).
fn strip_emphasis_count(text: &str, summary: &mut ImportSummary) -> String {
    let stripped = strip_emphasis(text);
    if stripped != text {
        summary.emphasis_stripped += 1;
    }
    stripped
}

/// Remove Fountain emphasis markers (`*`, `**`, `***`, `_`) from text,
/// honouring backslash escapes (`\*` → `*`, `\_` → `_`, `\\` → `\`).
///
/// Heuristic for "is this marker emphasis or a literal asterisk?":
/// per spec, "spaces around the emphasis characters are meaningful" —
/// meaning the marker has non-whitespace immediately on at least one side
/// when used for emphasis, and whitespace on both sides when literal
/// (e.g. `2 * 3 = 6`). We strip markers whose side neighbours are not
/// both whitespace.
fn strip_emphasis(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            if matches!(next, '*' | '_' | '\\') {
                out.push(next);
                i += 2;
                continue;
            }
        }
        if c == '*' || c == '_' {
            let prev_ws = if i == 0 {
                true
            } else {
                chars[i - 1].is_whitespace()
            };
            let next_ws = if i + 1 >= chars.len() {
                true
            } else {
                chars[i + 1].is_whitespace()
            };
            // Both sides whitespace → treat as literal arithmetic / underscore.
            if prev_ws && next_ws {
                out.push(c);
                i += 1;
                continue;
            }
            // Otherwise — strip this marker (and consecutive same markers
            // for `**` / `***`).
            while i < chars.len() && chars[i] == c {
                i += 1;
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

// ─── Stage 5: token folder ───────────────────────────────────────────────────

/// Fold the token stream into ProseMirror JSON content + scene_cards.
///
/// Bookkeeping:
/// - `scene_index_count` tracks how many `scene_heading` nodes have been
///   emitted (this is what `scene_cards.scene_index` is supposed to point
///   at — see `SceneCard::scene_index` doc).
/// - `pending_section_notes` holds section text we've seen since the last
///   scene heading; flushes onto the next scene's `shoot_notes` when we
///   open a new scene.
/// - `pending_pre_scene_notes` holds inline note anchors that appeared
///   before any scene_heading; we drop these with a warning rather than
///   guess where they belong.
fn fold_tokens(
    tokens: Vec<Token>,
    notes: &[Note],
    summary: &mut ImportSummary,
) -> (Value, Vec<SceneCard>) {
    let mut nodes: Vec<Value> = Vec::new();
    let mut cards: Vec<SceneCard> = Vec::new();
    let mut scene_index_count: usize = 0;
    // Section texts queued for the *next* scene's shoot_notes. Each entry
    // already carries its `[[#section depth=N]] ` marker so the export
    // round-trip can re-emit the section.
    let mut pending_section_notes: Vec<String> = Vec::new();
    let mut have_first_scene = false;

    /// Append a line to the active scene's shoot_notes, creating the card
    /// lazily if needed. Helper closure-style via inner fn — Rust closures
    /// can't easily borrow `cards` mutably across calls.
    fn append_to_card_notes(cards: &mut Vec<SceneCard>, scene_idx: usize, line: &str) {
        // Find or create the card for this scene_index.
        let pos = cards.iter().position(|c| c.scene_index == scene_idx);
        let card_ref = match pos {
            Some(p) => &mut cards[p],
            None => {
                cards.push(SceneCard {
                    scene_index: scene_idx,
                    description: String::new(),
                    shoot_notes: String::new(),
                    extra_characters: String::new(),
                    scheduled_date: String::new(),
                    location_group: String::new(),
                });
                cards.last_mut().unwrap()
            }
        };
        if !card_ref.shoot_notes.is_empty() {
            card_ref.shoot_notes.push('\n');
        }
        card_ref.shoot_notes.push_str(line);
    }

    fn append_to_card_description(cards: &mut Vec<SceneCard>, scene_idx: usize, line: &str) {
        let pos = cards.iter().position(|c| c.scene_index == scene_idx);
        let card_ref = match pos {
            Some(p) => &mut cards[p],
            None => {
                cards.push(SceneCard {
                    scene_index: scene_idx,
                    description: String::new(),
                    shoot_notes: String::new(),
                    extra_characters: String::new(),
                    scheduled_date: String::new(),
                    location_group: String::new(),
                });
                cards.last_mut().unwrap()
            }
        };
        if !card_ref.description.is_empty() {
            card_ref.description.push('\n');
        }
        card_ref.description.push_str(line);
    }

    for tok in tokens {
        match tok {
            Token::SceneHeading(text) => {
                nodes.push(text_node("scene_heading", &text));
                let new_idx = scene_index_count;
                scene_index_count += 1;
                have_first_scene = true;
                // Flush any sections we collected before this scene onto
                // its shoot_notes.
                for marked in pending_section_notes.drain(..) {
                    append_to_card_notes(&mut cards, new_idx, &marked);
                }
            }
            Token::Action(text) => {
                if !text.is_empty() {
                    nodes.push(text_node("action", &text));
                }
            }
            Token::Character(name) => {
                // Dual flag isn't carried on the token — we counted it at
                // tokenisation time and collapse to sequential pairs per
                // the locked scope (#184).
                nodes.push(text_node("character", &name));
            }
            Token::Parenthetical(text) => {
                nodes.push(text_node("parenthetical", &text));
            }
            Token::Dialogue(text) => {
                nodes.push(text_node("dialogue", &text));
            }
            Token::Transition(text) => {
                nodes.push(text_node("transition", &text));
            }
            Token::Synopsis(text) => {
                if have_first_scene {
                    let active = scene_index_count - 1;
                    append_to_card_description(&mut cards, active, &text);
                } else {
                    summary.warnings.push(format!(
                        "Synopsis '{}' appeared before any scene heading and was dropped.",
                        truncate_for_warning(&text)
                    ));
                }
            }
            Token::Section { depth, text } => {
                // Stash with marker for round-trip. The marker syntax is
                // documented in SCREENPLAY_FORMAT.md (issue #188).
                pending_section_notes.push(format!("[[#section depth={}]] {}", depth, text));
            }
            Token::NoteAnchor(idx) => {
                if let Some(note) = notes.get(idx) {
                    if have_first_scene {
                        let active = scene_index_count - 1;
                        append_to_card_notes(&mut cards, active, &note.text);
                    } else {
                        summary.warnings.push(format!(
                            "Note '{}' appeared before any scene heading and was dropped.",
                            truncate_for_warning(&note.text)
                        ));
                    }
                }
            }
        }
    }

    // Any trailing pending sections (sections after the last scene) attach
    // to the last scene's shoot_notes — better to land them somewhere
    // visible than to silently drop.
    if !pending_section_notes.is_empty() && have_first_scene {
        let active = scene_index_count - 1;
        for marked in pending_section_notes.drain(..) {
            append_to_card_notes(&mut cards, active, &marked);
        }
    } else if !pending_section_notes.is_empty() {
        for marked in pending_section_notes {
            summary.warnings.push(format!(
                "Section '{}' had no scene to attach to and was dropped.",
                truncate_for_warning(&marked)
            ));
        }
    }

    // Empty content guardrail — ProseMirror's `doc` requires at least one
    // child. If the file produced no nodes, drop in an empty scene heading
    // so the editor opens the file without erroring.
    if nodes.is_empty() {
        nodes.push(json!({ "type": "scene_heading" }));
    }

    let content = json!({ "type": "doc", "content": nodes });
    (content, cards)
}

/// Build a ProseMirror node with a single text child. Empty strings produce
/// a node with no child content (which the editor renders as an empty
/// paragraph of that type — what we want for blank scene headings).
fn text_node(kind: &str, text: &str) -> Value {
    if text.is_empty() {
        json!({ "type": kind })
    } else {
        json!({
            "type": kind,
            "content": [{ "type": "text", "text": text }]
        })
    }
}

/// Trim a warning excerpt so a long synopsis or note doesn't blow out the
/// summary toast.
fn truncate_for_warning(s: &str) -> String {
    const MAX: usize = 60;
    if s.chars().count() <= MAX {
        s.to_string()
    } else {
        let cutoff: String = s.chars().take(MAX).collect();
        format!("{cutoff}…")
    }
}

/// Cheap heuristic — does this body contain a meaningful amount of
/// Malayalam? We use it to surface the "no @-prefixed cues" warning only
/// when there's something to worry about.
fn has_substantial_malayalam(text: &str) -> bool {
    let mlym = text
        .chars()
        .filter(|c| {
            // Malayalam Unicode block: U+0D00 – U+0D7F.
            let code = *c as u32;
            (0x0D00..=0x0D7F).contains(&code)
        })
        .count();
    mlym >= 20
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn first_node_type(content: &Value) -> &str {
        content
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|n| n.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("<missing>")
    }

    fn node_text(content: &Value, idx: usize) -> String {
        let node = &content.get("content").and_then(|c| c.as_array()).unwrap()[idx];
        node.get("content")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|n| n.get("text").and_then(|t| t.as_str()))
                    .collect::<String>()
            })
            .unwrap_or_default()
    }

    fn node_type(content: &Value, idx: usize) -> String {
        content
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.get(idx))
            .and_then(|n| n.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("<missing>")
            .to_string()
    }

    fn node_count(content: &Value) -> usize {
        content
            .get("content")
            .and_then(|c| c.as_array())
            .map(|a| a.len())
            .unwrap_or(0)
    }

    // ─── Title page ──────────────────────────────────────────────────

    #[test]
    fn parses_title_page_standard_keys() {
        let input = "Title: Monsoon Wedding\n\
                     Author: Sabrina Dhawan\n\
                     Draft date: 2026-03-14\n\
                     Contact: sabrina@example.com\n\
                     \n\
                     INT. HOUSE - DAY\n";
        let (doc, _summary) = parse_fountain(input).unwrap();
        assert_eq!(doc.meta.title, "Monsoon Wedding");
        assert_eq!(doc.meta.author, "Sabrina Dhawan");
        assert_eq!(doc.meta.draft_date, "2026-03-14");
        assert_eq!(doc.meta.contact, "sabrina@example.com");
    }

    #[test]
    fn title_page_keys_are_case_insensitive() {
        let input = "title: Test\nAUTHOR: Hrishi\n\nINT. HOUSE - DAY\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(doc.meta.title, "Test");
        assert_eq!(doc.meta.author, "Hrishi");
    }

    #[test]
    fn title_page_authors_alias_works() {
        let input = "Title: Two\nAuthors: Joel & Ethan Coen\n\nINT. HOUSE - DAY\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(doc.meta.author, "Joel & Ethan Coen");
    }

    #[test]
    fn title_page_unknown_keys_go_into_extra() {
        let input = "Title: T\nSource: Based on a true story\nFoo: Bar\n\nINT. HOUSE - DAY\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(
            doc.meta.extra.get("Source").map(String::as_str),
            Some("Based on a true story")
        );
        assert_eq!(doc.meta.extra.get("Foo").map(String::as_str), Some("Bar"));
    }

    #[test]
    fn title_page_multi_line_value() {
        let input = "Title: T\n\
                     Contact: hello@example.com\n   555-1234\n   PO Box 99\n\
                     \n\
                     INT. HOUSE - DAY\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(doc.meta.contact, "hello@example.com\n555-1234\nPO Box 99");
    }

    #[test]
    fn no_title_page_when_first_line_is_body() {
        let input = "INT. HOUSE - DAY\n\nJohn enters.\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert!(doc.meta.title.is_empty());
        assert_eq!(node_type(&doc.content, 0), "scene_heading");
        assert_eq!(node_text(&doc.content, 0), "INT. HOUSE - DAY");
    }

    // ─── Scene headings ──────────────────────────────────────────────

    #[test]
    fn scene_heading_auto_detect() {
        let input = "INT. HOUSE - DAY\n\nJohn walks in.\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_type(&doc.content, 0), "scene_heading");
        assert_eq!(node_text(&doc.content, 0), "INT. HOUSE - DAY");
        assert_eq!(node_type(&doc.content, 1), "action");
    }

    #[test]
    fn scene_heading_forced_with_period() {
        let input = ".THE BEACH\n\nThe surf rolls in.\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_type(&doc.content, 0), "scene_heading");
        assert_eq!(node_text(&doc.content, 0), "THE BEACH");
    }

    #[test]
    fn double_period_is_not_forced_scene_heading() {
        // Per spec, the forced-scene marker is a single `.`, not `..`.
        // `..something` should fall through to action.
        let input = "..mystery dots\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_type(&doc.content, 0), "action");
    }

    #[test]
    fn scene_heading_drops_optional_scene_number() {
        let input = "INT. OFFICE - DAY #1A#\n\nAction.\n";
        let (doc, summary) = parse_fountain(input).unwrap();
        assert_eq!(node_text(&doc.content, 0), "INT. OFFICE - DAY");
        assert_eq!(summary.scene_numbers_dropped, 1);
    }

    // ─── Action ──────────────────────────────────────────────────────

    #[test]
    fn forced_action_with_bang_prefix() {
        // `!BANG!` should be action even though it's all-caps.
        let input = "INT. HOUSE - DAY\n\n!BANG!\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_type(&doc.content, 1), "action");
        assert_eq!(node_text(&doc.content, 1), "BANG!");
    }

    #[test]
    fn all_caps_action_followed_by_blank_stays_action() {
        // Per spec: a character cue requires a non-empty next line.
        // `BANG!` followed by a blank line is action, not character.
        let input = "INT. HOUSE - DAY\n\nBANG!\n\nJohn enters.\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_type(&doc.content, 1), "action");
        assert_eq!(node_text(&doc.content, 1), "BANG!");
    }

    // ─── Character / dialogue / parenthetical ────────────────────────

    #[test]
    fn character_dialogue_simple() {
        let input = "INT. HOUSE - DAY\n\nJOHN\nHello.\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_type(&doc.content, 1), "character");
        assert_eq!(node_text(&doc.content, 1), "JOHN");
        assert_eq!(node_type(&doc.content, 2), "dialogue");
        assert_eq!(node_text(&doc.content, 2), "Hello.");
    }

    #[test]
    fn character_with_extension_in_parens() {
        // Per spec: lowercase inside `(...)` is allowed in character cues.
        let input = "INT. HOUSE - DAY\n\nMOM (V.O.)\nHello dear.\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_type(&doc.content, 1), "character");
        assert_eq!(node_text(&doc.content, 1), "MOM (V.O.)");
    }

    #[test]
    fn forced_character_with_at_prefix_supports_malayalam() {
        // Critical for Malayalam — caseless script can never satisfy the
        // all-caps rule, so the @ prefix is mandatory for those cues.
        let input = "INT. HOUSE - DAY\n\n@രമേശ്\nനമസ്കാരം\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_type(&doc.content, 1), "character");
        assert_eq!(node_text(&doc.content, 1), "രമേശ്");
        assert_eq!(node_type(&doc.content, 2), "dialogue");
        assert_eq!(node_text(&doc.content, 2), "നമസ്കാരം");
    }

    #[test]
    fn parenthetical_after_character() {
        let input = "INT. HOUSE - DAY\n\nMARY\n(softly)\nI know.\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_type(&doc.content, 2), "parenthetical");
        assert_eq!(node_text(&doc.content, 2), "(softly)");
        assert_eq!(node_type(&doc.content, 3), "dialogue");
        assert_eq!(node_text(&doc.content, 3), "I know.");
    }

    #[test]
    fn dual_dialogue_caret_collapses_to_sequential() {
        let input = "INT. HOUSE - DAY\n\nMARY\nHi.\n\nTOM ^\nHello.\n";
        let (doc, summary) = parse_fountain(input).unwrap();
        // Both characters present, in order.
        assert_eq!(node_text(&doc.content, 1), "MARY");
        assert_eq!(node_text(&doc.content, 2), "Hi.");
        assert_eq!(node_text(&doc.content, 3), "TOM");
        assert_eq!(node_text(&doc.content, 4), "Hello.");
        assert_eq!(summary.dual_dialogue_count, 1);
    }

    // ─── Transitions ─────────────────────────────────────────────────

    #[test]
    fn transition_auto_detect() {
        let input = "INT. HOUSE - DAY\n\nAction.\n\nCUT TO:\n\nEXT. PARK - NIGHT\n";
        let (doc, _) = parse_fountain(input).unwrap();
        let txn_idx = (0..node_count(&doc.content))
            .find(|i| node_type(&doc.content, *i) == "transition")
            .expect("transition node");
        assert_eq!(node_text(&doc.content, txn_idx), "CUT TO:");
    }

    #[test]
    fn forced_transition_with_gt_prefix() {
        let input = "INT. HOUSE - DAY\n\n> FADE OUT.\n";
        let (doc, _) = parse_fountain(input).unwrap();
        let txn_idx = (0..node_count(&doc.content))
            .find(|i| node_type(&doc.content, *i) == "transition")
            .expect("transition node");
        assert_eq!(node_text(&doc.content, txn_idx), "FADE OUT.");
    }

    #[test]
    fn fade_in_at_top_is_transition() {
        let input = "FADE IN:\n\nINT. HOUSE - DAY\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_type(&doc.content, 0), "transition");
        assert_eq!(node_text(&doc.content, 0), "FADE IN:");
    }

    // ─── Centred text ────────────────────────────────────────────────

    #[test]
    fn centered_text_becomes_action() {
        let input = "INT. HOUSE - DAY\n\n> THE END <\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_type(&doc.content, 1), "action");
        assert_eq!(node_text(&doc.content, 1), "THE END");
    }

    // ─── Synopses, sections, notes ───────────────────────────────────

    #[test]
    fn synopsis_attaches_to_preceding_scene_description() {
        let input = "INT. HOUSE - DAY\n= Hero meets villain\n\nAction.\n";
        let (doc, summary) = parse_fountain(input).unwrap();
        assert_eq!(summary.synopses_count, 1);
        let card = &doc.scene_cards[0];
        assert_eq!(card.scene_index, 0);
        assert_eq!(card.description, "Hero meets villain");
    }

    #[test]
    fn section_attaches_to_next_scene_with_depth_marker() {
        let input = "# Act One\n\nINT. HOUSE - DAY\n\nAction.\n";
        let (doc, summary) = parse_fountain(input).unwrap();
        assert_eq!(summary.sections_count, 1);
        let card = &doc.scene_cards[0];
        assert_eq!(card.scene_index, 0);
        assert_eq!(card.shoot_notes, "[[#section depth=1]] Act One");
    }

    #[test]
    fn nested_sections_preserve_depth() {
        let input = "## Sequence A\n\nINT. HOUSE - DAY\n\nAction.\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(
            doc.scene_cards[0].shoot_notes,
            "[[#section depth=2]] Sequence A"
        );
    }

    #[test]
    fn standalone_note_attaches_to_containing_scene_notes() {
        let input = "INT. HOUSE - DAY\n\n[[check this name]]\n\nAction.\n";
        let (doc, summary) = parse_fountain(input).unwrap();
        assert_eq!(summary.notes_count, 1);
        assert_eq!(doc.scene_cards[0].shoot_notes, "check this name");
    }

    // ─── Boneyard ────────────────────────────────────────────────────

    #[test]
    fn boneyard_is_dropped_silently_with_count() {
        let input = "INT. HOUSE - DAY\n\n/* cut scene */\n\nAction.\n";
        let (doc, summary) = parse_fountain(input).unwrap();
        assert_eq!(summary.boneyards_dropped, 1);
        // The boneyard removal leaves a blank line, so the action lands
        // as the second node after the scene heading.
        let action_idx = (0..node_count(&doc.content))
            .find(|i| node_type(&doc.content, *i) == "action")
            .expect("action node");
        assert_eq!(node_text(&doc.content, action_idx), "Action.");
    }

    #[test]
    fn boneyard_can_span_multiple_lines() {
        let input = "INT. HOUSE - DAY\n\n/* cut\nscene\nspans lines */\n\nAction.\n";
        let (doc, summary) = parse_fountain(input).unwrap();
        assert_eq!(summary.boneyards_dropped, 1);
        let action_idx = (0..node_count(&doc.content))
            .find(|i| node_type(&doc.content, *i) == "action")
            .expect("action node");
        assert_eq!(node_text(&doc.content, action_idx), "Action.");
    }

    // ─── Page break ──────────────────────────────────────────────────

    #[test]
    fn page_break_is_dropped() {
        let input = "INT. HOUSE - DAY\n\nAction.\n\n===\n\nMore action.\n";
        let (doc, _) = parse_fountain(input).unwrap();
        // No page-break node in our schema; we should see two action
        // nodes after the scene heading.
        let actions: Vec<_> = (0..node_count(&doc.content))
            .filter(|i| node_type(&doc.content, *i) == "action")
            .collect();
        assert_eq!(actions.len(), 2);
    }

    // ─── Emphasis ────────────────────────────────────────────────────

    #[test]
    fn emphasis_markers_are_stripped() {
        let input = "INT. HOUSE - DAY\n\nHe said *softly* and **firmly**.\n";
        let (doc, summary) = parse_fountain(input).unwrap();
        assert!(summary.emphasis_stripped >= 1);
        assert_eq!(node_text(&doc.content, 1), "He said softly and firmly.");
    }

    #[test]
    fn escaped_emphasis_stays_literal() {
        let input = "INT. HOUSE - DAY\n\n2 \\* 3 = 6\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_text(&doc.content, 1), "2 * 3 = 6");
    }

    #[test]
    fn arithmetic_asterisk_with_spaces_stays_literal() {
        // Spaces on both sides → not emphasis per spec heuristic.
        let input = "INT. HOUSE - DAY\n\n2 * 3 = 6\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(node_text(&doc.content, 1), "2 * 3 = 6");
    }

    // ─── Empty / minimal ─────────────────────────────────────────────

    #[test]
    fn empty_input_produces_blank_scene_heading() {
        let (doc, _) = parse_fountain("").unwrap();
        assert_eq!(first_node_type(&doc.content), "scene_heading");
    }

    #[test]
    fn title_page_only_produces_blank_body() {
        let input = "Title: Just A Title\n\n";
        let (doc, _) = parse_fountain(input).unwrap();
        assert_eq!(doc.meta.title, "Just A Title");
        assert_eq!(first_node_type(&doc.content), "scene_heading");
    }

    // ─── Malayalam warning ───────────────────────────────────────────

    #[test]
    fn malayalam_without_at_prefix_emits_warning() {
        // 20+ Malayalam chars, no @-prefixed cue.
        let input = "INT. HOUSE - DAY\n\nരമേശ് വീട്ടിലേക്ക് നടന്നു അവൻ കിതച്ചു.\n";
        let (_doc, summary) = parse_fountain(input).unwrap();
        assert!(
            summary
                .warnings
                .iter()
                .any(|w| w.contains("Malayalam") && w.contains("@-prefixed")),
            "expected Malayalam warning, got {:?}",
            summary.warnings
        );
    }

    #[test]
    fn malayalam_with_at_prefix_no_warning() {
        let input = "INT. HOUSE - DAY\n\n@രമേശ്\nനമസ്കാരം ഇത് ഒരു സന്ദേശമാണ്.\n";
        let (_doc, summary) = parse_fountain(input).unwrap();
        assert!(
            !summary.warnings.iter().any(|w| w.contains("Malayalam")),
            "did not expect Malayalam warning, got {:?}",
            summary.warnings
        );
    }

    // ─── Mixed full-screenplay smoke test ────────────────────────────

    #[test]
    fn full_screenplay_smoke_test() {
        let input = "Title: Smoke\nAuthor: Hrishi\n\
                     \n\
                     # Act One\n\
                     \n\
                     INT. HOUSE - DAY\n\
                     = Hero wakes up\n\
                     \n\
                     The room is dim.\n\
                     \n\
                     [[check the lamp]]\n\
                     \n\
                     JOHN\n\
                     (groggy)\n\
                     What time is it?\n\
                     \n\
                     CUT TO:\n\
                     \n\
                     EXT. STREET - NIGHT\n\
                     \n\
                     Rain falls.\n";
        let (doc, summary) = parse_fountain(input).unwrap();
        assert_eq!(doc.meta.title, "Smoke");
        assert_eq!(doc.meta.author, "Hrishi");
        assert_eq!(summary.sections_count, 1);
        assert_eq!(summary.synopses_count, 1);
        assert_eq!(summary.notes_count, 1);
        assert_eq!(summary.boneyards_dropped, 0);

        // Two scene cards (one per scene heading), card[0] populated.
        assert_eq!(doc.scene_cards.len(), 1); // only scene 0 picked up
                                              // attachments
        assert_eq!(doc.scene_cards[0].scene_index, 0);
        assert_eq!(doc.scene_cards[0].description, "Hero wakes up");
        assert!(doc.scene_cards[0]
            .shoot_notes
            .contains("[[#section depth=1]] Act One"));
        assert!(doc.scene_cards[0].shoot_notes.contains("check the lamp"));

        // Spot-check ordering.
        assert_eq!(node_type(&doc.content, 0), "scene_heading");
        let txn = (0..node_count(&doc.content))
            .find(|i| node_type(&doc.content, *i) == "transition")
            .expect("transition");
        assert_eq!(node_text(&doc.content, txn), "CUT TO:");
        let last_scene = (0..node_count(&doc.content))
            .rfind(|i| node_type(&doc.content, *i) == "scene_heading")
            .unwrap();
        assert_eq!(node_text(&doc.content, last_scene), "EXT. STREET - NIGHT");
    }
}
