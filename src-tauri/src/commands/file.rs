// Tauri commands for saving and opening .screenplay files

use crate::screenplay::document::ScreenplayDocument;

/// Creates a brand-new screenplay with default metadata and settings.
///
/// The `content` field is set to `serde_json::Value::Null` because no editor
/// content exists yet — the frontend will populate it once the user starts writing.
///
/// `#[tauri::command]` marks this function as callable from the frontend via `invoke()`.
#[tauri::command]
pub fn new_screenplay() -> Result<ScreenplayDocument, String> {
    Ok(ScreenplayDocument {
        content: serde_json::Value::Null,
        meta: Default::default(),       // Uses the Default impl we defined
        settings: Default::default(),
    })
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
