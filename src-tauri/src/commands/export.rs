// Tauri commands for PDF, Fountain, and plain text export
//
// These commands are called from the Svelte frontend via `invoke()`.
// Each command receives a ScreenplayDocument, processes it, and returns
// the exported data or an error string.

use crate::fonts;
use crate::screenplay::document::ScreenplayDocument;
use crate::screenplay::fountain;
use crate::screenplay::plaintext;
use crate::screenplay::pdf;
use serde::Deserialize;

/// Generates Typst markup from a screenplay document.
///
/// Returns the Typst markup string for debugging and preview purposes.
/// This is useful during development while PDF compilation is being implemented,
/// and may also be useful for advanced users who want to customize the Typst source.
///
/// # Arguments
///
/// * `document` — The full `.screenplay` document, deserialized from JSON by Tauri.
///
/// # Returns
///
/// * `Ok(String)` — The generated Typst markup source code.
/// * `Err(String)` — An error message if markup generation fails.
#[tauri::command]
pub fn export_typst_markup(document: ScreenplayDocument) -> Result<String, String> {
    // Map the font setting slug to the human-readable font name that Typst expects.
    // The document stores a slug like "manjari", but Typst needs the actual font
    // family name like "Manjari" to match against the embedded font data.
    let font_name = match document.settings.font.as_str() {
        "manjari" => "Manjari",
        _ => "Noto Sans Malayalam", // Default font if unrecognized
    };

    // `&document.content` passes a reference (borrow) to the content field.
    // We don't need to take ownership — we just need to read the JSON.
    // `&document.meta` passes a reference to the metadata so the markup generator
    // can include a title page if the screenplay has a title set.
    Ok(pdf::generate_typst_markup(&document.content, font_name, &document.meta))
}

/// Exports a screenplay document as PDF bytes.
///
/// The document's ProseMirror JSON content is converted to Typst markup,
/// then compiled to PDF with the selected font embedded. The PDF bytes
/// are returned to the frontend, which can then save them to disk.
///
/// # Arguments
///
/// * `document` — The full `.screenplay` document, deserialized from JSON by Tauri.
///
/// # Returns
///
/// * `Ok(Vec<u8>)` — The raw PDF file bytes ready to write to disk.
/// * `Err(String)` — An error message if PDF generation fails.
#[tauri::command]
pub fn export_pdf(document: ScreenplayDocument) -> Result<Vec<u8>, String> {
    // `bundled_fonts()` returns a Vec<BundledFont> — all fonts compiled into the binary.
    let bundled = fonts::bundled_fonts();

    // Determine which font to use based on the document's settings.
    // We need both the Typst font name and the matching BundledFont struct.
    let (font_name, font) = match document.settings.font.as_str() {
        "manjari" => (
            "Manjari",
            // `iter().find()` searches through the vector for the first matching item.
            // It returns `Option<&BundledFont>` — Some if found, None if not.
            bundled.iter().find(|f| f.name == "Manjari"),
        ),
        _ => (
            "Noto Sans Malayalam",
            bundled.iter().find(|f| f.name == "Noto Sans Malayalam"),
        ),
    };

    // `ok_or_else` converts an Option to a Result:
    // Some(value) → Ok(value), None → Err(the error string we provide).
    // The `?` at the end means: if this is Err, return that error immediately
    // from the whole function. This is Rust's error propagation operator.
    let font = font.ok_or_else(|| "Selected font not found in bundled fonts".to_string())?;

    // Build the FontData struct that pdf::generate_pdf expects.
    // `font.regular` and `font.bold` are `&'static [u8]` — static byte slices
    // that live for the entire program because they were embedded at compile time.
    let font_data = pdf::FontData {
        regular: font.regular,
        bold: font.bold,
    };

    // Pass `&document.meta` so the PDF includes a title page when metadata is present.
    pdf::generate_pdf(&document.content, font_name, &font_data, &document.meta)
}

/// Exports a screenplay document as PDF bytes in Indian two-column format.
///
/// Indian format places visuals/action in the left column (58%) and
/// audio/dialogue in the right column (42%). Scene headings span full width.
/// This is the standard format used by Indian film industries including Malayalam cinema.
///
/// The font resolution logic is identical to `export_pdf()` — the same bundled fonts
/// are used, just with a different page layout.
///
/// # Arguments
///
/// * `document` — The full `.screenplay` document, deserialized from JSON by Tauri.
///
/// # Returns
///
/// * `Ok(Vec<u8>)` — The raw PDF file bytes in Indian two-column format, ready to write to disk.
/// * `Err(String)` — An error message if PDF generation fails.
#[tauri::command]
pub fn export_pdf_indian(document: ScreenplayDocument) -> Result<Vec<u8>, String> {
    // `bundled_fonts()` returns all fonts compiled into the binary as a Vec<BundledFont>.
    let bundled = fonts::bundled_fonts();

    // Determine which font to use based on the document's settings.
    // This logic is the same as `export_pdf()` — map the slug to a font name
    // and find the matching BundledFont struct.
    let (font_name, font) = match document.settings.font.as_str() {
        "manjari" => (
            "Manjari",
            // `iter().find()` searches the vector for the first item where the
            // closure returns true. Returns `Option<&BundledFont>`.
            bundled.iter().find(|f| f.name == "Manjari"),
        ),
        _ => (
            "Noto Sans Malayalam",
            bundled.iter().find(|f| f.name == "Noto Sans Malayalam"),
        ),
    };

    // `ok_or_else` converts Option to Result: Some(val) → Ok(val), None → Err(...).
    // The `?` operator propagates the error — if the font isn't found, the function
    // returns early with this error message.
    let font = font.ok_or_else(|| "Selected font not found in bundled fonts".to_string())?;

    // Build the FontData struct with regular and bold font byte slices.
    // These are `&'static [u8]` — static references to font bytes embedded in the binary.
    let font_data = pdf::FontData {
        regular: font.regular,
        bold: font.bold,
    };

    // Call the Indian two-column PDF generator instead of the Hollywood one.
    // Pass `&document.meta` so the Indian format PDF also includes a title page.
    pdf::generate_pdf_indian(&document.content, font_name, &font_data, &document.meta)
}

/// Options for the combined PDF export — specifies which sections to include
/// and which screenplay format to use.
///
/// This struct is deserialized from the frontend's JSON payload. Each boolean
/// field corresponds to a checkbox in the Export modal.
#[derive(Deserialize)]
pub struct ExportOptions {
    /// Include the title page (from metadata)
    pub include_title_page: bool,
    /// Include the synopsis section (from story.synopsis)
    pub include_synopsis: bool,
    /// Include the treatment section (from story.treatment)
    pub include_treatment: bool,
    /// Include the screenplay content
    pub include_screenplay: bool,
    /// Include scene cards breakdown
    pub include_scene_cards: bool,
    /// Screenplay format: "hollywood" or "indian"
    pub format: String,
    /// Pre-computed scene cards data as JSON (auto-populated fields computed by frontend)
    pub scene_cards_data: serde_json::Value,
}

/// Combined PDF export — generates a single PDF with user-selected sections.
///
/// The frontend sends the document and an ExportOptions struct specifying which
/// sections to include. The Typst markup for each section is concatenated into
/// a single document and compiled to PDF.
///
/// # Arguments
/// * `document` — The full screenplay document
/// * `options` — Export options specifying which sections to include
///
/// # Returns
/// * `Ok(Vec<u8>)` — The combined PDF bytes
/// * `Err(String)` — Error if PDF generation fails
#[tauri::command]
pub fn export_combined_pdf(
    document: ScreenplayDocument,
    options: ExportOptions,
) -> Result<Vec<u8>, String> {
    let bundled = fonts::bundled_fonts();

    // Resolve the font — same logic as the other export commands
    let (font_name, font) = match document.settings.font.as_str() {
        "manjari" => (
            "Manjari",
            bundled.iter().find(|f| f.name == "Manjari"),
        ),
        _ => (
            "Noto Sans Malayalam",
            bundled.iter().find(|f| f.name == "Noto Sans Malayalam"),
        ),
    };

    let font = font.ok_or_else(|| "Selected font not found in bundled fonts".to_string())?;
    let font_data = pdf::FontData {
        regular: font.regular,
        bold: font.bold,
    };

    // Build the combined Typst markup by conditionally including each section.
    // We start with the screenplay markup (which includes the preamble/page setup),
    // then append additional sections as needed.

    let mut markup = String::new();

    // If we're including the screenplay, use the appropriate format generator
    // which already includes the Typst preamble and optionally the title page.
    if options.include_screenplay {
        // Create a meta that may or may not include title page based on the option
        let meta_for_export = if options.include_title_page {
            document.meta.clone()
        } else {
            // Empty meta = no title page
            Default::default()
        };

        markup = if options.format == "indian" {
            pdf::generate_indian_markup(&document.content, font_name, &meta_for_export)
        } else {
            pdf::generate_typst_markup(&document.content, font_name, &meta_for_export)
        };
    } else {
        // No screenplay — we still need a Typst preamble for the prose sections.
        // Generate a minimal preamble with just page setup and font.
        markup.push_str(&format!(
            r#"#set page(paper: "a4", margin: (top: 2.5cm, bottom: 2.5cm, left: 2.5cm, right: 2.5cm))
#set text(font: "{}", size: 12pt)
#set par(leading: 0.8em)
"#,
            font_name
        ));

        // If title page is requested without screenplay, generate a standalone title page
        if options.include_title_page && !document.meta.title.is_empty() {
            markup.push_str(&pdf::generate_title_page_markup(&document.meta));
        }
    }

    // Append synopsis section if requested
    if options.include_synopsis && !document.story.synopsis.is_empty() {
        markup.push_str(&pdf::generate_prose_section_markup(
            "SYNOPSIS",
            &document.story.synopsis,
            font_name,
            &document.meta.author,
        ));
    }

    // Append treatment section if requested
    if options.include_treatment && !document.story.treatment.is_empty() {
        markup.push_str(&pdf::generate_prose_section_markup(
            "TREATMENT",
            &document.story.treatment,
            font_name,
            &document.meta.author,
        ));
    }

    // Append scene cards section if requested
    if options.include_scene_cards {
        markup.push_str(&pdf::generate_scene_cards_markup(
            &options.scene_cards_data,
            font_name,
        ));
    }

    // Compile the combined markup to PDF
    pdf::compile_markup_to_pdf(&markup, &font_data)
}

/// Exports a screenplay document as formatted plain text.
///
/// Produces a readable screenplay with proper indentation:
/// character names at column 40, dialogue at column 25 with 35-char wrapping,
/// parentheticals at column 35, transitions right-aligned, scene headings uppercase.
///
/// # Arguments
/// * `document` — The full `.screenplay` document, deserialized from JSON by Tauri.
///
/// # Returns
/// * `Ok(String)` — The formatted plain text screenplay.
/// * `Err(String)` — An error message if conversion fails.
#[tauri::command]
pub fn export_plaintext(document: ScreenplayDocument) -> Result<String, String> {
    Ok(plaintext::generate_plaintext(&document.content, &document.meta))
}

/// Exports a screenplay document as a Fountain plain-text string.
///
/// Fountain is an open plain-text screenwriting format (fountain.io) that can
/// be read by Highland, Fade In, and other screenwriting tools. The output is
/// UTF-8 encoded, preserving Malayalam text as-is.
///
/// # Arguments
/// * `document` — The full `.screenplay` document, deserialized from JSON by Tauri.
///
/// # Returns
/// * `Ok(String)` — The Fountain-formatted screenplay text.
/// * `Err(String)` — An error message if conversion fails.
#[tauri::command]
pub fn export_fountain(document: ScreenplayDocument) -> Result<String, String> {
    Ok(fountain::generate_fountain(&document.content, &document.meta))
}
