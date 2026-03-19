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
│   │   │   ├── schema.ts         # Screenplay element schema
│   │   │   ├── keymap.ts         # Tab/Enter navigation
│   │   │   ├── autoUppercase.ts  # Auto-uppercase plugin for scene headings/characters
│   │   │   └── input/            # Malayalam input engine
│   │   │       ├── InputModeManager.ts
│   │   │       ├── mozhi.ts
│   │   │       ├── inscript1.ts
│   │   │       └── inscript2.ts
│   │   ├── stores/               # Svelte stores (app state)
│   │   │   ├── documentStore.svelte.ts  # Document state (content, meta, settings, dirty)
│   │   │   ├── editorStore.svelte.ts    # Shared EditorView reference
│   │   │   └── themeStore.svelte.ts     # Dark/light theme toggle with localStorage
│   │   └── components/           # UI components
│   │       ├── Editor.svelte          # ProseMirror editor + status bar
│   │       ├── TitleBar.svelte        # Top bar: actions, title, font/theme controls
│   │       ├── LeftPanel.svelte       # Tabbed sidebar (Scenes | Story), auto-widening
│   │       ├── SceneNavigator.svelte  # Scene list with click-to-jump
│   │       ├── StoryPanel.svelte      # Idea/Synopsis/Treatment text areas
│   │       ├── SceneCardsView.svelte  # Full-panel grid of scene breakdown cards
│   │       ├── StoryModeView.svelte  # Full-screen narrative writing view
│   │       ├── MetadataModal.svelte   # Screenplay metadata editor
│   │       ├── ExportModal.svelte     # Combined PDF export with section selection
│   │       ├── SettingsModal.svelte    # Consolidated settings (language, scheme, font, theme)
│   │       ├── HelpModal.svelte       # User guide with keyboard shortcuts
│   │       └── AboutModal.svelte      # App info, credits, version
│   └── routes/                   # SvelteKit pages
│       ├── +layout.svelte        # Global reset, CSS variables, theme system
│       └── +page.svelte          # Main app page, keyboard shortcuts, menu events
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # Tauri entry point
│   │   ├── lib.rs                # App builder, native menu setup
│   │   ├── commands/             # Tauri commands (called from frontend)
│   │   │   ├── mod.rs
│   │   │   ├── file.rs           # save/open .screenplay files
│   │   │   └── export.rs         # PDF, Fountain, plain text export
│   │   ├── screenplay/           # Document model and business logic
│   │   │   ├── mod.rs
│   │   │   ├── document.rs       # .screenplay JSON schema
│   │   │   └── pdf.rs            # Typst PDF generation
│   │   └── fonts/                # Font loading for Typst
│   ├── fonts/                    # Bundled .ttf files (Noto Sans Malayalam, Manjari)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── static/
│   └── fonts/                    # Fonts served to the Svelte UI
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
| Dialogue | Enter | Action |
| Dialogue | Tab | Parenthetical |
| Parenthetical | Enter | Dialogue |
| Parenthetical | Tab | Character |
| Transition | Enter | SceneHeading |
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

- Bundled fonts: Noto Sans Malayalam (default), Manjari (alternative)
- Both fonts licensed SIL OFL 1.1 — safe to bundle in commercial software
- Single font applies to ALL text — Malayalam and English both use the same font
- Font files live in two places:
  - `src-tauri/fonts/` — for Typst PDF embedding
  - `static/fonts/` — for the Svelte UI via CSS
- No system font dependency — app works on a fresh OS install

### PDF Export

- Typst Rust crate handles all PDF generation — not printpdf, not any other crate
- Flow: Rust receives ProseMirror JSON → generates Typst markup string → Typst compiles
  to PDF bytes in memory → bytes returned to frontend
- No temp files written to disk during export
- Selected font is embedded in every PDF
- Format (Hollywood or Indian two-column) is chosen at export time, not document creation

### File Format (.screenplay)

JSON with top-level keys:

```json
{
  "content": {},
  "meta": {
    "title": "",
    "author": "",
    "director": "",
    "contact": "",
    "draft_number": 1,
    "draft_date": "",
    "created_at": "",
    "updated_at": ""
  },
  "settings": {
    "font": "noto-sans-malayalam",
    "default_language": "malayalam",
    "input_scheme": "mozhi"
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

`content` is the ProseMirror document JSON serialization.
`meta` includes `director` field (added with `#[serde(default)]` for backward compat).
`story` holds Story Panel text sections plus full-length narrative.
`scene_cards` holds per-scene descriptions and shoot notes.

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
- Shows: auto-computed scene number + scene heading text
- Scene numbers derived from document order — never stored
- Drag to rearrange scenes triggers auto-renumber
- Click to jump to scene

### Theme System

- Dark mode (default) and light mode — both first-class
- CSS custom properties defined in `+layout.svelte` under `[data-theme]` selectors
- Theme state managed by `themeStore.svelte.ts` with `$state` rune
- Persisted to `localStorage` under key `scriptty-theme`
- Accessible via Settings modal (gear icon in status bar)
- Warm Kerala-rooted palette: teal accent, cream page, amber dirty indicator

### Story Panel

- Collapsible left panel tab alongside Scene Navigator
- Four sections: Idea (logline), Synopsis, Treatment (detailed story), Narrative (collapsed by default)
- Tab switcher at top of left panel: Scenes | Story
- Panel auto-widens to 420px on Story tab for more writing space
- All sections support Malayalam input (Ctrl+Space toggle applies)
- Data stored in `story` field of `.screenplay` file
- Narrative section has "Cmd+Shift+L for full screen" hint

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
- Return `Result<T, String>` from all Tauri commands so errors surface cleanly to the
  frontend
- Use `serde` for all JSON serialization/deserialization

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

- Auto-parentheses via CSS `::before`/`::after` pseudo-elements — parens are visual only, not stored in content
- All export formats (PDF, Fountain, plain text) defensively wrap in parens if not already present
- ProseMirror trailing `<br>` hidden in empty parentheticals to keep `()` on one line

## Remaining Work

### Medium Term
- **Revision mode** — track changes per draft, asterisk marks in margin, Hollywood color cycle
- **Draft history** — save snapshots on each save, restore from history, max 50 per file

## Deferred (Do Not Implement Yet)

- FDX (Final Draft XML) export
- Courier font / Hollywood submission mode
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
