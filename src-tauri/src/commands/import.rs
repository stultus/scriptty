// Tauri commands for importing .fountain files (issue #187).
//
// Two entry points:
//   - `import_fountain_as_film` reads a .fountain file from disk and
//     returns a fresh Film-shaped ScreenplayDocument. The frontend uses
//     this both for the File → Import Fountain menu item and for files
//     opened via the standard Open dialog with a .fountain extension.
//   - `import_fountain_as_episode` parses a .fountain file as a single
//     episode and appends it to an existing Series document, returning
//     the mutated series. Only meaningful when the frontend already has
//     a Series open.
//
// Both commands return the parsed document **and** the import summary
// (counts + warnings) so the frontend can render an informational toast.

use crate::screenplay::document::{Episode, EpisodeStatus, ProjectType, ScreenplayDocument};
use crate::screenplay::fountain_import::{parse_fountain, ImportSummary};

/// Bundle returned by both import commands. We use a named struct (not a
/// tuple) so the frontend's TypeScript binding stays self-documenting —
/// `result.document` reads better than `result[0]`.
#[derive(serde::Serialize)]
pub struct FountainImportResult {
    pub document: ScreenplayDocument,
    pub summary: ImportSummary,
}

/// Read a .fountain file from disk and parse it into a Film document.
///
/// Errors surface as user-facing strings: file-system errors include the
/// path so the writer knows which file failed; parse errors carry the
/// reason verbatim from `parse_fountain`.
#[tauri::command]
pub fn import_fountain_as_film(path: String) -> Result<FountainImportResult, String> {
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read Fountain file '{}': {}", path, e))?;
    let (document, summary) = parse_fountain(&text)?;
    Ok(FountainImportResult { document, summary })
}

/// Read a .fountain file and append it as a new episode to the Series
/// document the frontend hands us. Returns the mutated series so the
/// frontend can swap it into its store with a single state update.
///
/// The frontend is responsible for ensuring `current_document.type` is
/// `Series` before calling this — we still defend against misuse with
/// a clear error message.
#[tauri::command]
pub fn import_fountain_as_episode(
    path: String,
    mut current_document: ScreenplayDocument,
) -> Result<FountainImportResult, String> {
    if current_document.project_type != ProjectType::Series {
        return Err("Cannot import a Fountain file as an episode unless a Series is open.".into());
    }

    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read Fountain file '{}': {}", path, e))?;
    let (parsed, summary) = parse_fountain(&text)?;

    // Mint a new episode from the parsed document. UUID generation
    // happens in the frontend ordinarily; we use a millisecond timestamp
    // here to keep this command dependency-free (the frontend can swap
    // the id for a real UUID before save if desired, since `id` is
    // opaque-string typed).
    let new_id = format!(
        "ep-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );

    // Pick a default title if the parsed document didn't bring one. We
    // don't try to derive it from the filename here — the frontend has
    // the original path and can rename in a follow-up step if the writer
    // wants; falling back to "Imported episode" surfaces the import
    // visibly in the navigator.
    let title = if parsed.meta.title.is_empty() {
        "Imported episode".to_string()
    } else {
        parsed.meta.title.clone()
    };

    let series = current_document
        .series
        .as_mut()
        .ok_or_else(|| "Series document missing series payload.".to_string())?;

    let next_number = series.episodes.last().map(|e| e.number + 1).unwrap_or(1);

    series.episodes.push(Episode {
        id: new_id,
        number: next_number,
        title,
        status: EpisodeStatus::Outline,
        content: parsed.content,
        meta: parsed.meta,
        settings: parsed.settings,
        story: parsed.story,
        scene_cards: parsed.scene_cards,
    });

    Ok(FountainImportResult {
        document: current_document,
        summary,
    })
}
