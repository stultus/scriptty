# Scriptty — Development Progress

## Completed

### Project scaffold
- Tauri 2 + SvelteKit + TypeScript + Vite project initialized
- Directory structure matching CLAUDE.md spec
- SSR disabled for editor page (`+page.ts`)

### Rust backend foundation
- `ScreenplayDocument`, `ScreenplayMeta`, `ScreenplaySettings` structs with serde (de)serialization and defaults
- `screenplay/mod.rs` exports document module; `pdf.rs` placeholder
- `BundledFont` struct and `bundled_fonts()` with `include_bytes!` for all 4 fonts (Noto Sans Malayalam .ttf, Manjari .otf)
- `commands/mod.rs` declares `file` and `export` submodules (placeholder implementations)
- `lib.rs` declares `commands`, `fonts`, `screenplay` modules
- Compiles clean with `cargo check`

### ProseMirror editor
- Screenplay schema with 7 node types: `doc`, `scene_heading`, `action`, `character`, `parenthetical`, `dialogue`, `transition`, `text`
- No marks (no bold/italic/underline)
- Each element renders as `<p>` with `data-type` attribute for DOM round-tripping

### Tab/Enter keymap
- Enter creates new block of correct type per transition table
- Tab changes current node type in-place (action↔character, dialogue→character)
- Shift-Tab reverts character/dialogue to action

### Malayalam input system
- `InputModeManager` singleton: tracks `isMalayalam` toggle and active `scheme`
- Inscript 2 keymap: 80+ mappings (vowels, matras, consonants, chillus, Malayalam numerals)
- Inscript 1 keymap: original layout with differences (e.g. `z`→`െ`, `]`→`്`)
- Mozhi stub: `processMozhiKey()` returns null, ready for Varnam JS integration
- Capture-phase keydown listener on `view.dom` intercepts keys before ProseMirror

### Editor UI
- `Editor.svelte` component with ProseMirror wired up (schema + keymap + history)
- Ctrl+Space toggles English/Malayalam mode
- Status bar showing current mode (ENGLISH/MALAYALAM) and element type
- Hollywood-format CSS: uppercase scene headings, centered characters, indented dialogue, right-aligned transitions
- `@font-face` declarations for all 4 bundled fonts
- Dark theme (#1a1a1a background, #242424 editor surface)

## In Progress

Nothing currently in progress.

## Next Up

1. **Tauri commands for file I/O** — save/open `.screenplay` files via `commands/file.rs`
2. **PDF export** — Typst integration in `screenplay/pdf.rs`, Hollywood single-column layout
3. **Scene navigator** — collapsible left panel (Ctrl+B), click-to-jump, drag-to-reorder
4. **Character autocomplete** — trigger after 2 chars in Character element, Unicode-aware
5. **Title page** — auto-generated at PDF export from `meta` fields
6. **Indian two-column PDF export** — alternate layout option at export time
7. **Fountain export** — UTF-8 plain text for interoperability
8. **Plain text export** — readable draft sharing
9. **Mozhi integration** — Varnam JS for transliteration input
10. **Font selection UI** — switch between Noto Sans Malayalam and Manjari

## Known Issues

- Debug `console.log` statements in `Editor.svelte` keydown handler — remove before release
- Default input scheme is `inscript2` (temporary) — should respect user settings once settings UI exists
- Mozhi input scheme is a non-functional stub — selecting it silently does nothing
- No undo/redo keyboard shortcuts wired up yet (history plugin is loaded but Cmd+Z binding not confirmed)
- No visual feedback when Ctrl+Space toggles mode (only status bar updates, no toast/flash)
