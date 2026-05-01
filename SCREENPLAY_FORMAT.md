# .screenplay File Format Specification

Version: 1.0
App: **Scriptty** — an offline screenwriting application for Malayalam and English writers.

This document defines the JSON structure of `.screenplay` files. Any tool or LLM that produces a valid JSON file conforming to this spec can generate screenplays openable in Scriptty.

---

## Top-Level Structure

A `.screenplay` file is a UTF-8 encoded JSON object with five top-level keys:

```json
{
  "content": { },
  "meta": { },
  "settings": { },
  "story": { },
  "scene_cards": [ ]
}
```

All five keys should be present. Missing keys will be filled with defaults, but for maximum compatibility, include all of them.

---

## 1. `meta` — Project Metadata

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `title` | string | yes | `""` | Screenplay title. Appears on the title page and section headers in PDF exports. |
| `author` | string | yes | `""` | Writer name(s). For multiple writers, use `" & "` for writing teams (e.g. `"Joel & Ethan Coen"`) or `" and "` for sequential writers (e.g. `"Scott Frank and Jon Cohen"`). |
| `director` | string | no | `""` | Director name. If same as author (case-insensitive), the app renders "Written and Directed by". |
| `contact` | string | yes | `""` | Contact info (email, phone, agent). May contain newlines (`\n`). |
| `draft_number` | integer | yes | `1` | Draft revision number, starting at 1. |
| `draft_date` | string | yes | `""` | Human-readable date, e.g. `"2026-03-14"` or `"March 14, 2026"`. |
| `created_at` | string | yes | `""` | ISO 8601 timestamp of document creation, e.g. `"2026-01-15T09:30:00Z"`. |
| `updated_at` | string | yes | `""` | ISO 8601 timestamp of most recent save. |
| `extra` | object | no | `{}` | Non-standard title-page metadata preserved for Fountain round-trip — string-to-string map. Holds Fountain title-page keys that don't map to a first-class meta field (e.g. `Source`, `Copyright`, custom keys). Original key spelling preserved. Omitted from the JSON when empty so legacy files stay byte-clean on resave. |

**Example:**

```json
"meta": {
  "title": "Monsoon Wedding",
  "author": "Sabrina Dhawan",
  "director": "Mira Nair",
  "contact": "sabrina@example.com",
  "draft_number": 3,
  "draft_date": "2026-03-14",
  "created_at": "2025-11-01T10:00:00Z",
  "updated_at": "2026-03-14T18:45:00Z"
}
```

---

## 2. `settings` — Editor Preferences

| Field | Type | Required | Default | Allowed Values |
|---|---|---|---|---|
| `font` | string | yes | `"manjari"` | `"noto-sans-malayalam"`, `"manjari"` |
| `default_language` | string | yes | `"malayalam"` | `"malayalam"`, `"english"` |
| `input_scheme` | string | yes | `"mozhi"` | `"mozhi"`, `"inscript1"`, `"inscript2"` |

These control editor display and input behavior. For LLM-generated files, use the defaults or set `default_language` to match the screenplay's primary language.

**Example:**

```json
"settings": {
  "font": "noto-sans-malayalam",
  "default_language": "malayalam",
  "input_scheme": "mozhi"
}
```

---

## 3. `story` — Development Notes

Pre-writing material stored alongside the screenplay. All fields are plain text (no markup).

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `idea` | string | yes | `""` | Logline or core premise. Typically 1–3 sentences. |
| `synopsis` | string | yes | `""` | Full story arc in prose. Typically 300–800 words. |
| `treatment` | string | yes | `""` | Scene-by-scene narrative breakdown. Can be thousands of words. |
| `narrative` | string | no | `""` | Long-form prose version of the story. Free-form, no length limit. |

These fields are optional in practice — empty strings are valid. They can be included in PDF exports.

**Example:**

```json
"story": {
  "idea": "A retired detective in Kochi receives a letter from a murder victim — posted the day before the killing.",
  "synopsis": "Inspector Menon, forced into early retirement after a corruption scandal...",
  "treatment": "",
  "narrative": ""
}
```

---

## 4. `scene_cards` — Production Notes

An array of scene card objects. Each card stores user-written notes for a specific scene. If no cards have been created, use an empty array `[]`.

| Field | Type | Description |
|---|---|---|
| `scene_index` | integer | Zero-based index matching the scene's position in the screenplay. The first scene heading is index 0, the second is index 1, etc. |
| `description` | string | What happens in the scene (2–4 lines typical). Fountain synopses (`= ...`) imported into Scriptty land here — and they're emitted back as `=` lines on Fountain export, so this field is semi-public. |
| `shoot_notes` | string | Production notes: equipment, VFX, stunts, location details. Also the storage site for Fountain inline notes (`[[ ... ]]`) and section headers attached during import — see the section-marker convention below. |

Scene metadata like location, time of day, characters, and page count are computed automatically from the screenplay content — they are **not** stored in scene cards.

### Fountain section-marker convention

When a Fountain file is imported, section headers (`# Act One`, `## Sequence A`, …) attach to the next scene's `shoot_notes` with an internal marker so the round-trip back to Fountain can re-emit them at the right depth:

```
[[#section depth=1]] Act One
```

Multiple lines (sections + free-form notes + inline `[[ ... ]]` notes) are joined with `\n` in the same field. The Fountain exporter recognises the marker prefix and emits a `# Act One` heading before the scene; lines without the marker emit as inline `[[ ... ]]` notes. Don't author the marker syntax by hand — let the importer manage it.

**Example:**

```json
"scene_cards": [
  {
    "scene_index": 0,
    "description": "Establishing shot. Menon alone in his apartment, reading the letter.",
    "shoot_notes": "Practicals only — desk lamp, TV glow. Handheld camera."
  },
  {
    "scene_index": 1,
    "description": "Flashback to the police station, six months earlier.",
    "shoot_notes": "Period dressing for the station. 30 extras needed."
  }
]
```

---

## 5. `content` — The Screenplay

This is the screenplay itself, stored as a ProseMirror document tree in JSON format.

### Document Root

```json
{
  "type": "doc",
  "content": [
    // Array of screenplay element nodes
  ]
}
```

The `content` array must contain **one or more** block-level element nodes. An empty screenplay has a single empty scene heading:

```json
{
  "type": "doc",
  "content": [
    { "type": "scene_heading" }
  ]
}
```

### Element Nodes

Every screenplay element is a JSON object with this structure:

```json
{
  "type": "<element_type>",
  "content": [
    { "type": "text", "text": "The actual text content" }
  ]
}
```

- `type` — one of the six element types listed below.
- `content` — an array of text nodes. For an empty element, omit the `content` key or use an empty array `[]`.

**Text nodes** are always `{ "type": "text", "text": "..." }`. There are no marks (bold, italic, underline) — all text is plain. Multiple text nodes in a single element are concatenated.

### The Six Element Types

#### `scene_heading`

The slug line that opens each scene. Conventionally written in UPPERCASE.

**Format:** `INT./EXT. LOCATION - TIME OF DAY`

Common prefixes: `INT.`, `EXT.`, `INT./EXT.`, `I/E.`
Common time suffixes: `DAY`, `NIGHT`, `DAWN`, `DUSK`, `CONTINUOUS`, `LATER`, `MOMENTS LATER`

```json
{ "type": "scene_heading", "content": [{ "type": "text", "text": "INT. COFFEE SHOP - DAY" }] }
{ "type": "scene_heading", "content": [{ "type": "text", "text": "EXT. BEACH - NIGHT" }] }
```

The app auto-uppercases text typed into scene headings. When generating files, always write scene headings in UPPERCASE.

#### `action`

Scene description, stage direction, or narrative prose. Written in normal sentence case.

```json
{ "type": "action", "content": [{ "type": "text", "text": "Ramesh walks through the crowded market, scanning faces." }] }
```

For multiple paragraphs of action, use separate `action` nodes — one per paragraph.

#### `character`

The name of the character who is about to speak. Always in UPPERCASE. May include extensions in parentheses.

Common extensions: `(V.O.)` (voice over), `(O.S.)` (off screen), `(O.C.)` (off camera), `(CONT'D)` (continued).

```json
{ "type": "character", "content": [{ "type": "text", "text": "RAMESH" }] }
{ "type": "character", "content": [{ "type": "text", "text": "MEERA (V.O.)" }] }
```

The app auto-uppercases text typed into character elements. When generating files, always write character names in UPPERCASE.

#### `dialogue`

The spoken words of the character named in the preceding `character` element. Written in normal sentence case.

```json
{ "type": "dialogue", "content": [{ "type": "text", "text": "I've been waiting for you." }] }
```

Dialogue can be in any language, including Malayalam Unicode text:

```json
{ "type": "dialogue", "content": [{ "type": "text", "text": "ഞാൻ നിന്നെ കാത്തിരിക്കുകയായിരുന്നു." }] }
```

Mixed-language dialogue is fully supported:

```json
{ "type": "dialogue", "content": [{ "type": "text", "text": "അവൻ flat ലേക്ക് പോയി." }] }
```

#### `parenthetical`

Brief acting direction for the character, placed between the character name and dialogue (or between dialogue lines). Written in lowercase.

**Important:** Do NOT include parentheses in the text content. The app adds them automatically for display and export. Store only the inner text.

```json
{ "type": "parenthetical", "content": [{ "type": "text", "text": "whispering" }] }
{ "type": "parenthetical", "content": [{ "type": "text", "text": "to Meera" }] }
{ "type": "parenthetical", "content": [{ "type": "text", "text": "beat" }] }
```

#### `transition`

Editing instructions, typically right-aligned. Written in UPPERCASE, usually ending with `TO:`.

```json
{ "type": "transition", "content": [{ "type": "text", "text": "CUT TO:" }] }
{ "type": "transition", "content": [{ "type": "text", "text": "FADE OUT." }] }
{ "type": "transition", "content": [{ "type": "text", "text": "SMASH CUT TO:" }] }
```

Use transitions sparingly — modern screenwriting style often omits them.

---

## Element Sequencing Rules

Screenplay elements follow conventional ordering patterns. While the format does not enforce strict sequencing, well-formed screenplays follow these rules:

1. A screenplay typically opens with a `scene_heading` (or optionally a `transition` like `"FADE IN:"`).
2. A `scene_heading` is followed by one or more `action` elements.
3. A dialogue block is a sequence: `character` → optional `parenthetical` → `dialogue`. Multiple rounds of `parenthetical` and `dialogue` can follow a single `character`.
4. After dialogue, the screenplay returns to `action` or starts a new dialogue block with another `character`.
5. A `transition` is typically followed by a `scene_heading`.

**Valid dialogue block patterns:**

```
character → dialogue
character → parenthetical → dialogue
character → dialogue → parenthetical → dialogue
character → parenthetical → dialogue → parenthetical → dialogue
```

**Invalid patterns to avoid:**

- `dialogue` without a preceding `character` — orphaned dialogue
- `parenthetical` without a nearby `character` — orphaned direction
- Two consecutive `character` elements — missing dialogue
- `scene_heading` immediately followed by `character` — usually needs an `action` beat first (though not strictly forbidden)

---

## Complete Example

A short bilingual screenplay demonstrating all element types:

```json
{
  "content": {
    "type": "doc",
    "content": [
      {
        "type": "scene_heading",
        "content": [{ "type": "text", "text": "INT. RAMESH'S APARTMENT - NIGHT" }]
      },
      {
        "type": "action",
        "content": [{ "type": "text", "text": "A small, dimly lit apartment. Books and newspapers are stacked on every surface. RAMESH (50s), unshaven, sits at a desk reading a handwritten letter. His hands tremble." }]
      },
      {
        "type": "character",
        "content": [{ "type": "text", "text": "RAMESH" }]
      },
      {
        "type": "parenthetical",
        "content": [{ "type": "text", "text": "reading aloud" }]
      },
      {
        "type": "dialogue",
        "content": [{ "type": "text", "text": "\"നിങ്ങൾ ഇത് വായിക്കുമ്പോൾ ഞാൻ ജീവിച്ചിരിക്കില്ല...\"" }]
      },
      {
        "type": "action",
        "content": [{ "type": "text", "text": "He sets the letter down. Stares at the postmark date." }]
      },
      {
        "type": "character",
        "content": [{ "type": "text", "text": "RAMESH" }]
      },
      {
        "type": "parenthetical",
        "content": [{ "type": "text", "text": "to himself" }]
      },
      {
        "type": "dialogue",
        "content": [{ "type": "text", "text": "ഇത് ഇന്നലെ post ചെയ്തതാണ്." }]
      },
      {
        "type": "action",
        "content": [{ "type": "text", "text": "His phone RINGS. He picks it up." }]
      },
      {
        "type": "character",
        "content": [{ "type": "text", "text": "MEERA (V.O.)" }]
      },
      {
        "type": "dialogue",
        "content": [{ "type": "text", "text": "Ramesh, have you seen the news?" }]
      },
      {
        "type": "character",
        "content": [{ "type": "text", "text": "RAMESH" }]
      },
      {
        "type": "dialogue",
        "content": [{ "type": "text", "text": "What news?" }]
      },
      {
        "type": "character",
        "content": [{ "type": "text", "text": "MEERA (V.O.)" }]
      },
      {
        "type": "parenthetical",
        "content": [{ "type": "text", "text": "beat" }]
      },
      {
        "type": "dialogue",
        "content": [{ "type": "text", "text": "Suresh is dead." }]
      },
      {
        "type": "action",
        "content": [{ "type": "text", "text": "Ramesh looks at the letter. Then at the phone. His expression hardens." }]
      },
      {
        "type": "transition",
        "content": [{ "type": "text", "text": "SMASH CUT TO:" }]
      },
      {
        "type": "scene_heading",
        "content": [{ "type": "text", "text": "EXT. POLICE STATION - DAY" }]
      },
      {
        "type": "action",
        "content": [{ "type": "text", "text": "Ramesh walks up the steps of the station he was forced to leave six months ago. He pauses at the entrance, then pushes through the doors." }]
      }
    ]
  },
  "meta": {
    "title": "The Last Letter",
    "author": "Arun Kumar",
    "director": "Arun Kumar",
    "contact": "arun@example.com\n+91 98765 43210",
    "draft_number": 1,
    "draft_date": "2026-03-14",
    "created_at": "2026-03-14T10:00:00Z",
    "updated_at": "2026-03-14T10:00:00Z"
  },
  "settings": {
    "font": "noto-sans-malayalam",
    "default_language": "malayalam",
    "input_scheme": "mozhi"
  },
  "story": {
    "idea": "A retired detective receives a letter from a murder victim — posted the day before the killing.",
    "synopsis": "",
    "treatment": "",
    "narrative": ""
  },
  "scene_cards": []
}
```

---

## Notes for LLM Generation

1. **Character names must be UPPERCASE** in `character` elements.
2. **Scene headings must be UPPERCASE** and follow the `INT./EXT. LOCATION - TIME` format.
3. **Parentheticals must NOT include parentheses** — store `"whispering"`, not `"(whispering)"`. The app wraps them automatically.
4. **Each text block is a separate node.** Don't put multiple paragraphs in one `action` node — use one `action` node per paragraph.
5. **Malayalam text** is standard Unicode. Mixed-language text within a single element is normal and expected.
6. **No formatting marks.** There is no bold, italic, or underline — the schema has no marks. Emphasis is conveyed through CAPS, word choice, and parentheticals.
7. **The `content` array in a text-bearing element** should contain `{ "type": "text", "text": "..." }` objects. For empty elements, omit `content` or use `[]`.
8. **The file extension** must be `.screenplay`.
9. **Encoding** must be UTF-8.
10. **Transitions are optional** and used sparingly in modern screenwriting. `CUT TO:` between every scene is considered outdated.
