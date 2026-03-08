// .screenplay JSON schema: content, meta, and settings structs with serde serialization

// `use` brings these traits into scope so we can derive them on our structs.
// Serialize/Deserialize let us convert structs to/from JSON automatically.
use serde::{Deserialize, Serialize};

/// Metadata about the screenplay — title, author info, and draft tracking.
///
/// Stored in the `"meta"` key of a `.screenplay` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenplayMeta {
    /// The title of the screenplay
    pub title: String,
    /// The author's name
    pub author: String,
    /// Contact information (email, phone, agent, etc.)
    pub contact: String,
    /// Draft revision number, starting at 1
    pub draft_number: u32,
    /// Human-readable date string for this draft (e.g. "2026-03-08")
    pub draft_date: String,
    /// ISO timestamp of when the document was first created
    pub created_at: String,
    /// ISO timestamp of the most recent save
    pub updated_at: String,
}

/// `impl Default` lets us create a ScreenplayMeta with sensible starting values
/// by calling `ScreenplayMeta::default()`.
impl Default for ScreenplayMeta {
    fn default() -> Self {
        // `Self` refers to the type we're implementing Default for (ScreenplayMeta)
        Self {
            title: String::new(),
            author: String::new(),
            contact: String::new(),
            draft_number: 1,
            draft_date: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

/// User-level settings stored alongside the screenplay document.
///
/// Stored in the `"settings"` key of a `.screenplay` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenplaySettings {
    /// Which bundled font to use (e.g. "noto-sans-malayalam" or "manjari")
    pub font: String,
    /// Default writing language: "malayalam" or "english"
    pub default_language: String,
    /// Input scheme for Malayalam: "mozhi", "inscript1", or "inscript2"
    pub input_scheme: String,
}

impl Default for ScreenplaySettings {
    fn default() -> Self {
        Self {
            font: "noto-sans-malayalam".to_string(),
            default_language: "malayalam".to_string(),
            input_scheme: "mozhi".to_string(),
        }
    }
}

/// The complete `.screenplay` document — the top-level JSON structure.
///
/// The `content` field holds the ProseMirror editor state as arbitrary JSON.
/// We use `serde_json::Value` here because ProseMirror's document format is
/// a deeply nested JSON tree whose exact shape is defined by the frontend
/// schema, not by Rust types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenplayDocument {
    /// The ProseMirror document JSON — an arbitrary JSON value
    pub content: serde_json::Value,
    /// Screenplay metadata (title, author, draft info)
    pub meta: ScreenplayMeta,
    /// User-level settings (font, language, input scheme)
    pub settings: ScreenplaySettings,
}
