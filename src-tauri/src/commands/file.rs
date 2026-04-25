// Tauri commands for saving and opening .screenplay files

use crate::screenplay::document::{ProjectType, ScreenplayDocument};

/// Creates a brand-new screenplay with default metadata and settings.
///
/// `content` is seeded with a minimal ProseMirror doc (one empty scene
/// heading) rather than `Value::Null`, so an export triggered before the
/// editor has written anything still produces a PDF with a blank scene
/// instead of an empty body (see issue #44).
///
/// `#[tauri::command]` marks this function as callable from the frontend via `invoke()`.
#[tauri::command]
pub fn new_screenplay() -> ScreenplayDocument {
    // Infallible: all fields are constructed from defaults or literals, so
    // there's nothing that can fail. Dropping the `Result` wrapper lets the
    // frontend call this without a meaningless try/catch.
    ScreenplayDocument {
        project_type: ProjectType::Film,
        series: None,
        content: serde_json::json!({
            "type": "doc",
            "content": [
                { "type": "scene_heading" }
            ]
        }),
        meta: Default::default(),       // Uses the Default impl we defined
        settings: Default::default(),
        story: Default::default(),      // Empty story sections
        scene_cards: Vec::new(),        // No scene cards initially
    }
}

/// Saves a screenplay document to disk as a JSON file.
///
/// # Arguments
/// * `path` - The file path to write to (e.g. "/Users/hrishi/scripts/movie.screenplay")
/// * `document` - The full screenplay document to serialize
///
/// `serde_json::to_string_pretty` converts the struct to nicely formatted JSON.
/// `.map_err(|e| e.to_string())` converts any error type into a String so it
/// can be returned to the frontend.
#[tauri::command]
pub fn save_screenplay(path: String, document: ScreenplayDocument) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&document)
        .map_err(|e| format!("Failed to serialize document: {}", e))?;

    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write file '{}': {}", path, e))?;

    Ok(())
}

/// Opens a screenplay document from a JSON file on disk.
///
/// # Arguments
/// * `path` - The file path to read from
///
/// `std::fs::read_to_string` reads the entire file into a String.
/// `serde_json::from_str` parses that JSON string back into our struct.
/// The `?` operator is shorthand for "if this is an Err, return early with that error".
#[tauri::command]
pub fn open_screenplay(path: String) -> Result<ScreenplayDocument, String> {
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file '{}': {}", path, e))?;

    // `serde_json::from_str` deserializes the JSON string into a ScreenplayDocument.
    // The turbofish `::<ScreenplayDocument>` tells Rust which type to parse into.
    let document: ScreenplayDocument = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse screenplay file: {}", e))?;

    Ok(document)
}

/// Opens a URL in the system's default browser or mail client.
///
/// Uses Tauri's opener plugin on the Rust side, which bypasses the
/// frontend scope restrictions that can silently block `openUrl()` calls.
///
/// # Arguments
/// * `url` — The URL to open (e.g. "https://stultus.in" or "mailto:hello@stultus.in")
/// * `app` — The Tauri AppHandle, injected automatically by Tauri
#[tauri::command]
pub fn open_external_url(url: String, app: tauri::AppHandle) -> Result<(), String> {
    // `OpenerExt` is a trait that adds the `.opener()` method to AppHandle
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| format!("Failed to open URL '{}': {}", url, e))
}

/// Path of the autosave sidecar for a given .screenplay path.
///
/// `<path>.autosave` is the convention — sits next to the original so it's
/// obvious in Finder/Explorer that the two belong together, and so a
/// crash that takes out the original directory takes out the autosave too
/// (better than orphaning autosaves in a hidden app-data location).
fn autosave_path(path: &str) -> String {
    format!("{}.autosave", path)
}

/// Information returned to the frontend when an autosave sidecar exists
/// and is **newer** than the file the user just opened. Carries enough
/// data for the UI to either restore the autosave content silently or
/// prompt the writer with a Restore/Discard dialog.
#[derive(serde::Serialize)]
pub struct AutosaveInfo {
    /// Full document deserialized from the autosave sidecar — drop-in
    /// replacement for what `open_screenplay` returned.
    pub document: ScreenplayDocument,
    /// Last-modified time of the autosave file, in ms since epoch.
    pub autosave_time_ms: i64,
    /// Last-modified time of the original .screenplay file, in ms since
    /// epoch — lets the UI render "unsaved changes from <duration>".
    pub original_time_ms: i64,
}

/// Write a copy of the document to `<path>.autosave`. Called on a debounced
/// timer from the frontend after every dirty edit. Skips writing when the
/// document hasn't changed (the frontend tracks dirty state and only calls
/// us when it has). Failures are logged from the frontend but don't
/// bubble up as user-facing errors — autosave is best-effort.
#[tauri::command]
pub fn autosave_screenplay(path: String, document: ScreenplayDocument) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&document)
        .map_err(|e| format!("Failed to serialize autosave: {}", e))?;
    let target = autosave_path(&path);
    std::fs::write(&target, json)
        .map_err(|e| format!("Failed to write autosave '{}': {}", target, e))
}

/// Delete the autosave sidecar — called by the frontend after a successful
/// real save, so we don't leave stale recovery candidates lying around.
/// Missing-file is **not** an error: a freshly-saved file may never have
/// had an autosave (saved before the autosave timer fired).
#[tauri::command]
pub fn discard_autosave(path: String) -> Result<(), String> {
    let target = autosave_path(&path);
    match std::fs::remove_file(&target) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Failed to remove autosave '{}': {}", target, e)),
    }
}

/// Check whether `<path>.autosave` exists and is newer than `<path>`. If
/// so, parse and return it. If no autosave exists, or the original is
/// newer (autosave is stale — last save was after the last autosave),
/// returns Ok(None) and the frontend keeps the just-loaded document.
///
/// The freshness check guards against the common case where the user
/// saved normally, the autosave file lingered for a moment, and then a
/// later open shouldn't show a recovery prompt for state the user
/// already committed to disk.
#[tauri::command]
pub fn load_autosave(path: String) -> Result<Option<AutosaveInfo>, String> {
    let target = autosave_path(&path);

    // No sidecar → nothing to recover.
    let autosave_meta = match std::fs::metadata(&target) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Failed to stat autosave '{}': {}", target, e)),
    };

    // If the original is newer (or equal), the autosave is stale — discard
    // it and report "nothing to recover" so the user isn't prompted to
    // restore something older than the file they just opened.
    let original_meta = std::fs::metadata(&path)
        .map_err(|e| format!("Failed to stat '{}': {}", path, e))?;

    let autosave_time = autosave_meta
        .modified()
        .map_err(|e| format!("autosave mtime: {}", e))?;
    let original_time = original_meta
        .modified()
        .map_err(|e| format!("original mtime: {}", e))?;

    if autosave_time <= original_time {
        // Stale — best-effort cleanup; ignore the result.
        let _ = std::fs::remove_file(&target);
        return Ok(None);
    }

    let json = std::fs::read_to_string(&target)
        .map_err(|e| format!("Failed to read autosave '{}': {}", target, e))?;
    let document: ScreenplayDocument = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse autosave '{}': {}", target, e))?;

    Ok(Some(AutosaveInfo {
        document,
        autosave_time_ms: system_time_to_ms(autosave_time),
        original_time_ms: system_time_to_ms(original_time),
    }))
}

/// Convert a SystemTime to ms-since-epoch. Returns 0 on times before the
/// UNIX epoch, which only happens on a machine with a badly-set clock —
/// 0 just means "render no relative time" in the UI rather than panicking.
fn system_time_to_ms(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
