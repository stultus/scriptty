# Scriptty — Development Progress

## Status: v0.10.0 shipped — Fountain & Final Draft interop, consolidated import wizard

Highlights since v0.8.0:

- **Final Draft (`.fdx`) import** (v0.10.0). Hand-rolled XML parser using `quick-xml`. The six native paragraph types map directly; `Shot` folds to scene heading, `General` / `Lyrics` / `Outline N` fold to action. Inline `<Text Style="Bold+Italic+Underline">` runs become ProseMirror marks. `<DualDialogue>` collapses to sequential pairs. `<ScriptNote>`, `<TagData>`, revisions, locked numbers, headers/footers, page layout drop with summary counts. Title-page text always lands in `meta.extra["fdx_title_page"]`; a Beat-style heuristic best-effort fills the standard meta fields. The FDX `Version` attribute lands in `meta.extra["fdx_source_version"]`.
- **Round-trip-safe Fountain import + export** (v0.9.0 + v0.10.0). Full Fountain spec parser. Synopses absorb into scene-card descriptions; sections attach to the next scene's `shoot_notes` with a `[[#section depth=N]]` marker; inline `[[ ]]` notes attach to the containing scene. The export side applies forcing rules (`@` for non-all-caps-Latin character cues, `.` for non-slug scene headings, `>` for non-`TO:` transitions, `!` for action that would auto-detect as anything else) so a co-writer can edit a Scriptty-touched file and round-trip it back without silent corruption. Non-standard title-page keys round-trip via the new `meta.extra: BTreeMap<String, String>` schema field.
- **Single Import Screenplay wizard** (v0.10.0). One File-menu entry replaces four format-and-destination items. Centered-card modal with editorial-vocabulary header, format radio cards (Fountain / Final Draft) and destination radio cards (new film / episode of active series — disabled with explanatory sub-line when no series open). The standard `Cmd+O` Open dialog also accepts `.fountain` and `.fdx` directly.
- **Per-episode Fountain export** for Series projects (v0.9.0). Toggle in the Export modal; pick a folder, get `01-pilot.fountain`, `02-the-return.fountain`, …
- **CI gating** (v0.9.0). New `.github/workflows/ci.yml` runs `cargo clippy --lib --tests -- -D warnings`, `cargo test --lib`, and `npm run check -- --fail-on-warnings` on push/PR to main. Caught and repaired 32 pre-existing stale tests in `pdf.rs` from earlier struct refactors. Update-Download-Links workflow hardened against the duplicate-trigger race that surfaced spurious failures on every release (concurrency group + rebase-retry).

Highlights from v0.8.0 (still relevant):

- **Production planning end-to-end.** Scene cards carry a location group, shoot date, and extras list. Daily Shoot List PDF groups scenes by day → location with industry-standard page-eighths totals. Statistics panel gains Schedule and Episodes views, sortable columns, and CSV export across Characters / Locations / Schedule.
- **Editorial-grade PDF redesign.** Title page, prose covers, scene-card cover, and shoot-list cover share one masthead vocabulary. Per-section page numbering. Transition widow control. Courier Prime now bundled into PDFs alongside the body font for accent typography.
- **Episode Breakout view.** Series projects get a top-level card per episode with a scene preview list. IDE-style episode explorer in the sidebar with per-episode status (Outline / Draft / Revision / Final).
- **Editor.** Smart curly quotes. Adjustable editor font size. Autosave + crash recovery — a hidden recovery file survives power loss. Paste-to-script — convert plain text into a screenplay (Hollywood-style detection plus a Malayalam-aware character-cue path).
- **Title bar.** Colophon-style "Scriptty" wordmark with press-mark + hairline rule. Centred title gets flanking middle-dot ornaments. Episode badge becomes a click-to-switch popover. Metadata icon button. View-switcher tabs gain leading element-type glyphs.

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

### 30. Fountain import + round-trip (v0.9.0, #184)

- [x] `meta.extra: BTreeMap<String, String>` schema field for non-standard
      title-page keys (#185)
- [x] Hand-rolled Fountain parser in `src-tauri/src/screenplay/fountain_import.rs`
      following the canonical reference parser's precedence (boneyard pre-pass,
      note extraction, title page, body state machine) (#186)
- [x] Synopses → `scene_cards[].description`; sections → `shoot_notes` with
      `[[#section depth=N]]` marker; inline `[[ ]]` notes → `shoot_notes`
- [x] Boneyard / dual dialogue / emphasis dropped with summary counts; warning
      surfaced when a file looks Malayalam-heavy without `@`-prefixed cues
- [x] Round-trip-safe export with forcing rules: `@` for caseless / mixed-case
      character cues, `.` for non-slug scene headings, `>` for non-`TO:`
      transitions, `!` for action that would auto-detect as anything else (#188)
- [x] `meta.extra` keys emit alphabetically (BTreeMap ordering) for diff-stable
      output; `meta.registration_number` ↔ `Copyright:`; `meta.footnote` ↔
      `Notes:`
- [x] Tauri commands `import_fountain_as_film` / `import_fountain_as_episode`
      returning `{ document, summary }`; `ImportSummaryToast` renders the
      summary (#187)
- [x] Per-episode Fountain export for Series projects (one `.fountain` per
      episode in a chosen directory)
- [x] 65 unit tests across `fountain_import` (36) and `fountain` (29) including
      round-trip fixed-point tests

### 31. Final Draft (FDX) import + import wizard (v0.10.0, #190)

- [x] `quick-xml` (MIT, pure-Rust) added as a dependency; FDX parser in
      `src-tauri/src/screenplay/fdx_import.rs` (#191)
- [x] Six native paragraph types map directly; `Shot` folds to scene heading;
      `General` / `Lyrics` / `Outline N` fold to action; unknown types fold
      to action with a count
- [x] Inline `<Text Style="Bold+Italic+Underline">` runs map to ProseMirror
      bold / italic / underline marks; Strikeout / AllCaps / Highlight drop
- [x] `<DualDialogue>` collapses to sequential pairs (counted)
- [x] `<ScriptNote>` / `<TagData>` / locked scene `Number=` / revisions /
      headers-footers / page-layout drop with counts surfaced in the toast
- [x] Title-page heuristic: full text → `meta.extra["fdx_title_page"]`,
      Beat-style guess fills `meta.title` / `.author` / `.draft_date` /
      `.contact` when the layout is recognisable; FDX `Version` →
      `meta.extra["fdx_source_version"]`
- [x] Tauri commands `import_fdx_as_film` / `import_fdx_as_episode` (#192);
      `ImportSummaryToast` generalised over a discriminated-union summary
      so one component renders both Fountain and FDX counts
- [x] Single `Import Screenplay…` File-menu entry + command-palette entry
      replace the four format/destination items (#192 follow-up)
- [x] `ImportWizardModal.svelte` — centered-card wizard picking format
      (Fountain / Final Draft) and destination (new film / episode of active
      series); disabled state for the episode card when no series is open
- [x] Open dialog accepts `.fountain` and `.fdx` alongside `.screenplay` and
      auto-routes by extension
- [x] 22 FDX unit tests covering native types, inline marks, dual-dialogue
      collapse, type folding, drops, title-page heuristic, XML entity decoding,
      UTF-8 BOM stripping, Malayalam pass-through

### 32. CI hardening

- [x] New `.github/workflows/ci.yml` — gates `cargo clippy --lib --tests
    -- -D warnings`, `cargo test --lib`, `npm run check --
    --fail-on-warnings` on push/PR to main (#189)
- [x] Repaired 32 pre-existing stale tests in `pdf.rs` (struct refactors
      from #103 had drifted past the test code)
- [x] `update-downloads.yml` race fix — `release` event was firing the
      workflow on both `published` and `released` types, both runs racing
      on `git push`. Trigger narrowed to `published`, concurrency group
      added, push step does up to 3 rebase-retries.

---

## Remaining Work

### Medium Term

- [ ] Revision mode — track changes per draft, asterisk marks in margin, Hollywood color cycle
- [ ] Draft history — save snapshots on each save, restore from history, max 50 per file
- [ ] FDX (Final Draft XML) **export** — currently we import FDX but don't export
      it. Lower-priority than import: Fountain is the canonical co-writing
      handoff, and FDX export would mean choosing a Final Draft template
      version to target.

---

## Deferred (Do Not Implement Yet)

- Rachana font / traditional Malayalam orthography
- Real-time collaboration
- Cloud sync
- Mobile support
- `.fdr` (legacy Final Draft binary) import — FD 1–7 era, stopped being
  written in 2009. `.fdx` import covers any modern Final Draft user.
