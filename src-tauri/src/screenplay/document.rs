// .screenplay JSON schema: content, meta, and settings structs with serde serialization

// `use` brings these traits into scope so we can derive them on our structs.
// Serialize/Deserialize let us convert structs to/from JSON automatically.
use serde::{Deserialize, Serialize};

/// Metadata about the screenplay — title, author info, and draft tracking.
///
/// Stored in the `"meta"` key of a `.screenplay` file.
/// Every string field is `#[serde(default)]` so slim-format or
/// hand-authored `.screenplay` files (including series episodes, which
/// typically omit timestamps) load without a hard parse error. Missing
/// fields become empty strings; the UI treats empty as "not set".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenplayMeta {
    /// The title of the screenplay
    #[serde(default)]
    pub title: String,
    /// The writer's name (or names — use "&" for writing teams, "and" for sequential writers)
    #[serde(default)]
    pub author: String,
    /// The director's name. Uses `default` so old files without this field still load.
    #[serde(default)]
    pub director: String,
    /// A one-line tagline / logline rendered under the title on the title page.
    /// Optional — empty string means "no tagline", and old `.screenplay` files
    /// without this field load with it blank (issue #14).
    #[serde(default)]
    pub tagline: String,
    /// Registration / copyright identifier (e.g. WGA or film-board registration)
    /// shown on the title page alongside contact info. Optional.
    #[serde(default)]
    pub registration_number: String,
    /// A short note printed at the bottom of the title page — typically a
    /// confidentiality line, a "based on" credit, or a dedication. Optional.
    #[serde(default)]
    pub footnote: String,
    /// Contact information (email, phone, agent, etc.)
    #[serde(default)]
    pub contact: String,
    /// Draft revision number. Defaults to 1 via `default_draft_number`
    /// because bare `u32::default()` is 0, and a "Draft 0" label is
    /// surprising on files that simply omit the field.
    #[serde(default = "default_draft_number")]
    pub draft_number: u32,
    /// Human-readable date string for this draft (e.g. "2026-03-08")
    #[serde(default)]
    pub draft_date: String,
    /// ISO timestamp of when the document was first created
    #[serde(default)]
    pub created_at: String,
    /// ISO timestamp of the most recent save
    #[serde(default)]
    pub updated_at: String,
}

/// Default draft number for `ScreenplayMeta::draft_number` when the field
/// is absent from a `.screenplay` file. We want 1, not the `u32::default()`
/// value of 0, so slim-format files render as "Draft 1".
fn default_draft_number() -> u32 {
    1
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
            tagline: String::new(),
            registration_number: String::new(),
            footnote: String::new(),
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
    /// Which bundled font to use (e.g. "noto-sans-malayalam" or "manjari").
    /// `default` lets slim episode settings (which typically omit default_language)
    /// and any future abbreviated format still load cleanly.
    #[serde(default = "default_font")]
    pub font: String,
    /// Default writing language: "malayalam" or "english"
    #[serde(default = "default_language")]
    pub default_language: String,
    /// Input scheme for Malayalam: "mozhi", "inscript1", or "inscript2"
    #[serde(default = "default_input_scheme")]
    pub input_scheme: String,
    /// Starting scene number — useful when co-writing and this file covers
    /// a specific range of scenes (e.g. 34–44). Defaults to 1.
    /// Uses `default` so old .screenplay files without this field still load.
    /// A custom `deserialize_with` clamps the stored value to 1..=9999 so a
    /// hand-edited 0, negative cast, or absurdly large value can't make the
    /// PDF pipeline's `saturating_sub` silently drop scene character lists.
    #[serde(default = "default_scene_number_start", deserialize_with = "deserialize_scene_number_start")]
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

/// Default font when `.screenplay` files omit `settings.font`. Matches
/// `ScreenplaySettings::default()` so all load paths agree.
fn default_font() -> String {
    "manjari".to_string()
}

/// Default writing language when `.screenplay` files omit
/// `settings.default_language`. Matches `ScreenplaySettings::default()`.
fn default_language() -> String {
    "malayalam".to_string()
}

/// Default input scheme when `.screenplay` files omit
/// `settings.input_scheme`. Matches `ScreenplaySettings::default()`.
fn default_input_scheme() -> String {
    "mozhi".to_string()
}

/// Clamp `scene_number_start` to a sane range on deserialize.
///
/// The upper bound (9999) is well above anything a real screenplay needs and
/// keeps downstream index math comfortably below `u32::MAX`. The lower bound
/// (1) guarantees the scene counter never pre-decrements into overflow via
/// the `scene_number_start - 1` expression used in the PDF pipeline.
/// `deserialize_with` is serde's hook for running custom logic in the middle
/// of field-level deserialize; we accept the raw `u32`, then clamp.
fn deserialize_scene_number_start<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Accept the raw value first, then clamp. If the JSON holds a negative
    // or a number larger than u32 can represent, serde itself errors out —
    // that's fine; the outer load path falls back to defaults on parse error.
    let raw = u32::deserialize(deserializer)?;
    Ok(raw.clamp(1, 9999))
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
    /// Comma-separated list of characters who are physically in the scene but
    /// never have a dialogue line (background extras, silent antagonists). The
    /// character-list collector merges these with auto-detected speakers. Old
    /// .screenplay files don't carry this, hence `#[serde(default)]`.
    #[serde(default)]
    pub extra_characters: String,
}

/// Default ProseMirror document JSON — a single empty scene heading.
///
/// Matches the frontend's `createInitialDoc()` shape so a missing or null
/// `content` field deserializes to something the PDF pipeline can walk
/// (and the editor can render) instead of an empty/broken state.
fn default_content() -> serde_json::Value {
    serde_json::json!({
        "type": "doc",
        "content": [
            { "type": "scene_heading" }
        ]
    })
}

/// Map JSON `null` to `default_content()` while preserving every other
/// value as-is. Serde's own `default` only fires when the field is
/// absent, not when it's present-but-null — which is exactly what
/// `new_screenplay()` historically wrote. This hook covers both cases
/// at the same boundary.
fn deserialize_content<'de, D>(deserializer: D) -> Result<serde_json::Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = serde_json::Value::deserialize(deserializer)?;
    if raw.is_null() {
        Ok(default_content())
    } else {
        Ok(raw)
    }
}

/// Top-level kind of the document. Films are single screenplays (the
/// original and still default shape); Series projects hold multiple
/// episodes, each of which is a complete screenplay of its own.
///
/// Serialized as lowercase JSON strings — `"film"` or `"series"` — so the
/// on-disk format reads naturally. Defaults to `Film` so every existing
/// `.screenplay` file opens unchanged (issue: series support).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Film,
    Series,
}

impl Default for ProjectType {
    fn default() -> Self {
        ProjectType::Film
    }
}

/// One episode inside a Series project. Mirrors the film-level fields so
/// every existing editor feature (scene navigator, export, scene cards,
/// story panel) can operate on an episode without changing its code path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// UUID minted by the frontend when the episode is created. Stable
    /// across renames and reorders so UI state (expanded/collapsed folder,
    /// active selection) survives edits.
    pub id: String,
    /// Display number, typically 1-based and sequential after reorder.
    pub number: u32,
    /// Episode title (e.g. "Pilot", "The Return"). Can be empty.
    pub title: String,
    #[serde(default = "default_content", deserialize_with = "deserialize_content")]
    pub content: serde_json::Value,
    /// Each episode's meta mirrors the film-level meta. `#[serde(default)]`
    /// so authors can write minimal episode blocks (just id/number/title/content)
    /// and still get a valid deserialize.
    #[serde(default)]
    pub meta: ScreenplayMeta,
    /// Same rationale as `meta` — slim episodes can omit settings entirely.
    #[serde(default)]
    pub settings: ScreenplaySettings,
    #[serde(default)]
    pub story: ScreenplayStory,
    #[serde(default)]
    pub scene_cards: Vec<SceneCard>,
}

/// Series-level container. Only present when the document's `project_type`
/// is `Series`; in Film mode the top-level meta/settings/story/content/
/// scene_cards are used directly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeriesData {
    /// The series name (e.g. "The Return"). Shown above the episode list
    /// and used as the series-level title page on full-series exports.
    pub title: String,
    /// Ordered list of episodes. Order is authoritative; `number` tracks
    /// the display number and is resynced by the frontend on reorder.
    pub episodes: Vec<Episode>,
}

/// The complete `.screenplay` document — the top-level JSON structure.
///
/// The `content` field holds the ProseMirror editor state as arbitrary JSON.
/// We use `serde_json::Value` here because ProseMirror's document format is
/// a deeply nested JSON tree whose exact shape is defined by the frontend
/// schema, not by Rust types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenplayDocument {
    /// `"film"` (default) or `"series"`. Films use the top-level meta/
    /// settings/story/content/scene_cards fields directly; series files use
    /// the `series` container and the top-level fields are placeholders.
    #[serde(default, rename = "type")]
    pub project_type: ProjectType,
    /// Series container — only meaningful when `project_type` is `Series`.
    /// Absent on every film file (including all existing ones).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<SeriesData>,
    /// The ProseMirror document JSON — an arbitrary JSON value.
    /// Older `.screenplay` files written before the editor schema stabilized
    /// may omit this field entirely, and `new_screenplay()` historically wrote
    /// an explicit `null` here. The custom `deserialize_with` hook maps both
    /// cases to a minimal doc with one empty scene heading (matching the
    /// editor's `createInitialDoc()` shape), so the PDF pipeline renders a
    /// blank scene instead of an empty body (see issues #44/#45).
    #[serde(default = "default_content", deserialize_with = "deserialize_content")]
    pub content: serde_json::Value,
    /// Screenplay metadata (title, author, draft info).
    /// `#[serde(default)]` so series files — which carry real meta inside
    /// each episode — can omit the top-level block entirely, and so any
    /// future slim-format film exporter can write nothing here without
    /// breaking the loader.
    #[serde(default)]
    pub meta: ScreenplayMeta,
    /// User-level settings (font, language, input scheme).
    /// Same rationale as `meta` — `#[serde(default)]` lets series files drop
    /// the top-level settings block (episodes carry their own).
    #[serde(default)]
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
