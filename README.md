<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="128" height="128" alt="Scriptty logo">
</p>

# Scriptty

An offline desktop screenwriting app for Malayalam and English screenwriters.

Built with Tauri 2, SvelteKit, TypeScript, ProseMirror, and Typst.

---

## Features

- Hollywood single-column and Indian two-column PDF export
- Malayalam input — Mozhi, Inscript 1, and Inscript 2 schemes
- Mixed-script writing — Malayalam and English in the same document
- Scene navigator with click-to-jump and auto-numbering
- Bundled fonts — Noto Sans Malayalam and Manjari
- Fully offline — no cloud, no telemetry, no subscription
- .screenplay file format — plain JSON, version-control friendly

## Supported Platforms

- macOS (primary)
- Windows and Linux (Tauri 2 supported, untested)

## Development Setup

Prerequisites: Rust 1.70+, Node.js 18+, npm
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

Output: src-tauri/target/release/bundle/

## Tech Stack

- Frontend: SvelteKit + TypeScript + ProseMirror
- Backend: Tauri 2 + Rust
- PDF: Typst (in-memory compilation, no temp files)
- Fonts: Noto Sans Malayalam, Manjari (SIL OFL 1.1)

## License

MIT — see LICENSE
