# Scriptty — Development Progress

## Status: v0.7.0 shipped — Web series support, command palette, outline peek, Kerala palette

---

## Phase 1 — Completed

### Infrastructure
- [x] Tauri 2 + SvelteKit scaffold — desktop window
- [x] Claude Code config — CLAUDE.md, 3 sub-agents, hooks
- [x] Project structure scaffolded
- [x] Bundled fonts — Noto Sans Malayalam (Regular + Bold), Manjari (Regular + Bold)
- [x] Rust backend structs — ScreenplayDocument, ScreenplayMeta, ScreenplaySettings
- [x] App binary renamed to scriptty, identifier updated
- [x] App icon — ഋ clapperboard, all platform sizes generated
- [x] adapter-static for Tauri build

### Editor
- [x] ProseMirror schema — 8 node types
- [x] Tab/Enter navigation keymap — full Hollywood element flow
- [x] Shift+Enter — new scene heading from anywhere
- [x] Shift+Tab — convert Action to Scene Heading
- [x] Cmd+Z / Cmd+Shift+Z — undo/redo
- [x] Auto-uppercase for scene headings and character names (Latin only, Malayalam passthrough)
- [x] Hollywood screenplay CSS formatting — fixed pixel margins, centered content area
- [x] Page background — cream page on dark/light desk aesthetic
- [x] Font rendering via :global() CSS

### Input Methods
- [x] InputModeManager — Ctrl+Space toggle English/Malayalam
- [x] Inscript 1 — static keymap
- [x] Inscript 2 — static keymap
- [x] Mozhi — full transliteration engine (greedy longest-match, conjuncts, chillus, geminate caps)
- [x] Input scheme switcher UI in status bar
- [x] Default scheme: Mozhi

### File I/O
- [x] .screenplay file format — JSON with content, meta, settings
- [x] save_screenplay, open_screenplay, new_screenplay Tauri commands
- [x] saveWithDialog() — native save dialog, Cmd+S shortcut
- [x] Save As — Cmd+Shift+S, always opens file dialog
- [x] openDocument() — native open dialog, Cmd+O shortcut
- [x] Title derived from filename on first save
- [x] Dirty state tracking — amber dot indicator

### Scene Navigator
- [x] Collapsible left panel — Ctrl+B toggle
- [x] Auto-numbered scene list
- [x] Click-to-jump
- [x] Reactive updates on every keystroke

### Metadata
- [x] MetadataModal — title, author, contact, draft number, draft date
- [x] Meta button in TitleBar
- [x] Metadata persisted in .screenplay file

### Font Selection
- [x] Font selector UI — segmented control (Noto | Manjari)
- [x] Live font switching in editor
- [x] Font persisted in document settings

### PDF Export
- [x] Typst compiler integration — ScreenplayWorld trait, in-memory compilation
- [x] Hollywood single-column PDF — A4, all element types, page break rules
- [x] Indian two-column PDF — 50/50 grid, character/dialogue alignment, page break rules
- [x] Title page — auto-generated from metadata
- [x] Bundled font embedding in PDF
- [x] Export buttons in TitleBar
- [x] 17 unit tests passing

### UI / Design System
- [x] Full UI revamp — CSS custom properties, warm Kerala-rooted palette
- [x] Dark/light theme toggle — themeStore with localStorage persistence
- [x] TitleBar — ghost buttons, segmented font selector, teal primary Save
- [x] Status bar — full-width bottom bar (was floating corner)
- [x] MetadataModal — backdrop blur, scale animation, themed inputs
- [x] SceneNavigator — accent left-border on active, cubic-bezier transitions
- [x] Scrollbar styling — thin, muted color
- [x] App menu — macOS native menu bar (File + Edit) with Tauri 2 menu API

---

## Phase 2 — Completed

### 1. Help/About Menu
- [x] Help submenu in macOS native menu bar
- [x] "About Scriptty" menu item → emits `menu-about` event → AboutModal
- [x] AboutModal.svelte — ഋ logo, version 0.2.0, developer info, credits
- [x] "Report an Issue" → opens GitHub issues in browser (tauri-plugin-opener)
- [x] "View on GitHub" → opens repo in browser

### 2. Story Panel
- [x] `story` field added to ScreenplayDocument (Rust + TypeScript) with `#[serde(default)]`
- [x] StoryPanel.svelte — three collapsible sections (Idea, Synopsis, Treatment)
- [x] LeftPanel.svelte — tab switcher (Scenes | Story), widens to 420px on Story tab
- [x] Malayalam input works in Story Panel text areas (inherits from editor context)
- [x] Data persisted in .screenplay JSON

### 3. Export Modal
- [x] ExportModal.svelte — replaces separate Hollywood/Indian buttons
- [x] Checkbox sections: Title Page, Synopsis, Treatment, Screenplay, Scene Cards
- [x] Format radio: Hollywood / Indian
- [x] Combined PDF generation in Rust (`export_combined_pdf` command)
- [x] Synopsis/Treatment PDF sections: centered heading, prose layout
- [x] Scene Cards PDF section: table layout per scene
- [x] Single "Export" button in TitleBar opens modal

### 4. Scene Cards
- [x] `scene_cards` field added to ScreenplayDocument (Rust + TypeScript) with `#[serde(default)]`
- [x] Scene heading parser — extracts location, time from INT./EXT. headings
- [x] Character extractor — collects Character elements per scene
- [x] Page estimate — character count / 3000 chars per page
- [x] SceneCardsView.svelte — responsive grid of cards
- [x] Editable description and shoot notes per card
- [x] "Back to Script" button to return to editor
- [x] Cmd+Shift+K shortcut to toggle view

### 5. Dirty-State Guard
- [x] Save confirmation dialog (Save / Don't Save / Cancel) via native `message` dialog
- [x] Guards on: New, Open (TitleBar buttons + menu events + keyboard shortcuts)
- [x] Window close interception via `onCloseRequested`
- [x] Quit interception — custom menu item replaces `PredefinedMenuItem::quit`
- [x] `confirmIfDirty()` method on documentStore

---

## Phase 3 — Completed

### 6. Character Autocomplete
- [x] ProseMirror plugin triggers after 2 chars typed in Character element
- [x] Collects character names from document, filters by prefix (case-insensitive, Unicode-aware)
- [x] Dropdown positioned below cursor, keyboard navigation (arrows/Enter/Tab/Escape)
- [x] Accepts suggestion and creates Dialogue element below

### 7. Fountain Export
- [x] `fountain.rs` — ProseMirror JSON → Fountain plain text (.fountain)
- [x] Title page block, auto-detected scene headings, Malayalam character `@` prefix
- [x] Parentheticals wrapped, transitions auto-detected or forced with `>`
- [x] `export_fountain` Tauri command, button in ExportModal
- [x] 9 unit tests passing

### 8. Find and Replace
- [x] ProseMirror plugin with DecorationSet for search highlighting
- [x] FindReplaceBar.svelte — find/replace modes, case sensitivity toggle
- [x] Match navigation (next/prev), replace current, replace all (single undo step)
- [x] Cmd+F (find), Cmd+Shift+H (find and replace)
- [x] Menu items in Edit menu

### 9. Script Statistics
- [x] StatisticsModal.svelte — computes from ProseMirror JSON on modal open
- [x] Page count, scene count, word count, dialogue blocks, screen time estimate
- [x] INT/EXT/Day/Night scene breakdown
- [x] Per-character table: scenes, dialogue blocks, percentage — sorted by dialogue count
- [x] Refresh button, Cmd+Shift+I shortcut, View menu item

### 10. Plain Text Export
- [x] `plaintext.rs` — ProseMirror JSON → formatted plain text (.txt)
- [x] Character names at col 40, dialogue at col 25 (35-char wrap), parentheticals at col 35
- [x] Transitions right-aligned, scene headings uppercase, metadata header block
- [x] Unicode-aware word wrapping for Malayalam text
- [x] `export_plaintext` Tauri command, button in ExportModal
- [x] 9 unit tests passing

### 11. UI Consistency Fixes
- [x] All modals standardized to 480px width and 24px padding
- [x] Hardcoded `#999` scene number color → `var(--text-muted)`
- [x] FindReplaceBar border-radius standardized to 6px
- [x] App opens maximized by default (changed from fullscreen in Phase 5)
- [x] Window close/quit permission fix (`core:window:allow-close`)

### 12. Drag-and-Drop Scene Reordering
- [x] Scene Navigator: drag handle (⠿) appears on hover, custom mouse-event drag (WebKit-compatible)
- [x] Scene Cards: scene number badge as drag handle, teal border highlight on drop target
- [x] Reorder is a single ProseMirror transaction — undoable with Cmd+Z
- [x] Editor kept mounted (hidden) when Scene Cards shown so ProseMirror view stays alive
- [x] Document marked dirty after reorder, editor scrolls to moved scene

---

## Phase 4 — Completed

### 13. Story Mode
- [x] StoryModeView.svelte — full-screen narrative writing view
- [x] Page-card styling matching screenplay editor (white page, box shadow, centered)
- [x] Malayalam input via InputModeManager singleton (Ctrl+Space, scheme selector)
- [x] Word count display, Escape to close
- [x] Cmd+Shift+L shortcut, menu item in View menu
- [x] Narrative section added to StoryPanel (collapsed by default, "Cmd+Shift+L for full screen" hint)
- [x] `narrative` field added to ScreenplayStory (Rust + TypeScript) with `#[serde(default)]`

### 14. Director Credits & PDF Export Improvements
- [x] `director` field added to ScreenplayMeta (Rust + TypeScript) with `#[serde(default)]`
- [x] MetadataModal updated — "Written by" / "Directed by" labels, director input field
- [x] Smart credit formatting: combined "Written and Directed by" when same person
- [x] Title page: visual hierarchy — labels 11pt gray, names 16pt
- [x] Prose sections (Synopsis/Treatment/Narrative): project title heading, section subtitle, credit lines
- [x] Scene Cards PDF: project title + credits header
- [x] Conditional pagebreaks — no blank leading page when title page excluded
- [x] Prose margins fixed: symmetric `left: 3cm, right: 3cm`
- [x] Narrative checkbox added to ExportModal
- [x] Format selector shown only when Screenplay is checked

### 15. Parenthetical Element Support
- [x] Tab from Dialogue creates Parenthetical (was Dialogue → Tab → Character)
- [x] Tab from Parenthetical → Character, Shift+Tab from Parenthetical → Dialogue
- [x] Auto-parentheses via CSS `::before`/`::after` — parens are visual only, not stored in content
- [x] ProseMirror trailing `<br>` hidden in empty parentheticals (keeps `()` on one line)
- [x] PDF export: parentheses wrapping in Hollywood (grouped + standalone) and Indian format
- [x] Fountain + plain text exports already had defensive wrapping
- [x] HelpModal updated with parenthetical navigation

### 16. Transition Shortcut
- [x] Cmd+Shift+T converts any element to Transition
- [x] HelpModal updated with shortcut

### 17. File Format Specification
- [x] SCREENPLAY_FORMAT.md — complete spec of .screenplay JSON format
- [x] All element types, meta fields, settings, story, scene cards documented
- [x] Sequencing rules, examples, and LLM generation notes included

---

## Phase 5 — Completed

### 18. Continuous Page View (PR #2)
- [x] Editor uses infinite scroll — single continuous page, no page breaks
- [x] ProseMirror min-height for seamless scrolling experience
- [x] Simplified Editor.svelte — removed paginated rendering logic

### 19. Menu Bar Cleanup (PR #3)
- [x] TitleBar simplified — left-pane toggle button added
- [x] Font selector, theme toggle, language controls removed from TitleBar
- [x] Controls consolidated into Settings modal

### 20. Integrated Settings Modal (PR #4)
- [x] SettingsModal.svelte — consolidated language, keyboard scheme, font, theme
- [x] Opens from gear icon in editor status bar (bottom-left popup)
- [x] Keyboard scheme dropdown shown only when Malayalam mode is active
- [x] Segmented controls for font and theme selection

### 21. Window & CI Improvements
- [x] Window launches maximized instead of fullscreen (fixes Windows taskbar issue)
- [x] Rust dependency caching in GitHub Actions release workflow
- [x] Hiran Venugopalan added as developer in About modal

---

## Phase 6 — Completed (v0.6.x → v0.7.0)

### 22. In-app Updates
- [x] `Help → Check for Updates` menu item with non-intrusive `UpdateToast`
- [x] `updateStore.svelte.ts` performs the version check on demand
- [x] Toast z-index lowered below modals (#56)

### 23. Theme & Typography
- [x] Kerala palette — teal accent, amber dirty-indicator, oxblood error tones (#69)
- [x] Courier Prime + new typography hierarchy (#66, #70) — UI font, not embedded in PDFs
- [x] Subtle fractal grain on the screenplay page (#68)
- [x] Cool find-match highlight, raised page depth, SVG drag handle (#62, #64, #65)

### 24. Editor Polish
- [x] Floating B/I/U bubble above selection (`FormatBubble.svelte`, #71)
- [x] Visual signals in Scene Navigator — INT/EXT, DAY/NIGHT, notes (#72)
- [x] Signature scene-number gutter (#67)
- [x] Outline Peek strip at the bottom of the editor (#75)
- [x] Mid-scene transitions — Enter after Transition creates Action (montage support)
- [x] Parenthetical parens stored in content, not CSS (#59 / commit 27a126f)
- [x] Required title validation in MetadataModal (#61)
- [x] Document Properties moved from View → File menu (#77)

### 25. Command Palette & Status Bar
- [x] ⌘K Command Palette with fuzzy search (#76)
- [x] Quieter status bar (#76) — view-switcher shortcuts on hover (#74)
- [x] "Saved N min ago" indicator (#73)
- [x] Symmetric view-switcher tabs

### 26. Performance
- [x] Consolidated gutter RAF chain + resize observer (#63)
- [x] Event-driven input mode (replaced 200ms polling, #60)

### 27. Web Series Support
- [x] Series data model + `ProjectType::Film | Series` enum
- [x] Active-episode accessors on `documentStore` — `activeContent`, `activeMeta`,
      `activeSettings`, `activeStory`, `activeSceneCards`, `activeEpisode`,
      `activeEpisodeIndex`
- [x] `SeriesEpisodeList.svelte` — episode tree with rename/reorder/delete
- [x] `SeriesTitleDialog.svelte` — new-series prompt
- [x] Series-aware: Statistics, OutlinePeek, MetadataModal, ExportModal
- [x] Series-level title page in exports
- [x] Synthetic `episode_boundary` ProseMirror node for inter-episode pagebreaks
- [x] Smooth slide animation when switching active episode
- [x] Scene-card character extras keyed by flat `scene_index` across episodes

### 28. Issue-review batch (#78–#97)
- [x] Series export in backend commands (#78)
- [x] StatisticsModal / OutlinePeek read activeContent (#79, #80)
- [x] Scene-card extras keying in series PDF (#81)
- [x] Hardcoded colors → theme tokens for light theme (#82)
- [x] Listener leak on mount error (#83)
- [x] ExportModal episode breadcrumb (#84)
- [x] MetadataModal live-resync on episode switch (#85)
- [x] Series-level title page (#86)
- [x] New episode inherits current font (#87)
- [x] Modal focus restoration (#88)
- [x] Modal close-button spec unified (#89)
- [x] AboutModal error handling (#90)
- [x] Drop `Result` from infallible `new_screenplay` (#91)
- [x] PDF renderer logs unknown elements (#92)
- [x] `SceneCard.scene_index` semantics documented on Rust + TS sides (#93)
- [x] SceneNavigator icon-button aria-labels (#94)
- [x] Episode label width in TitleBar (#95)
- [x] Clippy pedantic warnings in pdf.rs (#96)
- [x] Single `DEFAULT_FONT` const (#97)

### 29. Release engineering
- [x] All four platforms ship signed/notarized installers (macOS arm64, macOS x64,
      Windows, Linux deb/AppImage/rpm) via tauri-action matrix build
- [x] `update-downloads.yml` workflow auto-refreshes `docs/downloads.json` on release
- [x] `cargo clippy` + `npx svelte-check` at zero warnings (gate)

---

## Remaining Work

### Medium Term
- [ ] Revision mode — track changes per draft, asterisk marks in margin, Hollywood color cycle
- [ ] Draft history — save snapshots on each save, restore from history, max 50 per file

---

## Deferred (Do Not Implement Yet)

- FDX (Final Draft XML) export
- Rachana font / traditional Malayalam orthography
- Import from Final Draft / Fountain
- Real-time collaboration
- Cloud sync
- Mobile support
