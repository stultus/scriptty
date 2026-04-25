# Scriptty — Claude Code Configuration

## Project Overview

Scriptty is an offline desktop screenwriting application built for Malayalam and English
screenwriters. It supports Hollywood single-column format and Indian two-column format
(export only). The app targets Malayalam filmmakers and writers, with built-in Malayalam
input methods requiring no external tools.

**Product name:** Scriptty  
**Primary domain:** scriptty.app  
**File extension:** .screenplay  

---

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop framework | Tauri 2 |
| Frontend | Svelte 5 + TypeScript + Vite |
| Editor | ProseMirror |
| PDF generation | Typst (Rust crate) |
| Mozhi engine | Custom (greedy transliteration from SMC Mozhi spec) |
| Backend language | Rust 1.93+ |
| Node.js | v22 / npm |

---

## Developer Context

The lead developer (Hrishi) is not a Rust expert. When writing Rust code:

- Always add inline comments explaining ownership, borrowing, and lifetime concepts when
  they appear
- Explain non-obvious Rust patterns briefly in a code comment
- Prefer clarity over cleverness in Rust code
- Never assume Rust knowledge — explain what each new pattern does the first time it appears
- When introducing a new Rust concept (e.g. `Result`, `Option`, `impl`, lifetimes), add a
  one-line comment explaining what it means in plain English

---

## Project Structure

```
scriptty/
├── src/                          # Svelte frontend
│   ├── lib/
│   │   ├── editor/               # ProseMirror screenplay editor
│   │   │   ├── schema.ts             # Screenplay element schema
│   │   │   ├── keymap.ts             # Tab/Enter navigation
│   │   │   ├── autoUppercase.ts      # Auto-uppercase plugin for scene headings/characters
│   │   │   ├── characterAutocomplete.ts  # Character-name suggestion plugin
│   │   │   ├── characterList.ts      # Per-scene character extraction
│   │   │   ├── findReplace.ts        # Find/replace plugin with DecorationSet
│   │   │   └── input/                # Malayalam input engine
│   │   │       ├── InputModeManager.ts
│   │   │       ├── mozhi.ts
│   │   │       ├── inscript1.ts
│   │   │       └── inscript2.ts
│   │   ├── stores/               # Svelte stores (app state)
│   │   │   ├── documentStore.svelte.ts  # Document state + active-episode accessors
│   │   │   ├── editorStore.svelte.ts    # Shared EditorView reference
│   │   │   ├── themeStore.svelte.ts     # Dark/light theme toggle with localStorage
│   │   │   └── updateStore.svelte.ts    # In-app update check state
│   │   ├── actions/              # Svelte actions
│   │   │   └── focusTrap.ts          # Modal focus trap + restoration
│   │   └── components/           # UI components
│   │       ├── WelcomeScreen.svelte    # Landing screen — new film, new series, open
│   │       ├── Editor.svelte           # ProseMirror editor + scene number gutter
│   │       ├── TitleBar.svelte         # Top bar: actions, title, view switcher
│   │       ├── StatusBar.svelte        # Bottom bar: language, scheme, save state, gear
│   │       ├── LeftPanel.svelte        # Sidebar wrapper (scene navigator OR episode list)
│   │       ├── SceneNavigator.svelte   # Scene list — click-to-jump, drag-to-reorder
│   │       ├── SeriesEpisodeList.svelte # Episode tree for Series projects
│   │       ├── SeriesTitleDialog.svelte # New-series title prompt
│   │       ├── SceneCardsView.svelte   # Full-panel grid of scene breakdown cards
│   │       ├── StoryModeView.svelte    # Full-screen narrative writing view
│   │       ├── OutlinePeek.svelte      # Bottom strip showing current scene context
│   │       ├── CommandPalette.svelte   # ⌘K palette — commands + scene jump
│   │       ├── FormatBubble.svelte     # Floating B/I/U bubble above selection
│   │       ├── FindReplaceBar.svelte   # Inline find/replace UI (Cmd+F, Cmd+Shift+H)
│   │       ├── MetadataModal.svelte    # Screenplay metadata editor
│   │       ├── ExportModal.svelte      # Combined PDF/Fountain/text export
│   │       ├── SettingsModal.svelte    # Consolidated settings popover
│   │       ├── StatisticsModal.svelte  # Page/scene/word/dialogue statistics
│   │       ├── HelpModal.svelte        # User guide with keyboard shortcuts
│   │       ├── AboutModal.svelte       # App info, credits, version
│   │       └── UpdateToast.svelte      # Non-intrusive new-version toast
│   └── routes/                   # SvelteKit pages
│       ├── +layout.svelte        # Global reset, CSS variables, theme system
│       └── +page.svelte          # Main app page, keyboard shortcuts, menu events
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # Tauri entry point
│   │   ├── lib.rs                # App builder, native menu setup
│   │   ├── commands/             # Tauri commands (called from frontend)
│   │   │   ├── mod.rs
│   │   │   ├── file.rs               # save/open/new .screenplay files
│   │   │   └── export.rs             # PDF, Fountain, plain text export
│   │   ├── screenplay/           # Document model and business logic
│   │   │   ├── mod.rs
│   │   │   ├── document.rs           # .screenplay JSON schema (incl. series)
│   │   │   ├── pdf.rs                # Typst PDF generation
│   │   │   ├── fountain.rs           # Fountain export
│   │   │   └── plaintext.rs          # Plain text export
│   │   └── fonts/                # Font loading for Typst
│   ├── fonts/                    # Fonts embedded in PDFs (Noto Sans Malayalam, Manjari)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── static/
│   └── fonts/                    # Fonts served to the Svelte UI (incl. Courier Prime)
├── docs/                         # GitHub Pages site (scriptty.app)
│   ├── index.html
│   └── downloads.json            # Auto-updated on each release
├── .github/workflows/            # CI: cross-platform build + release tagging
├── .claude/                      # Claude Code configuration
│   ├── CLAUDE.md                 # This file
│   ├── PROGRESS.md               # Development progress tracker
│   ├── agents/                   # Sub-agent definitions
│   └── skills/                   # Custom skills (e.g. ui-design)
└── package.json
```

---

## Architecture Decisions (Locked)

These are final. Do not suggest alternatives unless explicitly asked.

### Editor

- ProseMirror is the editor library — not TipTap, not CodeMirror, not contenteditable
- Continuous page view — single scrollable editor, no page breaks
- Editor always shows Hollywood single-column format
- Indian two-column is a PDF export option only — not an editor mode
- Element types: SceneHeading, Action, Character, Parenthetical, Dialogue, Transition

### Element Navigation (Tab/Enter/Shortcut behavior)

| Current element | Key | Next element |
|---|---|---|
| SceneHeading | Enter | Action |
| Action | Enter | Action (new paragraph) |
| Action | Tab | Character |
| Character | Enter | Dialogue |
| Dialogue | Enter | Character |
| Dialogue | Tab | Parenthetical |
| Parenthetical | Enter | Dialogue |
| Parenthetical | Tab | Character |
| Transition | Enter | Action (Shift+Enter for a new scene — mid-scene transitions / montage) |
| Any element | Shift+Enter | SceneHeading (new scene) |
| Any element | Cmd+Shift+T | Transition |
| Character/Dialogue | Shift+Tab | Action |
| Parenthetical | Shift+Tab | Dialogue |
| Action (cursor at pos 0) | Shift+Tab | SceneHeading |

### Malayalam Language Support

- Three input schemes: SMC Mozhi, Inscript 2, Inscript 1
- Ctrl+Space toggles English/Malayalam mode mid-sentence
- Mixed script per line is the default (e.g. "രമേഷ് Flat ലേക്ക് നടന്നു")
- Mozhi uses a custom greedy transliteration engine (ported from 3in1.js by stultus)
- Mozhi conversion hash: Malayalam output + Latin input → new Malayalam output
- MozhiEngine class in mozhi.ts maintains a cyrBuffer for stateful matching
- Inscript 1 and Inscript 2 are static keymaps (~100 lines each)
- InputModeManager.ts is the single source of truth for input mode state
- Input scheme is user-level config, not document-level

### Fonts

- Bundled fonts (PDF + UI): **Manjari** (default), **Noto Sans Malayalam**
- UI-only font: **Courier Prime** — used by the editor for the classic screenplay
  monospace look on Latin text; not embedded in PDFs
- All fonts licensed SIL OFL 1.1 — safe to bundle in commercial software
- One selected font applies to all editor text — Malayalam and English both use it
- Font files live in two places:
  - `src-tauri/fonts/` — embedded in PDFs at export time (Manjari, Noto only)
  - `static/fonts/` — served to the Svelte UI via CSS (Manjari, Noto, Courier Prime)
- No system font dependency — app works on a fresh OS install
- Default font slug is the constant `DEFAULT_FONT` in `src-tauri/src/screenplay/document.rs`
  (currently `"manjari"`); both `default_font()` and `ScreenplaySettings::default()` reference it

### PDF Export

- Typst Rust crate handles all PDF generation — not printpdf, not any other crate
- Flow: Rust receives ProseMirror JSON → generates Typst markup string → Typst compiles
  to PDF bytes in memory → bytes returned to frontend
- No temp files written to disk during export
- Selected font is embedded in every PDF
- Format (Hollywood or Indian two-column) is chosen at export time, not document creation

### File Format (.screenplay)

JSON. Two top-level shapes — Film (default) and Series — distinguished by the
`type` field. Films use the top-level meta/settings/story/content/scene_cards
directly. Series files put real data inside `series.episodes[]` and the
top-level fields are placeholders.

```json
{
  "type": "film",
  "content": {},
  "meta": {
    "title": "",
    "author": "",
    "director": "",
    "tagline": "",
    "registration_number": "",
    "footnote": "",
    "contact": "",
    "draft_number": 1,
    "draft_date": "",
    "created_at": "",
    "updated_at": ""
  },
  "settings": {
    "font": "manjari",
    "default_language": "malayalam",
    "input_scheme": "mozhi",
    "scene_number_start": 1,
    "show_characters_below_header": false
  },
  "story": {
    "idea": "",
    "synopsis": "",
    "treatment": "",
    "narrative": ""
  },
  "scene_cards": []
}
```

Series file shape:

```json
{
  "type": "series",
  "series": {
    "title": "The Return",
    "episodes": [
      {
        "id": "uuid-...",
        "number": 1,
        "title": "Pilot",
        "content": {},
        "meta": { ... },
        "settings": { ... },
        "story": { ... },
        "scene_cards": []
      }
    ]
  }
}
```

Notes:
- `type` defaults to `"film"` when missing — every legacy file loads unchanged.
- Every meta/settings field is `#[serde(default)]`, so slim or hand-authored
  files (and series episodes that omit timestamps) deserialize without error.
- `meta` carries optional `tagline`, `registration_number`, and `footnote` fields
  for the title page.
- `settings.scene_number_start` is clamped to 1..=9999 on deserialize.
- `settings.show_characters_below_header` toggles the auto-generated character
  line under each scene heading in the editor.
- `scene_cards[].extra_characters` is a comma-separated list of background /
  silent characters merged with auto-detected speakers.
- `scene_cards[].scene_index` is a 0-based pointer into the flat ordered list
  of scene_heading nodes — not a stable ID. For series, the frontend's
  `buildSeriesExportDocument` flattens episode cards into a single list before
  the backend sees them; the PDF generator relies on that flattening.

Full format spec: see `SCREENPLAY_FORMAT.md` at project root.

### Export Formats

- **PDF** — Hollywood or Indian two-column layout, chosen at export time
- **Fountain** — UTF-8 plain text, interoperability with other tools
- **Plain text** — readable draft sharing

### Export System

- Single "Export" button opens an Export modal (replaces separate Hollywood/Indian buttons)
- Checkbox sections: Title Page, Synopsis, Treatment, Narrative, Screenplay, Scene Cards
- Format radio: Hollywood (single column) or Indian (two column) — shown only when Screenplay is selected
- Combined PDF output — selected sections concatenated in order
- Synopsis/Treatment/Narrative PDF: project title heading, section subtitle, credit lines, prose layout
- Scene Cards PDF: project title, credit lines, table/card layout per scene
- Smart credit formatting: "Written and Directed by" when same person, separate credits otherwise
- Conditional pagebreaks — no blank leading page when title page is excluded

### Title Page

- Auto-generated at PDF export time from `meta` fields
- Not editable inside the editor
- Fields printed: title, author, director (with smart credit formatting), contact, draft number, draft date
- Editor starts directly at FADE IN:

### Scene Navigator

- Collapsible left panel, toggle with Ctrl+B
- Shows: auto-computed scene number + scene heading text + visual signals
  (INT/EXT, DAY/NIGHT, notes presence)
- Scene numbers derived from document order, offset by `scene_number_start` setting
- `scene_number_start` is a per-document setting (default 1), configurable in Settings modal
- Useful for co-writing: each writer's file can start numbering from their assigned range
- Drag to rearrange scenes triggers auto-renumber
- Click to jump to scene
- For Series projects, `LeftPanel` shows `SeriesEpisodeList` instead — episodes
  expand to reveal their own `SceneNavigator` for the active episode

### Series Mode

- Activated when `document.type === "series"` (or via "New Series" on the
  Welcome screen / File menu)
- Series have a top-level title plus an ordered list of episodes; each
  episode is a complete screenplay (own meta/settings/story/content/scene_cards)
- `documentStore` exposes active-* accessors (`activeContent`, `activeMeta`,
  `activeSettings`, `activeStory`, `activeSceneCards`, `activeEpisode`,
  `activeEpisodeIndex`) that multiplex film vs. series data — UI components
  read these instead of branching on project type
- Active episode auto-expands in the navigator; switching slides smoothly
- Series export generates a series-level title page; episode cards are
  flattened into a single 0-based scene list before the backend renders
  (see `buildSeriesExportDocument` in `ExportModal.svelte`)
- Synthetic `episode_boundary` ProseMirror node carries pagebreaks between
  episodes during series PDF export

### Command Palette

- ⌘K opens a fuzzy-search palette listing every command and every scene heading
- Keyboard-first: arrow keys navigate, Enter activates, Escape dismisses
- Replaces several status bar / menu lookups for power-user flow

### Outline Peek

- Bottom strip in the editor showing the current scene in context
- Displays scene heading, position ("Scene 7 of 42"), and adjacent scene previews
- Off by default; toggleable from Settings or View menu

### Find & Replace

- ProseMirror plugin with DecorationSet for search highlighting
- `Cmd+F` opens find, `Cmd+Shift+H` opens find-and-replace
- Case sensitivity toggle, prev/next match navigation
- Replace-all is a single ProseMirror transaction (one undo step)

### Update Notifications

- `Help → Check for Updates` performs an in-app version check
- Non-intrusive `UpdateToast` shown when a newer version is available
- Powered by `updateStore.svelte.ts`; never blocks startup or writes
  localStorage nags

### Theme System

- Dark mode (default) and light mode — both first-class
- CSS custom properties defined in `+layout.svelte` under `[data-theme]` selectors
- Theme state managed by `themeStore.svelte.ts` with `$state` rune
- Persisted to `localStorage` under key `scriptty-theme`
- Accessible via Settings modal (gear icon in status bar)
- Warm Kerala-rooted palette: teal accent, cream page, amber dirty indicator

### Story Mode

- Full-screen narrative writing view, toggled with Cmd+Shift+L
- Page-card styling matching the screenplay editor (white page, box shadow, centered)
- Malayalam input support via InputModeManager singleton (same Ctrl+Space toggle)
- Status bar with language mode indicator and scheme selector
- Word count display in toolbar
- Escape to close, returns to screenplay editor

### Scene Cards

- Per-scene breakdown for shoot planning
- Auto-populated: scene number, heading, location, time, characters, page estimate
- Manually editable: scene description, shoot notes
- Grid view accessible via Cmd+Shift+K, replaces editor as full-panel view
- Data stored in `scene_cards` field of `.screenplay` file

### Dirty-State Guard

- Native OS confirmation dialog (Save / Don't Save / Cancel) via `message` from plugin-dialog
- Intercepts: New, Open (buttons + menu + keyboard), window close, Quit (custom menu item)
- `documentStore.confirmIfDirty()` — single entry point for all guard checks
- Custom quit menu item replaces `PredefinedMenuItem::quit` to allow frontend interception

### Character Autocomplete

- Triggers after 2 characters typed in a Character element
- Suggests only names already present in the document
- Unicode-aware — Malayalam names and English names in the same pool
- Dismissed with Escape, accepted with Enter or Tab

### Settings Modal

- Consolidated settings UI — language mode, keyboard scheme, font, theme in one popup
- Opens from gear icon in the editor status bar (bottom-left)
- Replaces scattered TitleBar controls (font selector, theme toggle removed from TitleBar)
- Keyboard scheme selector shown only when Malayalam mode is active
- Escape to close, click outside to dismiss

### Editor Layout

- Continuous page view — single scrollable editor without page breaks
- ProseMirror editor uses min-height for seamless infinite scroll
- Window launches maximized (not fullscreen) to keep title bar and taskbar visible

---

## Commands

### Development

```bash
# Start frontend dev server only
npm run dev

# Start full Tauri dev (frontend + backend + opens window)
cargo tauri dev

# Type check frontend only (no emit)
npx tsc --noEmit

# Lint frontend
npm run lint

# Check Rust compilation without full build (fast)
cd src-tauri && cargo check

# Lint Rust code
cd src-tauri && cargo clippy

# Format Rust code
cd src-tauri && cargo fmt
```

### Build

```bash
# Production build — outputs installer to src-tauri/target/release/bundle/
cargo tauri build
```

---

## Coding Standards

### TypeScript / Svelte

- TypeScript strict mode — no `any` types anywhere
- Svelte 5 runes syntax only: `$state`, `$derived`, `$effect`, `$props`
- Do not use legacy Svelte 4 syntax (`onMount` is acceptable, reactive `$:` is not)
- All Tauri command calls must handle errors explicitly — never assume success
- Use `invoke` from `@tauri-apps/api/core` for all Rust command calls

### Rust

- Follow standard Rust naming: `snake_case` for functions/variables, `PascalCase` for
  types and structs
- Never use `unwrap()` in Tauri command handlers — always use `?` or match on errors
- All public functions must have a doc comment (`///`)
- Return `Result<T, String>` from Tauri commands that can fail. For genuinely
  infallible commands, return `T` directly — don't fake a `Result` (see #91 / commit
  3f6d516 dropping `Result` from `new_screenplay`)
- Use `serde` for all JSON serialization/deserialization
- Keep `cargo clippy` and `npx svelte-check` at zero warnings — both are gates
  before any commit

---

## What is Tauri? (Reference)

Tauri is a desktop app framework. The frontend (Svelte) runs inside the OS's built-in
WebView (WebKit on Mac, WebView2 on Windows, WebKitGTK on Linux). The backend is a
compiled Rust binary. The two communicate via **commands** — the frontend calls a Rust
function using `invoke()`, Rust executes it and returns a result.

This means:
- UI logic, editor, input methods → Svelte/TypeScript
- File I/O, PDF generation, OS integration → Rust
- No server, no network, fully offline

---

### Parenthetical Elements

- Parentheses are stored **in content** (commit 27a126f / issue #59) — not via
  CSS pseudo-elements. Copy/paste, Fountain export, and external editors all see
  the literal `( )` characters.
- Empty parentheticals still render `()` on one line (trailing `<br>` hidden).
- All export formats defensively wrap in parens if the content somehow lacks them.

## Remaining Work

### Medium Term
- **Revision mode** — track changes per draft, asterisk marks in margin, Hollywood color cycle
- **Draft history** — save snapshots on each save, restore from history, max 50 per file

## Deferred (Do Not Implement Yet)

- FDX (Final Draft XML) export
- Rachana font / traditional Malayalam orthography
- Import from Final Draft / Fountain
- Real-time collaboration
- Cloud sync
- Mobile support

---

## Key Constraints (Non-negotiable)

- Fully offline — zero network calls at runtime
- No server setup required for end users
- Single installable binary — works on macOS, Windows, Linux out of the box
- Malayalam rendering via bundled fonts only — no OS font dependency
- Malayalam input works without any OS IME installation
- All data stored locally in .screenplay files — no database, no cloud
