# Scriptty — Product Requirements Document
**Version:** 1.0
**Date:** March 2026
**Author:** Hrishikesh Bhaskaran

---

## 1. Product Overview

Scriptty is an offline desktop screenwriting application for Malayalam and English screenwriters. It targets writers working in Indian vernacular languages, specifically the Malayalam film industry, while remaining fully functional for English-only and mixed-language screenwriting.

Every feature must work without an internet connection. There is no user account, no telemetry, and no subscription. A single `.screenplay` file contains everything related to a project.

### 1.1 Design Principles

- Offline first. No network dependency for any feature.
- Single file per project.
- Malayalam input must work in every text field in the application.
- The editor is the primary surface. UI chrome is minimal.
- Output must conform to accepted industry standards for script formatting.

---

## 2. Current Features (v0.1.0)

The following are already built and must not regress.

- Screenplay editor with six element types: Scene Heading, Action, Character, Dialogue, Parenthetical, Transition
- Tab/Enter keyboard navigation between elements following Hollywood conventions
- Auto-uppercase for scene headings and character names (Latin only; Malayalam passthrough)
- Malayalam input via Mozhi (default), Inscript 1, and Inscript 2
- Ctrl+Space toggle between English and Malayalam mid-sentence
- Input scheme switcher in the status bar
- Scene Navigator — collapsible left panel, auto-numbered, click to jump
- File management — New, Open, Save, Save As with native dialogs
- .screenplay file format — single JSON file per project
- Dirty state indicator
- Document metadata — title, author, contact, draft number, draft date
- Font switching — Noto Sans Malayalam and Manjari
- PDF export — Hollywood single-column
- PDF export — Indian two-column (character name right-aligned left column, dialogue left-aligned right column)
- Title page auto-generated from metadata when title is non-empty
- Native macOS menu — File (New, Open, Save, Save As, Quit) and Edit (Undo, Redo, Cut, Copy, Paste, Select All)

---

## 3. Planned Features

### 3.1 Story Panel

**Purpose:** Give writers a structured space to develop their story before and alongside writing the script.

**Three sections, in order of granularity:**

**Idea**
A short field for the core premise. One to five sentences. This is the seed of the story — the writer's first articulation of what the film is about.

**Synopsis**
A prose summary of the full story from beginning to end. Typically 300–800 words. This is a shareable document — what a writer would send to a producer or attach to a funding application.

**Treatment**
A detailed narrative prose version of the story, written scene by scene. This is a personal creative document for the writer's own clarity. No length limit.

**Behavior:**
- All three sections are plain text fields. No screenplay formatting.
- Malayalam input (Ctrl+Space toggle) must work in all three sections.
- Content is saved as part of the `.screenplay` file.
- Sections are accessible from a Story tab in the left panel, alongside the existing Scene Navigator tab.
- The left panel shows either the Scene Navigator or the Story Panel — not both at the same time. Switching between them via tabs at the top of the panel.
- Each section is collapsible independently.

---

### 3.2 Scene Cards

**Purpose:** A structured per-scene breakdown to help directors and ADs plan the shoot.

**Each scene card displays:**

Auto-populated from the screenplay (derived, not manually entered):
- Scene number
- Full scene heading text
- Location (parsed from scene heading)
- Time of day (parsed from scene heading — DAY, NIGHT, DAWN, etc.)
- Characters appearing in the scene (all Character elements within the scene)
- Approximate page count for the scene

Manually entered by the writer:
- Scene description — 2 to 5 sentences about what happens in the scene
- Shoot notes — special requirements, stunts, VFX flags, location notes, equipment needs

**Behavior:**
- Scene Cards view is a separate full-panel mode, replacing the editor while active.
- Accessible from the View menu or a keyboard shortcut.
- A clearly visible button returns the writer to the screenplay editor.
- Cards are displayed in a grid layout, two or three columns depending on window width.
- Auto-populated fields are read-only.
- Description and Shoot Notes are editable inline on the card.
- Changes to scene card data are saved as part of the `.screenplay` file.
- Malayalam input must work in the Description and Shoot Notes fields.
- If scenes are added or removed in the script, the Scene Cards view reflects this automatically.

---

### 3.3 Export System

**Purpose:** Replace the current two-button export with a flexible export modal that covers all document sections.

**Export Modal:**

Triggered from a single Export button in the title bar, or from File > Export in the menu.

The modal presents checkboxes for each section to include in the export:
- Title Page (uses existing title page generation)
- Synopsis (from Story Panel)
- Treatment (from Story Panel)
- Screenplay
- Scene Cards

For the Screenplay section, the writer selects the format:
- Hollywood (single column)
- Indian (two column)

Any combination of sections can be selected. Examples:
- Synopsis only → exports a standalone synopsis PDF
- Synopsis + Screenplay → exports synopsis followed by screenplay in one PDF
- Screenplay + Scene Cards → exports script followed by scene breakdown
- All sections → complete package document

**PDF layout for each section:**

*Synopsis section:* Heading "SYNOPSIS" centered and bold at top. Author name below. Body text in readable prose layout.

*Treatment section:* Heading "TREATMENT" centered and bold at top. Body text in readable prose layout.

*Scene Cards section:* Heading "SCENE BREAKDOWN" centered and bold at top. Each scene printed with its number, heading, location, time, characters, description, and shoot notes. Formatted to be printable and usable on set.

**Menu:**
File menu gains an Export option that opens this modal. The current Hollywood and Indian buttons in the title bar are replaced by a single Export button that also opens the modal.

---

### 3.4 Character Autocomplete

**Purpose:** Speed up character name entry by suggesting previously used character names.

**Behavior:**
- When the cursor is on a Character element and the writer has typed two or more characters, a suggestion dropdown appears showing matching character names from the current screenplay.
- Names are collected from all existing Character elements in the document.
- Pressing Tab or Enter selects the highlighted suggestion and moves to the next element (Dialogue).
- Pressing Escape dismisses the dropdown.
- The suggestion list is case-insensitive and Unicode-aware (Malayalam names must match correctly).
- If no matches exist, no dropdown is shown.

---

### 3.5 Fountain Export

**Purpose:** Allow writers to share scripts in the open Fountain plain-text format, compatible with Highland, Fade In, and other screenwriting apps.

**Behavior:**
- Exports the current screenplay as a `.fountain` file.
- The Fountain file follows the Fountain specification (fountain.io).
- Scene headings, action, character, dialogue, parenthetical, and transition elements all map to their correct Fountain equivalents.
- Malayalam text is preserved as-is in the output (UTF-8 encoded).
- Metadata (title, author, contact, draft) is written as Fountain title page key-value pairs.
- Accessible from File > Export or the Export modal.

---

### 3.6 Revision Mode

**Purpose:** Track changes between drafts using the standard Hollywood revision system.

**Behavior:**
- Revision mode is toggled on and off from the Edit menu or a keyboard shortcut.
- When revision mode is on, any line that has been added or changed since the mode was activated displays a revision mark — an asterisk (*) in the right margin of that line.
- Deleted content is not shown inline; the asterisk marks the surrounding lines.
- Revision marks are visible in the editor and in exported PDFs.
- Revision color sets follow Hollywood conventions:
  - Draft 1: White (no color)
  - Draft 2: Blue
  - Draft 3: Pink
  - Draft 4: Yellow
  - Draft 5: Green
  - Draft 6: Goldenrod
  - Draft 7: Buff
  - Draft 8: Salmon
  - Draft 9: Cherry
  - Additional drafts cycle back
- The current revision color is set in the document metadata (draft number maps to revision color).
- Exported PDFs in revision mode show the revision marks in the right margin.
- Revision data is saved in the `.screenplay` file.

---

### 3.7 Draft History

**Purpose:** Allow writers to access and restore previous versions of their script.

**Behavior:**
- Every time the writer saves the document, a snapshot of the screenplay content is stored internally within the `.screenplay` file.
- The writer can access draft history from the File menu.
- The history panel shows a list of saves: date, time, and draft number.
- Selecting a historical save shows a read-only preview of that version.
- The writer can restore from any historical save, which replaces the current content with the historical version.
- A maximum of 50 historical snapshots are stored per document. Older snapshots are dropped when the limit is reached.
- Draft history does not include story panel content or scene card data — screenplay content only.

---

### 3.8 Script Statistics

**Purpose:** Give writers useful metrics about their screenplay.

**Statistics to display:**
- Total page count
- Total scene count
- Total word count
- Total dialogue line count
- Per-character statistics: number of scenes, number of dialogue lines, approximate percentage of total dialogue
- Number of interior vs. exterior scenes
- Number of day vs. night scenes
- Estimated screen time (based on page count, 1 page ≈ 1 minute)

**Behavior:**
- Accessible from a Statistics option in the View menu.
- Displayed in a panel or modal — not replacing the editor.
- Updates in real time as the writer types, or on demand with a Refresh button.
- Malayalam character names are counted correctly.

---

### 3.9 Find and Replace

**Purpose:** Standard text search and replace within the screenplay.

**Behavior:**
- Accessible via Cmd+F (find) and Cmd+H (find and replace).
- Opens a toolbar or panel at the bottom or top of the editor.
- Find highlights all matching occurrences in the document.
- Replace replaces the current match. Replace All replaces all matches in the document.
- Case-sensitive option.
- Works correctly with Malayalam text.
- Pressing Escape or clicking outside closes the find bar.

---

### 3.10 Help and About Menu

**Purpose:** Standard macOS Help menu with application information and credits.

**Help Menu items:**
- About Scriptty — opens the About dialog
- Report an Issue — opens the GitHub issues page in the default browser
- View on GitHub — opens the GitHub repository in the default browser

**About Dialog:**

A modal dialog with the following content:
- Scriptty app icon at approximately 80px
- App name: Scriptty
- Version number (current build version)
- Tagline: "Offline screenwriting for Malayalam and English writers"
- Separator line
- Developer: Hrishikesh Bhaskaran (stultus)
- Email: hello@stultus.in
- Website: stultus.in
- Separator line
- Section heading: Inputs and Feedback
- Names: Abraham Joseph (Abrooz), Aswin Raveendran, Sanjay Krishna, Aashiq Abu (director)
- Separator line
- MIT License notice with year 2026
- Close button

Styling matches the application's dark theme.

---

## 4. Feature Priority

### Immediate (next release)
1. Help and About menu
2. Story Panel
3. Export modal
4. Character autocomplete

### Short term
5. Scene Cards
6. Find and Replace
7. Fountain export
8. Script statistics

### Medium term
9. Revision mode
10. Draft history

---

## 5. Constraints

- All features are offline. No network calls for any core functionality.
- All text input areas must support Malayalam via the existing input method system.
- All new document data is saved in the existing `.screenplay` file format.
- PDF export uses the existing Typst pipeline.
- The application must remain a single-window experience. No floating panels or secondary windows except for modal dialogs.
- macOS is the primary target platform. All keyboard shortcuts follow macOS conventions (Cmd key).
