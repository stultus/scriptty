<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="128" height="128" alt="Scriptty logo">
</p>

# Scriptty

[![Build & Release](https://github.com/stultus/scriptty/actions/workflows/release.yml/badge.svg)](https://github.com/stultus/scriptty/actions/workflows/release.yml)

An offline desktop screenwriting app for Malayalam and English screenwriters.

Built with Tauri 2, SvelteKit, TypeScript, ProseMirror, and Typst.

---

## Features

- **Film and web series projects** — one `.screenplay` file holds either a single
  screenplay or an entire series with multiple episodes
- Hollywood single-column and Indian two-column PDF export with auto-generated
  title pages (smart "Written and Directed by" credits)
- Fountain and plain-text export for interoperability
- Malayalam input — Mozhi, Inscript 1, and Inscript 2 schemes; `Ctrl+Space`
  toggles English/Malayalam mid-line
- Mixed-script writing — Malayalam and English in the same document
- ⌘K Command palette — fuzzy-search every command and scene heading
- Scene Navigator — click-to-jump, auto-numbering, drag-to-reorder, INT/EXT/DAY/NIGHT signals
- Outline Peek — bottom strip showing scene context as you write
- Story panel + full-screen Story Mode — idea, synopsis, treatment, narrative
- Scene Cards — per-scene shoot-planning grid with auto-detected characters
- Find & replace, statistics (page/scene/word/dialogue counts), character autocomplete
- Bundled fonts — Manjari (default), Noto Sans Malayalam, Courier Prime
- Warm Kerala-rooted theme — dark and light modes
- Fully offline — no cloud, no telemetry, no subscription, no account
- `.screenplay` file format — plain JSON, version-control friendly

## Supported Platforms

Signed installers shipped for every release (see [Releases](https://github.com/stultus/scriptty/releases)):

- macOS — Apple Silicon and Intel `.dmg`
- Windows — `.exe` and `.msi`
- Linux — `.deb`, `.AppImage`, and `.rpm`

## Development Setup

Prerequisites: Rust 1.77+, Node.js 22, npm

```bash
git clone https://github.com/stultus/scriptty.git
cd scriptty
npm install
npx tauri dev
```

## Build
```bash
npx tauri build
```

Output: `src-tauri/target/release/bundle/`

## Tech Stack

- Frontend: SvelteKit + Svelte 5 (runes) + TypeScript + ProseMirror
- Backend: Tauri 2 + Rust
- PDF: Typst (in-memory compilation, no temp files)
- Fonts: Manjari, Noto Sans Malayalam, Courier Prime (all SIL OFL 1.1)

## License

MIT — see LICENSE
