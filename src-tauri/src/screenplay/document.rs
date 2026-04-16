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
    /// The writer's name (or names — use "&" for writing teams, "and" for sequential writers)
    pub author: String,
    /// The director's name. Uses `default` so old files without this field still load.
    #[serde(default)]
    pub director: String,
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
            director: String::new(),
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
    /// Starting scene number — useful when co-writing and this file covers
    /// a specific range of scenes (e.g. 34–44). Defaults to 1.
    /// Uses `default` so old .screenplay files without this field still load.
    #[serde(default = "default_scene_number_start")]
    pub scene_number_start: u32,
    /// When true, the editor shows an auto-generated "characters: …" line
    /// below each scene heading listing every character who speaks in that
    /// scene. Defaults to false so existing files behave exactly as before.
    #[serde(default)]
    pub show_characters_below_header: bool,
}

/// Default value for `scene_number_start` — returns 1 so scenes start at 1
/// when the field is missing from an old .screenplay file.
fn default_scene_number_start() -> u32 {
    1
}

impl Default for ScreenplaySettings {
    fn default() -> Self {
        Self {
            font: "manjari".to_string(),
            default_language: "malayalam".to_string(),
            input_scheme: "mozhi".to_string(),
            scene_number_start: 1,
            show_characters_below_header: false,
        }
    }
}

/// Story development sections — Idea, Synopsis, Treatment, and Narrative.
///
/// Stored in the `"story"` key of a `.screenplay` file.
/// These are plain text sections the writer uses to develop the story
/// before and during scripting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenplayStory {
    /// One to three lines — the core premise / elevator pitch
    pub idea: String,
    /// A few paragraphs — the full story in prose (300–800 words typical)
    pub synopsis: String,
    /// Full narrative prose — scene-by-scene treatment (2,000–10,000+ words)
    pub treatment: String,
    /// Full-length story text — an independent long-form narrative.
    /// Uses `default` so old files without this field still load.
    #[serde(default)]
    pub narrative: String,
}

impl Default for ScreenplayStory {
    fn default() -> Self {
        Self {
            idea: String::new(),
            synopsis: String::new(),
            treatment: String::new(),
            narrative: String::new(),
        }
    }
}

/// Per-scene breakdown card for shoot planning.
///
/// Stored in the `"scene_cards"` array of a `.screenplay` file.
/// Auto-populated fields (location, time, characters) are derived at runtime
/// from the screenplay content — only manually-written fields are stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneCard {
    /// Matches scene position in the script (0-based index)
    pub scene_index: usize,
    /// Short scene description written by the writer (2–4 lines)
    pub description: String,
    /// Shoot notes — equipment, stunts, VFX flags, location notes
    pub shoot_notes: String,
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
    /// Story development sections (idea, synopsis, treatment).
    /// Uses `default` so old .screenplay files without this field still load.
    #[serde(default)]
    pub story: ScreenplayStory,
    /// Per-scene breakdown cards (description, shoot notes).
    /// Uses `default` so old .screenplay files without this field still load.
    #[serde(default)]
    pub scene_cards: Vec<SceneCard>,
}
