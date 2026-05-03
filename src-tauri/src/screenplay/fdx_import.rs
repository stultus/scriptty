// Final Draft import: .fdx (XML) → ProseMirror JSON + meta + scene_cards.
//
// Final Draft does not publish a public schema for FDX; the format is
// XML 1.0 / UTF-8 with a `<FinalDraft><Content>` root and per-paragraph
// `Type=` attributes. We pull-parse with `quick-xml`, collect paragraphs
// into a flat intermediate representation, then emit ProseMirror nodes
// + scene_cards in a second pass — the same shape `fountain_import` uses
// so the two flows feel symmetrical to the writer.
//
// Mapping decisions are locked in #190; refer there for the table. v1
// scope:
//   - 6 native paragraph types map to native nodes (plus Shot →
//     scene_heading, General/Lyrics/Outline → action)
//   - inline `<Text Style="Bold+Italic+Underline">` runs become
//     ProseMirror bold/italic/underline marks
//   - <DualDialogue> wrappers collapse to sequential pairs (counted)
//   - <Revisions>, <ScriptNotes>, <TagData>, locked Numbers, etc. are
//     dropped with counts surfaced to the import summary toast
//   - title page best-effort — heuristic fills meta.title/author/
//     draft_date/contact, full text always lands in
//     `meta.extra["fdx_title_page"]` so nothing is silently lost
//   - the FDX `Version` attribute lands in
//     `meta.extra["fdx_source_version"]` as a debug aid

use crate::screenplay::document::{
    ProjectType, ScreenplayDocument, ScreenplayMeta, ScreenplaySettings, ScreenplayStory,
};
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use serde_json::{json, Value};

/// Counts and warnings reported back to the frontend so the import-summary
/// toast can describe what the importer transformed or dropped. Field
/// names are stable across versions — TS bindings depend on them.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct FdxImportSummary {
    /// Number of `<DualDialogue>` blocks we flattened to sequential pairs.
    pub dual_dialogue_count: usize,
    /// Number of `<ScriptNote>` elements (anywhere in the file) we dropped.
    pub script_notes_dropped: usize,
    /// Number of revision sets dropped (the count of `<Text RevisionID>`
    /// values ≥ 1, deduplicated). We don't try to preserve revision marks
    /// — the `Range="loc,end"` re-anchoring problem is intractable once
    /// the editor has touched the doc.
    pub revisions_dropped: usize,
    /// Number of scene headings whose `Number=` (locked production scene
    /// number) we dropped. Important for the writer to know — locked
    /// numbers matter on a real production. Future enhancement: preserve
    /// in a per-scene metadata bag.
    pub locked_scene_numbers_dropped: usize,
    /// Number of `<TagData>` entries dropped (Final Draft 12+ production
    /// tagging). Scriptty has no equivalent feature.
    pub tag_data_dropped: usize,
    /// Number of paragraphs whose `Type=` we didn't recognise; these are
    /// folded into Action with a count so the writer can spot-check.
    pub unknown_types_folded_to_action: usize,
    /// Free-form warnings — surfaced verbatim in the toast.
    pub warnings: Vec<String>,
}

/// Parse a `.fdx` file into a Film-shaped `ScreenplayDocument` plus an
/// import summary. Returns `Err` only on malformed XML; everything else
/// the importer encounters either maps to a Scriptty concept or gets
/// counted-and-dropped.
pub fn parse_fdx(input: &str) -> Result<(ScreenplayDocument, FdxImportSummary), String> {
    // Strip a leading UTF-8 BOM if present — quick-xml accepts the BOM
    // but it ends up as a literal char in the first text event, which
    // would then leak into the title-page heuristic.
    let trimmed = input.strip_prefix('\u{FEFF}').unwrap_or(input);

    let mut reader = Reader::from_str(trimmed);
    // The default config trims neither whitespace nor empty events; we
    // want raw text so the writer's intentional spacing in `<Text>`
    // children is preserved.
    reader.config_mut().trim_text(false);

    let mut summary = FdxImportSummary::default();
    let mut state = ParseState::default();
    let mut buf: Vec<u8> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf).map_err(|e| {
            format!(
                "XML parse error at byte {}: {}",
                reader.buffer_position(),
                e
            )
        })? {
            Event::Start(e) => handle_start(&e, &mut state, &reader)?,
            Event::Empty(e) => {
                // Self-closing: handle as Start+End back-to-back. Empty
                // elements occur in real FDX for spacers (`<Text/>`) and
                // attribute-only nodes (`<Page Number="1A"/>`).
                handle_start(&e, &mut state, &reader)?;
                handle_end(e.name(), &mut state, &mut summary)?;
            }
            Event::Text(e) => {
                // quick-xml 0.38 emits entity references as their own
                // `GeneralRef` events instead of inlining them into
                // Text. So a Text event's content is already free of
                // entities — `xml_content()` is decode + EOL-normalise,
                // no entity unescape needed.
                let text = e
                    .xml_content()
                    .map_err(|err| format!("XML text decode error: {err}"))?
                    .into_owned();
                handle_text(&text, &mut state);
            }
            Event::GeneralRef(e) => {
                // Entity references (`&amp;`, `&lt;`, `&#65;`, `&#xFF;`).
                // Numeric character refs decode via `resolve_char_ref`;
                // named XML entities (the standard five) resolve via
                // `escape::resolve_predefined_entity`. Anything else —
                // a custom DTD-defined entity — we drop with a warning
                // counter, which is acceptable for FDX (the format
                // doesn't define custom entities).
                if let Some(ch) = e
                    .resolve_char_ref()
                    .map_err(|err| format!("XML char ref error: {err}"))?
                {
                    let s = ch.to_string();
                    handle_text(&s, &mut state);
                } else {
                    let name = e
                        .decode()
                        .map_err(|err| format!("XML entity decode error: {err}"))?;
                    if let Some(resolved) = resolve_predefined_entity(&name) {
                        handle_text(resolved, &mut state);
                    }
                    // Else: custom entity, silently dropped — FDX uses
                    // only the predefined five.
                }
            }
            Event::End(e) => handle_end(e.name(), &mut state, &mut summary)?,
            Event::Eof => break,
            // Comments, processing instructions, CDATA, declarations —
            // nothing in real FDX needs special handling here. CDATA in
            // particular doesn't appear in any FDX I checked, but if a
            // third-party app emitted one we'd just lose its content,
            // which is a known v1 limitation.
            _ => {}
        }
        buf.clear();
    }

    // Transfer the deduplicated revision-id set into the summary count
    // before consuming the rest of the parse state.
    summary.revisions_dropped = state.revision_ids_seen.len();

    let (content, scene_cards) = build_content(state.body, &mut summary);
    let mut meta = build_meta(state.title_page_lines, state.fdx_version, &mut summary);
    finalise_meta(&mut meta);

    Ok((
        ScreenplayDocument {
            project_type: ProjectType::Film,
            series: None,
            content,
            meta,
            settings: ScreenplaySettings::default(),
            story: ScreenplayStory::default(),
            scene_cards,
        },
        summary,
    ))
}

// ─── Parser state ────────────────────────────────────────────────────────────

/// Tracks where we are in the FDX tree. The state machine is intentionally
/// flat — FDX nesting is shallow and predictable, so a handful of bools +
/// the active paragraph buffer covers every case we care about.
#[derive(Default)]
struct ParseState {
    /// Source FDX `Version` attribute, captured when we open `<FinalDraft>`.
    fdx_version: Option<String>,

    /// Flat list of body paragraphs collected from `<Content>` (the top-
    /// level one inside `<FinalDraft>`, **not** `<TitlePage><Content>`).
    body: Vec<FdxParagraph>,

    /// Lines extracted from `<TitlePage>` — one entry per non-empty
    /// `<Paragraph><Text>`. Used by `build_meta` to do the heuristic
    /// title-page reconstruction.
    title_page_lines: Vec<TitlePageLine>,

    /// Currently-open paragraph being built. `None` outside `<Paragraph>`.
    current_para: Option<FdxParagraph>,

    /// Currently-open text run being built. `None` between `<Text>` ends
    /// and the next `<Text>` start.
    current_run: Option<TextRun>,

    /// Are we inside a `<DualDialogue>` wrapper? Inner `<Paragraph>`s
    /// inherit this flag so we can mark them in the IR for the count.
    in_dual_dialogue: bool,

    /// Are we inside `<TitlePage>`? If so, paragraphs go to
    /// `title_page_lines` instead of `body`.
    in_title_page: bool,

    /// Are we inside the body `<Content>` (the one under `<FinalDraft>`,
    /// not the one under `<TitlePage>`)? Nested `<Content>`s exist —
    /// this flag prevents mis-routing.
    in_body_content: bool,

    /// Element-name stack used to determine the meaning of a `<Content>`
    /// (its parent disambiguates body vs title-page).
    elem_stack: Vec<String>,

    /// Inside a `<ScriptNote>` subtree we suppress all text — script
    /// notes are dropped wholesale per #190 scope.
    in_script_note_depth: u32,

    /// Inside a `<TagData>` subtree we similarly drop everything.
    in_tag_data_depth: u32,

    /// Distinct non-zero `<Text RevisionID>` values seen, so we can
    /// report a count without storing the per-run IDs in the IR.
    revision_ids_seen: std::collections::BTreeSet<String>,

    /// Was the current paragraph's scene heading carrying a `Number=`?
    /// Counted on paragraph close.
    current_para_had_locked_number: bool,
}

#[derive(Debug, Clone)]
struct FdxParagraph {
    /// Raw `Type=` attribute (`"Scene Heading"`, `"Action"`, etc.). May
    /// be empty for FD's untyped paragraphs (treated as Action).
    type_attr: String,
    /// Plain text content — concatenation of all `<Text>` children with
    /// no formatting info. Fast path for callers that don't care about
    /// marks.
    plain_text: String,
    /// One entry per `<Text>` child, in order. Carries text + style
    /// flags so the emitter can attach ProseMirror marks.
    runs: Vec<TextRun>,
}

#[derive(Debug, Clone, Default)]
struct TextRun {
    text: String,
    bold: bool,
    italic: bool,
    underline: bool,
}

#[derive(Debug, Clone)]
struct TitlePageLine {
    text: String,
    /// `<Paragraph Alignment=>` attribute — `Left`/`Center`/`Centered`/
    /// `Right`. Used by the title-page heuristic to identify the title
    /// (the longest centred line) vs contact info (left-aligned).
    alignment: String,
}

// ─── Event handlers ──────────────────────────────────────────────────────────

fn handle_start(
    e: &quick_xml::events::BytesStart,
    state: &mut ParseState,
    reader: &Reader<&[u8]>,
) -> Result<(), String> {
    let local = local_name(e.name());
    state.elem_stack.push(local.clone());

    // Inside a script-note or tag-data subtree we drop everything wholesale.
    // The depth counter has already been bumped on the opening element of
    // the subtree; while we're inside it, we must not let nested
    // <Paragraph>/<Text> elements clobber the outer paragraph being built.
    // (The note's own text will be dropped because handle_text checks the
    // same depth flags.)
    if state.in_script_note_depth > 0 || state.in_tag_data_depth > 0 {
        // Still recognise the subtree-opening elements themselves so the
        // matching End handler can decrement; but skip everything else.
        match local.as_str() {
            "ScriptNote" => state.in_script_note_depth += 1,
            "TagData" => state.in_tag_data_depth += 1,
            _ => {}
        }
        return Ok(());
    }

    match local.as_str() {
        "FinalDraft" => {
            // Capture the Version attribute for meta.extra. The actual
            // value is informational only — readers should be lenient.
            for attr in e.attributes().flatten() {
                if local_name(attr.key) == "Version" {
                    state.fdx_version = Some(decode_attr(&attr, reader)?);
                }
            }
        }
        "TitlePage" => state.in_title_page = true,
        "Content" => {
            // Disambiguate body Content from TitlePage Content via the
            // element below us in the stack. Body content's parent is
            // `FinalDraft`; title-page Content's parent is `TitlePage`.
            // The top of the stack right now is "Content" itself — peek
            // at the second-to-top.
            let parent = state
                .elem_stack
                .get(state.elem_stack.len().saturating_sub(2))
                .cloned()
                .unwrap_or_default();
            if parent == "FinalDraft" {
                state.in_body_content = true;
            }
        }
        "Paragraph" => {
            // Open a paragraph buffer. Type / Alignment / Number come
            // from attributes. An untyped Paragraph (used as a wrapper
            // for DualDialogue) has type_attr empty — we detect that
            // pattern below.
            let mut type_attr = String::new();
            let mut alignment = String::new();
            let mut had_number = false;
            for attr in e.attributes().flatten() {
                match local_name(attr.key).as_str() {
                    "Type" => type_attr = decode_attr(&attr, reader)?,
                    "Alignment" => alignment = decode_attr(&attr, reader)?,
                    "Number" => had_number = true,
                    _ => {}
                }
            }
            // For wrappers around <DualDialogue>, type_attr will be
            // empty and the paragraph carries no text — the inner
            // paragraphs do all the work. We open a buffer regardless;
            // it just won't be used.
            state.current_para = Some(FdxParagraph {
                type_attr,
                plain_text: String::new(),
                runs: Vec::new(),
            });
            state.current_para_had_locked_number = had_number;

            // Title-page paragraphs need the alignment for the heuristic.
            if state.in_title_page {
                // Stash the alignment on a per-line basis when we close
                // the paragraph. Done via a closure variable below.
                state.elem_stack.last_mut().unwrap().push_str(":__align=");
                state.elem_stack.last_mut().unwrap().push_str(&alignment);
            }
        }
        "Text" => {
            // Open a text run, capturing Style flags from `Style="Bold"`
            // / `"Italic"` / `"Underline"` (joined by `+`). Other style
            // values (`Strikeout`, `AllCaps`, `Highlight`, etc.) are
            // intentionally dropped.
            let mut run = TextRun::default();
            for attr in e.attributes().flatten() {
                match local_name(attr.key).as_str() {
                    "Style" => {
                        let val = decode_attr(&attr, reader)?;
                        for s in val.split('+') {
                            match s.trim() {
                                "Bold" => run.bold = true,
                                "Italic" => run.italic = true,
                                "Underline" => run.underline = true,
                                _ => {}
                            }
                        }
                    }
                    "RevisionID" => {
                        let id = decode_attr(&attr, reader)?;
                        if id != "0" && !id.is_empty() {
                            state.revision_ids_seen.insert(id);
                        }
                    }
                    _ => {}
                }
            }
            state.current_run = Some(run);
        }
        "DualDialogue" => state.in_dual_dialogue = true,
        "ScriptNote" => state.in_script_note_depth += 1,
        "TagData" => state.in_tag_data_depth += 1,
        _ => {}
    }
    Ok(())
}

fn handle_text(text: &str, state: &mut ParseState) {
    // Drop text inside script-note / tag-data subtrees per #190 scope.
    if state.in_script_note_depth > 0 || state.in_tag_data_depth > 0 {
        return;
    }
    if let Some(run) = state.current_run.as_mut() {
        run.text.push_str(text);
    }
}

fn handle_end(
    name: QName,
    state: &mut ParseState,
    summary: &mut FdxImportSummary,
) -> Result<(), String> {
    let local = local_name(name);
    match local.as_str() {
        "TitlePage" => state.in_title_page = false,
        "Content" => {
            let parent = state
                .elem_stack
                .get(state.elem_stack.len().saturating_sub(2))
                .cloned()
                .unwrap_or_default();
            if parent == "FinalDraft" {
                state.in_body_content = false;
            }
        }
        "Paragraph" => {
            // Close the paragraph and route it.
            let para = state.current_para.take();
            // Pop the alignment stash off the stack entry's name. We
            // appended `:__align=...` to the top frame for title-page
            // paragraphs only.
            let frame = state.elem_stack.last().cloned().unwrap_or_default();
            let align_value = frame
                .split_once(":__align=")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();

            if let Some(mut para) = para {
                // Update per-paragraph counters: locked scene numbers
                // are dropped, counted.
                if state.current_para_had_locked_number && para.type_attr == "Scene Heading" {
                    summary.locked_scene_numbers_dropped += 1;
                }

                // Coalesce runs into plain_text for fast access.
                for run in &para.runs {
                    para.plain_text.push_str(&run.text);
                }

                if state.in_title_page {
                    // Title-page paragraph — flatten and stash for the
                    // heuristic. Don't strip whitespace at this stage;
                    // keep blank-ish lines as empty strings so the
                    // heuristic sees the original spacing pattern.
                    state.title_page_lines.push(TitlePageLine {
                        text: para.plain_text.clone(),
                        alignment: align_value,
                    });
                } else if state.in_body_content {
                    // Body paragraph. An untyped wrapper that exists
                    // solely to host a DualDialogue has empty
                    // type_attr and no runs — skip it; the inner
                    // paragraphs were already pushed by their own
                    // close handlers.
                    if !(para.type_attr.is_empty() && para.runs.is_empty()) {
                        state.body.push(para);
                    }
                }
            }
            state.current_para_had_locked_number = false;
        }
        "Text" => {
            // Close the text run and attach to the active paragraph.
            if let Some(run) = state.current_run.take() {
                if state.in_script_note_depth == 0 && state.in_tag_data_depth == 0 {
                    if let Some(para) = state.current_para.as_mut() {
                        para.runs.push(run);
                    }
                }
            }
        }
        "DualDialogue" => {
            state.in_dual_dialogue = false;
            summary.dual_dialogue_count += 1;
        }
        "ScriptNote" => {
            state.in_script_note_depth = state.in_script_note_depth.saturating_sub(1);
            summary.script_notes_dropped += 1;
        }
        "TagData" => {
            state.in_tag_data_depth = state.in_tag_data_depth.saturating_sub(1);
            summary.tag_data_dropped += 1;
        }
        _ => {}
    }
    state.elem_stack.pop();
    Ok(())
}

fn local_name(name: QName) -> String {
    // FDX never namespaces, but be defensive: strip any prefix.
    String::from_utf8_lossy(name.local_name().as_ref()).into_owned()
}

fn decode_attr(
    attr: &quick_xml::events::attributes::Attribute,
    reader: &Reader<&[u8]>,
) -> Result<String, String> {
    // `decode_and_unescape_value` handles both XML entity decoding and
    // the source encoding declared in the XML prolog. quick-xml 0.36
    // returns `Cow<str>` so we just clone into an owned String.
    attr.decode_and_unescape_value(reader.decoder())
        .map(|cow| cow.into_owned())
        .map_err(|e| format!("attribute decode error: {e}"))
}

// ─── Body emission ───────────────────────────────────────────────────────────

fn build_content(
    paragraphs: Vec<FdxParagraph>,
    summary: &mut FdxImportSummary,
) -> (Value, Vec<crate::screenplay::document::SceneCard>) {
    let mut nodes: Vec<Value> = Vec::new();

    for para in paragraphs {
        // Empty paragraphs in the body are vertical spacers in FD's
        // page layout — not screenplay content. Skip.
        if para.plain_text.trim().is_empty() {
            continue;
        }

        let kind = map_type(&para.type_attr, summary);
        let inline_children = build_inline_children(&para.runs, kind);
        nodes.push(json!({
            "type": kind,
            "content": inline_children,
        }));
    }

    if nodes.is_empty() {
        // ProseMirror's `doc` requires at least one block child — emit
        // an empty scene heading so the editor can open the imported
        // file even when the FDX had nothing in <Content>.
        nodes.push(json!({ "type": "scene_heading" }));
    }

    let content = json!({ "type": "doc", "content": nodes });
    // We don't emit scene_cards in v1 — there's nothing FDX-side to
    // populate them with that we don't drop (synopses don't exist in
    // FDX, script notes are intentionally dropped).
    (content, Vec::new())
}

/// Map an FDX `Type=` to a Scriptty schema type. Returns the schema
/// name as a `&'static str`.
fn map_type(fdx_type: &str, summary: &mut FdxImportSummary) -> &'static str {
    match fdx_type {
        "Scene Heading" | "Shot" => "scene_heading",
        "Action" | "General" | "Lyrics" => "action",
        "Character" => "character",
        "Parenthetical" => "parenthetical",
        "Dialogue" => "dialogue",
        "Transition" => "transition",
        // Outline 1, Outline 2, ... fall here.
        t if t.starts_with("Outline") => "action",
        // Untyped paragraphs (rare in real files; happen in
        // <DualDialogue> wrappers but those are filtered upstream).
        "" => "action",
        // Anything else — Cast List, More, New Act, etc. — fold into
        // action so the writer doesn't lose the text. Counted so the
        // toast can report the fold.
        _ => {
            summary.unknown_types_folded_to_action += 1;
            "action"
        }
    }
}

/// Build the inline children array for a ProseMirror block node from
/// a paragraph's text runs. Empty runs become an empty array (the
/// node renders as a blank paragraph of that type, which is what we
/// want).
///
/// `node_kind` decides whether we apply the FD "auto-uppercase on
/// render" convention for Scene Heading / Character / Transition.
/// FDX stores those as the writer authored them; FD uppercases at
/// render time. Scriptty's editor expects them already uppercased,
/// matching what the editor's auto-uppercase plugin would have
/// produced if the user typed them locally.
fn build_inline_children(runs: &[TextRun], node_kind: &str) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    let upper_kind = matches!(node_kind, "scene_heading" | "character" | "transition");
    for run in runs {
        if run.text.is_empty() {
            continue;
        }
        let text = if upper_kind {
            run.text.to_uppercase()
        } else {
            run.text.clone()
        };
        let mut marks: Vec<Value> = Vec::new();
        if run.bold {
            marks.push(json!({ "type": "bold" }));
        }
        if run.italic {
            marks.push(json!({ "type": "italic" }));
        }
        if run.underline {
            marks.push(json!({ "type": "underline" }));
        }
        let mut node = json!({ "type": "text", "text": text });
        if !marks.is_empty() {
            node["marks"] = json!(marks);
        }
        out.push(node);
    }
    out
}

// ─── Title page / meta ───────────────────────────────────────────────────────

fn build_meta(
    title_lines: Vec<TitlePageLine>,
    fdx_version: Option<String>,
    _summary: &mut FdxImportSummary,
) -> ScreenplayMeta {
    let mut meta = ScreenplayMeta::default();

    // Fast path: no title page at all.
    if title_lines.is_empty() {
        if let Some(v) = fdx_version {
            meta.extra.insert("fdx_source_version".into(), v);
        }
        return meta;
    }

    // Stash the full title-page text verbatim — the heuristic below is
    // best-effort, and we don't want a wrong guess to silently lose the
    // writer's actual title-page content. The block goes into
    // `meta.extra` so it's preserved on save and visible to anyone who
    // wants to manually re-key the standard fields.
    let full_text = title_lines
        .iter()
        .map(|l| l.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    if !full_text.trim().is_empty() {
        meta.extra.insert("fdx_title_page".into(), full_text);
    }

    // Beat-style heuristic for the standard fields. This is a best-
    // effort guess — Final Draft's title page has no semantic markers,
    // so we infer roles from line position and alignment.
    apply_title_page_heuristic(&title_lines, &mut meta);

    if let Some(v) = fdx_version {
        meta.extra.insert("fdx_source_version".into(), v);
    }

    meta
}

/// Heuristic mapping of free-form title-page lines to standard meta
/// fields. Mirrors Beat's emit conventions (Title → "Written by" →
/// Author → Source → blank → Contact / Draft Date), reading them in
/// reverse:
///
///   1. The longest centred line in the upper portion of the page is
///      almost certainly the title.
///   2. The centred line that most resembles a person's name and sits
///      just below a `Written by` / `Authors:` / `Story by` cue is the
///      author.
///   3. Left-aligned lines near the bottom of the page are contact
///      info (email, phone, agent block).
///   4. Anything matching a date pattern in the bottom half is the
///      draft date.
///
/// Where the heuristic fails (atypical layouts), the standard fields
/// stay empty and the writer can fill them in via the Metadata modal.
/// `meta.extra["fdx_title_page"]` always carries the full text either
/// way.
fn apply_title_page_heuristic(lines: &[TitlePageLine], meta: &mut ScreenplayMeta) {
    let non_empty: Vec<&TitlePageLine> =
        lines.iter().filter(|l| !l.text.trim().is_empty()).collect();
    if non_empty.is_empty() {
        return;
    }

    // Find the centred line cluster. The title is typically the line
    // immediately preceding a "Written by" / "Story by" cue, or — when
    // no cue is present — the longest centred line.
    let centred: Vec<&&TitlePageLine> = non_empty
        .iter()
        .filter(|l| is_centered(&l.alignment))
        .collect();

    // Title detection: the line whose text is longest among the first
    // few centred lines. A single Title line is the canonical layout.
    if !centred.is_empty() {
        // Look at the first half of the centred lines (the title
        // typically sits before the "by" cue).
        let cap = centred.len().min(centred.len() / 2 + 2);
        let title_candidate = centred[..cap]
            .iter()
            .max_by_key(|l| l.text.trim().chars().count())
            .copied();
        if let Some(t) = title_candidate {
            meta.title = t.text.trim().to_string();
        }

        // Author detection: find a line that follows a "Written by" /
        // "Story by" / "By" cue. Falls back to the second-longest
        // centred line if no cue is found.
        let mut author = String::new();
        for w in centred.windows(2) {
            let cue = w[0].text.trim().to_lowercase();
            if matches!(
                cue.as_str(),
                "written by" | "story by" | "by" | "screenplay by" | "teleplay by"
            ) {
                author = w[1].text.trim().to_string();
                break;
            }
        }
        if author.is_empty() && centred.len() >= 2 {
            // Pick the longest centred line that isn't the title.
            let title_text = meta.title.clone();
            author = centred
                .iter()
                .map(|l| l.text.trim().to_string())
                .filter(|t| !t.is_empty() && t != &title_text)
                .max_by_key(|t| t.chars().count())
                .unwrap_or_default();
        }
        if !author.is_empty() {
            meta.author = author;
        }
    }

    // Contact + draft date: bottom-portion non-centred lines. We sweep
    // the last third of the title page.
    let cutoff = non_empty.len().saturating_sub(non_empty.len() / 3);
    let bottom = &non_empty[cutoff..];
    let mut contact_lines: Vec<String> = Vec::new();
    for line in bottom {
        let text = line.text.trim();
        if text.is_empty() {
            continue;
        }
        if meta.draft_date.is_empty() && looks_like_date(text) {
            meta.draft_date = text.to_string();
        } else if !is_centered(&line.alignment) {
            contact_lines.push(text.to_string());
        }
    }
    if !contact_lines.is_empty() {
        meta.contact = contact_lines.join("\n");
    }
}

fn is_centered(alignment: &str) -> bool {
    matches!(alignment, "Center" | "Centered" | "center" | "centered")
}

/// Cheap pattern check — does the line look like a date string? Catches
/// `2026-05-02`, `May 2, 2026`, `5/2/26`, `Draft date: ...` prefixes.
/// Deliberately lenient; false positives at worst put a date into
/// `meta.draft_date`, which the writer can correct in one click.
fn looks_like_date(s: &str) -> bool {
    let lower = s.to_lowercase();
    if lower.starts_with("draft date") || lower.starts_with("date:") {
        return true;
    }
    // ISO yyyy-mm-dd
    let chars: Vec<char> = s.chars().collect();
    if chars.len() >= 10
        && chars[0..4].iter().all(|c| c.is_ascii_digit())
        && chars[4] == '-'
        && chars[5..7].iter().all(|c| c.is_ascii_digit())
        && chars[7] == '-'
        && chars[8..10].iter().all(|c| c.is_ascii_digit())
    {
        return true;
    }
    // Month-name pattern: look for any English month name.
    const MONTHS: &[&str] = &[
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    MONTHS.iter().any(|m| lower.contains(m))
}

/// Attach derived metadata (revision counts, etc.) and tidy. Called
/// after the parse loop so we have access to the final summary state.
fn finalise_meta(_meta: &mut ScreenplayMeta) {
    // Hook for future tweaks; currently no-op.
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn node_type(content: &Value, idx: usize) -> String {
        content
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.get(idx))
            .and_then(|n| n.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string()
    }

    fn node_text(content: &Value, idx: usize) -> String {
        let arr = content
            .get("content")
            .and_then(|c| c.as_array())
            .unwrap_or(&Vec::new())
            .clone();
        let node = match arr.get(idx) {
            Some(n) => n.clone(),
            None => return String::new(),
        };
        node.get("content")
            .and_then(|c| c.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|n| n.get("text").and_then(|t| t.as_str()))
                    .collect::<String>()
            })
            .unwrap_or_default()
    }

    fn node_marks(content: &Value, block_idx: usize, run_idx: usize) -> Vec<String> {
        let arr = content
            .get("content")
            .and_then(|c| c.as_array())
            .unwrap_or(&Vec::new())
            .clone();
        let block = match arr.get(block_idx) {
            Some(n) => n.clone(),
            None => return Vec::new(),
        };
        block
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|runs| runs.get(run_idx))
            .and_then(|run| run.get("marks"))
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("type").and_then(|t| t.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn node_count(content: &Value) -> usize {
        content
            .get("content")
            .and_then(|c| c.as_array())
            .map(|a| a.len())
            .unwrap_or(0)
    }

    // ─── Minimal / structural ────────────────────────────────────────

    #[test]
    fn minimal_fdx_parses() {
        let input = r#"<?xml version="1.0" encoding="UTF-8" standalone="no" ?>
<FinalDraft DocumentType="Script" Template="No" Version="3">
  <Content>
  </Content>
</FinalDraft>"#;
        let (doc, summary) = parse_fdx(input).unwrap();
        assert_eq!(node_type(&doc.content, 0), "scene_heading");
        assert_eq!(summary.dual_dialogue_count, 0);
    }

    #[test]
    fn fdx_version_lands_in_meta_extra() {
        let input = r#"<?xml version="1.0" encoding="UTF-8" standalone="no" ?>
<FinalDraft Version="5"><Content/></FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        assert_eq!(
            doc.meta.extra.get("fdx_source_version").map(String::as_str),
            Some("5")
        );
    }

    #[test]
    fn utf8_bom_is_stripped() {
        let input = "\u{FEFF}<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
                     <FinalDraft Version=\"3\"><Content/></FinalDraft>";
        let result = parse_fdx(input);
        assert!(result.is_ok(), "BOM should not error: {result:?}");
    }

    // ─── Native types ────────────────────────────────────────────────

    #[test]
    fn six_native_types_map_correctly() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Scene Heading"><Text>INT. HOUSE - DAY</Text></Paragraph>
  <Paragraph Type="Action"><Text>John walks in.</Text></Paragraph>
  <Paragraph Type="Character"><Text>JOHN</Text></Paragraph>
  <Paragraph Type="Parenthetical"><Text>(softly)</Text></Paragraph>
  <Paragraph Type="Dialogue"><Text>Hello.</Text></Paragraph>
  <Paragraph Type="Transition"><Text>CUT TO:</Text></Paragraph>
</Content></FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        assert_eq!(node_type(&doc.content, 0), "scene_heading");
        assert_eq!(node_text(&doc.content, 0), "INT. HOUSE - DAY");
        assert_eq!(node_type(&doc.content, 1), "action");
        assert_eq!(node_text(&doc.content, 1), "John walks in.");
        assert_eq!(node_type(&doc.content, 2), "character");
        assert_eq!(node_type(&doc.content, 3), "parenthetical");
        assert_eq!(node_type(&doc.content, 4), "dialogue");
        assert_eq!(node_type(&doc.content, 5), "transition");
    }

    #[test]
    fn shot_type_folds_to_scene_heading() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Shot"><Text>WIDE SHOT</Text></Paragraph>
</Content></FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        assert_eq!(node_type(&doc.content, 0), "scene_heading");
    }

    #[test]
    fn general_lyrics_outline_fold_to_action() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="General"><Text>General text.</Text></Paragraph>
  <Paragraph Type="Lyrics"><Text>La la la</Text></Paragraph>
  <Paragraph Type="Outline 2"><Text>Sequence A</Text></Paragraph>
</Content></FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        for i in 0..3 {
            assert_eq!(node_type(&doc.content, i), "action", "block {i}");
        }
    }

    #[test]
    fn unknown_type_folds_to_action_and_counts() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Cast List"><Text>JOHN, MARY</Text></Paragraph>
</Content></FinalDraft>"#;
        let (doc, summary) = parse_fdx(input).unwrap();
        assert_eq!(node_type(&doc.content, 0), "action");
        assert_eq!(summary.unknown_types_folded_to_action, 1);
    }

    // ─── Inline marks ────────────────────────────────────────────────

    #[test]
    fn bold_italic_underline_runs_become_marks() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Action">
    <Text>Plain </Text>
    <Text Style="Bold">bold</Text>
    <Text> </Text>
    <Text Style="Italic">italic</Text>
    <Text> </Text>
    <Text Style="Bold+Italic+Underline">all three</Text>
    <Text>.</Text>
  </Paragraph>
</Content></FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        assert_eq!(node_text(&doc.content, 0), "Plain bold italic all three.");
        assert!(
            node_marks(&doc.content, 0, 0).is_empty(),
            "plain run no marks"
        );
        assert_eq!(node_marks(&doc.content, 0, 1), vec!["bold"]);
        assert_eq!(node_marks(&doc.content, 0, 3), vec!["italic"]);
        let combined = node_marks(&doc.content, 0, 5);
        assert!(combined.contains(&"bold".to_string()));
        assert!(combined.contains(&"italic".to_string()));
        assert!(combined.contains(&"underline".to_string()));
    }

    #[test]
    fn unknown_styles_are_dropped_silently() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Action">
    <Text Style="Strikeout+Highlight+AllCaps">styled</Text>
  </Paragraph>
</Content></FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        assert!(node_marks(&doc.content, 0, 0).is_empty());
        assert_eq!(node_text(&doc.content, 0), "styled");
    }

    // ─── Auto-uppercase on render ────────────────────────────────────

    #[test]
    fn scene_heading_character_transition_auto_uppercase() {
        // Real FDX often stores these mixed-case; FD uppercases on
        // render. Scriptty's editor expects them already uppercased.
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Scene Heading"><Text>int. house - day</Text></Paragraph>
  <Paragraph Type="Character"><Text>john</Text></Paragraph>
  <Paragraph Type="Transition"><Text>cut to:</Text></Paragraph>
</Content></FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        assert_eq!(node_text(&doc.content, 0), "INT. HOUSE - DAY");
        assert_eq!(node_text(&doc.content, 1), "JOHN");
        assert_eq!(node_text(&doc.content, 2), "CUT TO:");
    }

    #[test]
    fn action_does_not_auto_uppercase() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Action"><Text>John walks in calmly.</Text></Paragraph>
</Content></FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        assert_eq!(node_text(&doc.content, 0), "John walks in calmly.");
    }

    // ─── Dual dialogue ───────────────────────────────────────────────

    #[test]
    fn dual_dialogue_collapses_to_sequential_pairs() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph>
    <DualDialogue>
      <Paragraph Type="Character"><Text>ALICE</Text></Paragraph>
      <Paragraph Type="Dialogue"><Text>I'm leaving.</Text></Paragraph>
      <Paragraph Type="Character"><Text>BOB</Text></Paragraph>
      <Paragraph Type="Dialogue"><Text>Don't.</Text></Paragraph>
    </DualDialogue>
  </Paragraph>
</Content></FinalDraft>"#;
        let (doc, summary) = parse_fdx(input).unwrap();
        assert_eq!(summary.dual_dialogue_count, 1);
        assert_eq!(node_count(&doc.content), 4);
        assert_eq!(node_type(&doc.content, 0), "character");
        assert_eq!(node_text(&doc.content, 0), "ALICE");
        assert_eq!(node_type(&doc.content, 1), "dialogue");
        assert_eq!(node_type(&doc.content, 2), "character");
        assert_eq!(node_text(&doc.content, 2), "BOB");
        assert_eq!(node_type(&doc.content, 3), "dialogue");
    }

    // ─── Drops ───────────────────────────────────────────────────────

    #[test]
    fn locked_scene_numbers_are_dropped_and_counted() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Scene Heading" Number="1A"><Text>INT. HOUSE - DAY</Text></Paragraph>
  <Paragraph Type="Scene Heading" Number="2"><Text>EXT. STREET - NIGHT</Text></Paragraph>
  <Paragraph Type="Scene Heading"><Text>INT. CAR - DAY</Text></Paragraph>
</Content></FinalDraft>"#;
        let (_doc, summary) = parse_fdx(input).unwrap();
        assert_eq!(summary.locked_scene_numbers_dropped, 2);
    }

    #[test]
    fn script_notes_are_dropped_and_counted() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Action">
    <Text>John walks in.</Text>
    <ScriptNote><Paragraph><Text>Cast Mary as the lead.</Text></Paragraph></ScriptNote>
  </Paragraph>
</Content>
<ScriptNotes>
  <ScriptNote Range="0,5"><Paragraph><Text>Top-level note.</Text></Paragraph></ScriptNote>
</ScriptNotes>
</FinalDraft>"#;
        let (doc, summary) = parse_fdx(input).unwrap();
        assert!(summary.script_notes_dropped >= 2);
        // The action text survives without the note's text mixed in.
        assert_eq!(node_text(&doc.content, 0), "John walks in.");
        assert!(!node_text(&doc.content, 0).contains("Cast Mary"));
    }

    #[test]
    fn revisions_are_counted_not_preserved() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Action">
    <Text>Original.</Text>
    <Text RevisionID="1"> Blue rev.</Text>
    <Text RevisionID="2"> Pink rev.</Text>
  </Paragraph>
</Content></FinalDraft>"#;
        let (doc, summary) = parse_fdx(input).unwrap();
        assert_eq!(summary.revisions_dropped, 2);
        // The revision text is still preserved as plain content — only
        // the revision attribution is dropped.
        assert_eq!(node_text(&doc.content, 0), "Original. Blue rev. Pink rev.");
    }

    #[test]
    fn tag_data_is_dropped() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Action"><Text>Plain action.</Text></Paragraph>
</Content>
<TagData><Tag Number="1" Name="Music"/><Tag Number="2" Name="Props"/></TagData>
</FinalDraft>"#;
        let (_doc, summary) = parse_fdx(input).unwrap();
        assert!(summary.tag_data_dropped >= 1);
    }

    // ─── Title page ──────────────────────────────────────────────────

    #[test]
    fn title_page_full_text_lands_in_meta_extra() {
        let input = r#"<FinalDraft Version="3">
<TitlePage><Content>
  <Paragraph Alignment="Center"><Text>THE GREAT SCRIPT</Text></Paragraph>
  <Paragraph Alignment="Center"><Text>Written by</Text></Paragraph>
  <Paragraph Alignment="Center"><Text>Hrishikesh</Text></Paragraph>
  <Paragraph Alignment="Left"><Text>hrishi@example.com</Text></Paragraph>
</Content></TitlePage>
<Content>
  <Paragraph Type="Scene Heading"><Text>INT. HOUSE - DAY</Text></Paragraph>
</Content>
</FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        let dump = doc
            .meta
            .extra
            .get("fdx_title_page")
            .cloned()
            .unwrap_or_default();
        assert!(dump.contains("THE GREAT SCRIPT"));
        assert!(dump.contains("Hrishikesh"));
    }

    #[test]
    fn title_page_heuristic_picks_title_and_author() {
        let input = r#"<FinalDraft Version="3">
<TitlePage><Content>
  <Paragraph Alignment="Center"><Text>THE GREAT SCRIPT</Text></Paragraph>
  <Paragraph Alignment="Center"><Text></Text></Paragraph>
  <Paragraph Alignment="Center"><Text>Written by</Text></Paragraph>
  <Paragraph Alignment="Center"><Text>Hrishikesh</Text></Paragraph>
  <Paragraph Alignment="Left"><Text>hrishi@example.com</Text></Paragraph>
</Content></TitlePage>
<Content>
  <Paragraph Type="Scene Heading"><Text>INT. HOUSE - DAY</Text></Paragraph>
</Content>
</FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        assert_eq!(doc.meta.title, "THE GREAT SCRIPT");
        assert_eq!(doc.meta.author, "Hrishikesh");
        assert!(doc.meta.contact.contains("hrishi@example.com"));
    }

    #[test]
    fn title_page_heuristic_falls_back_to_longest_centred_when_no_cue() {
        let input = r#"<FinalDraft Version="3">
<TitlePage><Content>
  <Paragraph Alignment="Center"><Text>SHORT TITLE</Text></Paragraph>
  <Paragraph Alignment="Center"><Text>The Slightly Longer Author Name</Text></Paragraph>
</Content></TitlePage>
<Content/>
</FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        // The longer line is detected as title because we sort by length.
        // Author falls back to the next-longest centred line.
        assert!(!doc.meta.title.is_empty());
        assert!(!doc.meta.author.is_empty());
        assert_ne!(doc.meta.title, doc.meta.author);
    }

    // ─── Encoding / entities ─────────────────────────────────────────

    #[test]
    fn xml_entities_decode_in_text() {
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Action"><Text>Tom &amp; Jerry &lt;cat &amp; mouse&gt;</Text></Paragraph>
</Content></FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        assert_eq!(node_text(&doc.content, 0), "Tom & Jerry <cat & mouse>");
    }

    #[test]
    fn malayalam_text_passes_through() {
        // FDX is type-explicit, so Malayalam doesn't need the @-prefix
        // gymnastics Fountain forces on us.
        let input = r#"<FinalDraft Version="3"><Content>
  <Paragraph Type="Character"><Text>രമേശ്</Text></Paragraph>
  <Paragraph Type="Dialogue"><Text>നമസ്കാരം</Text></Paragraph>
</Content></FinalDraft>"#;
        let (doc, _) = parse_fdx(input).unwrap();
        assert_eq!(node_type(&doc.content, 0), "character");
        // Malayalam is caseless — to_uppercase() returns the same text.
        assert_eq!(node_text(&doc.content, 0), "രമേശ്");
        assert_eq!(node_text(&doc.content, 1), "നമസ്കാരം");
    }

    // ─── Smoke ───────────────────────────────────────────────────────

    #[test]
    fn full_screenplay_smoke() {
        let input = r#"<?xml version="1.0" encoding="UTF-8" standalone="no" ?>
<FinalDraft DocumentType="Script" Template="No" Version="5">
<Content>
  <Paragraph Type="Scene Heading"><Text>INT. OFFICE - DAY</Text></Paragraph>
  <Paragraph Type="Action"><Text>The room is empty.</Text></Paragraph>
  <Paragraph Type="Character"><Text>JOHN</Text></Paragraph>
  <Paragraph Type="Parenthetical"><Text>(quietly)</Text></Paragraph>
  <Paragraph Type="Dialogue"><Text>Anyone here?</Text></Paragraph>
  <Paragraph Type="Transition"><Text>CUT TO:</Text></Paragraph>
  <Paragraph Type="Scene Heading"><Text>EXT. STREET - NIGHT</Text></Paragraph>
  <Paragraph Type="Action"><Text>Rain falls.</Text></Paragraph>
</Content>
</FinalDraft>"#;
        let (doc, summary) = parse_fdx(input).unwrap();
        assert_eq!(node_count(&doc.content), 8);
        assert_eq!(summary.dual_dialogue_count, 0);
        assert_eq!(summary.script_notes_dropped, 0);
        assert_eq!(summary.locked_scene_numbers_dropped, 0);
    }
}
