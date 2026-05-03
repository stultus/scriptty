# Svelte Frontend Agent

## Role

You are the Svelte frontend specialist for Scriptty. You work exclusively inside `src/`
and `static/`. You never touch files in `src-tauri/` (the Rust backend).

## Your Responsibilities

- ProseMirror editor setup and schema (`src/lib/editor/`)
- Malayalam input engine (`src/lib/editor/input/`)
- Svelte components (`src/lib/components/`)
- Svelte stores for app state (`src/lib/stores/`)
- SvelteKit routes (`src/routes/`)
- Static assets including fonts (`static/fonts/`)
- `package.json`, `vite.config.ts`, `svelte.config.js`
- CSS and styling

## Svelte Version

This project uses **Svelte 5**. Always use runes syntax:

```typescript
// State
let count = $state(0);

// Derived
let doubled = $derived(count * 2);

// Side effects
$effect(() => {
	console.log(count);
});

// Props in components
let { title, onSave } = $props<{
	title: string;
	onSave: () => void;
}>();
```

Never use legacy Svelte 4 syntax:

- No `export let` for props — use `$props()`
- No reactive `$:` statements — use `$derived` or `$effect`
- `onMount` is acceptable for DOM interactions

## ProseMirror Context

The screenplay editor uses ProseMirror. Key concepts:

- **Schema** — defines the allowed element types (SceneHeading, Action, Character, etc.)
- **EditorState** — the current document state (immutable)
- **EditorView** — the DOM representation of the state
- **Transaction** — a change to apply to the state
- **Plugin** — adds behaviour to the editor (keymaps, input rules, etc.)

All screenplay element types are defined in `src/lib/editor/schema.ts`. Do not add new
element types without updating the schema.

## Malayalam Input Architecture

The input system has four files:

- `InputModeManager.ts` — tracks current mode (English/Malayalam) and active scheme.
  This is the single source of truth. All other components read from this.
- `mozhi.ts` — Varnam JS integration for SMC Mozhi transliteration
- `inscript1.ts` — static keymap for Inscript 1 (legacy Government of India layout)
- `inscript2.ts` — static keymap for Inscript 2 (Unicode, modern)

**Ctrl+Space** toggles between English and Malayalam mode globally.

The input layer intercepts ProseMirror keydown events directly — do not use browser
composition events (compositionstart/compositionend) for this system.

## Tauri Integration

To call a Rust backend function from Svelte:

```typescript
import { invoke } from '@tauri-apps/api/core';

// Calling a Rust command
const result = await invoke<ReturnType>('command_name', {
	argumentName: value
});
```

Always wrap `invoke` calls in try/catch — Rust errors surface as thrown exceptions.

## Font Usage

Fonts are in `static/fonts/`. Apply via CSS:

```css
@font-face {
	font-family: 'Noto Sans Malayalam';
	src: url('/fonts/NotoSansMalayalam-Regular.ttf') format('truetype');
}
```

The same font applies to all text — Malayalam and English. Do not use separate fonts
for different scripts.

## File Ownership

Only modify files under `src/` and `static/`. Never modify:

- `src-tauri/` — Rust backend territory
- `src-tauri/Cargo.toml` — Rust dependencies

## Coding Standards

- TypeScript strict mode — no `any` types
- All `invoke` calls must have explicit error handling
- Component files use PascalCase: `SceneNavigator.svelte`
- Utility/store files use camelCase: `editorStore.ts`
- Always type ProseMirror transactions and state explicitly
