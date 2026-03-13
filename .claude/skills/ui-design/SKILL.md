---
name: scriptty-ui-design
description: UI/UX design skill for Scriptty — offline Malayalam/English screenwriting desktop app. Use this skill for any frontend design work: revamping components, adding dark/light mode, styling the editor, TitleBar, SceneNavigator, modals, status bar, or any visual element of the app.
---

# Scriptty UI Design Skill

## App Identity

Scriptty is an offline desktop screenwriting app for Malayalam and English writers. The identity is:
- **Literary, not corporate** — this is a writer's tool, not a productivity SaaS
- **Kerala-rooted** — the primary user is a Malayalam filmmaker or writer; the aesthetic can draw from Kerala's visual culture: deep greens, warm terracotta, ink blacks, aged paper whites
- **Distraction-free, text-first** — the screenplay on screen is the hero; chrome exists only to serve it
- **Craft-forward** — the UI should feel like a well-made object, not a template

The app icon is ഋ inside a clapperboard, teal and dark. All UI decisions should feel consistent with this identity.

---

## Design Philosophy

### Core Principles (in priority order)
1. **The screenplay is the UI** — the page/document area must always feel like the primary element. Everything else is peripheral.
2. **Reduce cognitive load** — writers shouldn't think about the interface while writing. Hide complexity; surface only what's needed.
3. **Every pixel earns its place** — no decorative elements. If a button exists, it must be obviously useful. If a label exists, it must say something.
4. **Dark and light modes are both first-class** — not a palette swap. Each mode has its own character.

### Anti-patterns to avoid (learned from research)
- Final Draft: dated icons, unpleasant colors, cluttered toolbar, too many visible options at once
- Generic SaaS dark mode: purple gradients, neon accents, glassy cards — wrong context entirely
- Pure black (#000000) backgrounds — causes glare and haloing on most screens
- Pure white (#ffffff) text on dark — too harsh for extended writing sessions
- Flat icon buttons without labels in a desktop writing app — writers aren't gamers; label your actions

---

## Color System

### Dark Mode (default)
```css
/* Surfaces — layered, never flat black */
--surface-base: #1a1a1a;        /* Main window background */
--surface-elevated: #222222;    /* TitleBar, panels */
--surface-float: #2a2a2a;       /* Modals, dropdowns */
--surface-hover: #303030;       /* Button hover states */
--surface-active: #383838;      /* Button pressed */

/* The screenplay page — warm, paper-like, not pure white */
--page-bg: #f5f0e8;             /* Warm cream — like aged paper */
--page-shadow: rgba(0,0,0,0.5); /* Drop shadow under page */

/* Text — off-white hierarchy, never pure white */
--text-primary: #e8e6e1;        /* Main UI text — warm off-white */
--text-secondary: #9e9a94;      /* Labels, secondary info */
--text-muted: #5e5a55;          /* Disabled, placeholder */
--text-on-page: #1a1a1a;        /* Text inside the screenplay page */

/* Accent — teal, consistent with app icon */
--accent: #2d9b8a;              /* Primary accent — teal */
--accent-hover: #35b5a2;        /* Teal hover */
--accent-muted: rgba(45,155,138,0.15); /* Teal background wash */

/* State colors */
--dirty: #e8a04a;               /* Unsaved changes indicator — warm amber */
--error: #c0574a;               /* Error state */
--success: #4a9e6e;             /* Success */

/* Borders */
--border-subtle: rgba(255,255,255,0.07);
--border-medium: rgba(255,255,255,0.12);
```

### Light Mode
```css
/* Surfaces */
--surface-base: #f0ede8;        /* Warm off-white base — not clinical white */
--surface-elevated: #e8e4de;    /* TitleBar, panels — slightly darker */
--surface-float: #faf8f5;       /* Modals — lightest surface */
--surface-hover: #dedad4;       /* Hover */
--surface-active: #d2cdc7;      /* Pressed */

/* The screenplay page in light mode */
--page-bg: #ffffff;             /* Pure white page in light mode */
--page-shadow: rgba(0,0,0,0.12);

/* Text */
--text-primary: #1a1916;        /* Near-black, warm undertone */
--text-secondary: #5c5852;      /* Secondary */
--text-muted: #9c9891;          /* Disabled */
--text-on-page: #1a1a1a;

/* Accent — same teal, slightly darker for light bg contrast */
--accent: #1e8070;
--accent-hover: #237a6a;
--accent-muted: rgba(30,128,112,0.1);

/* State */
--dirty: #c47f28;
--error: #a83c30;
--success: #2e7d52;

/* Borders */
--border-subtle: rgba(0,0,0,0.08);
--border-medium: rgba(0,0,0,0.14);
```

### Theme Toggle Implementation
- Store in `documentStore` or a separate `themeStore` using `$state`
- Apply as a `data-theme="dark"` or `data-theme="light"` attribute on `<html>` or `<body>`
- CSS variables scoped via `[data-theme="dark"]` and `[data-theme="light"]`
- Toggle button in TitleBar — sun/moon icon with text label "Light" / "Dark"
- Persist preference to `localStorage` under key `scriptty-theme`
- On mount, read `localStorage` first, then fall back to `prefers-color-scheme`

---

## Typography

### UI Typography (TitleBar, status bar, panels)
- **Font**: `system-ui, -apple-system` — native macOS San Francisco for all UI chrome
- Never use web fonts for UI labels — they slow rendering and look wrong on macOS
- Size scale:
  - `11px` — status bar labels, scheme selector, muted metadata
  - `12px` — standard button labels, navigator scene text
  - `13px` — primary UI text, modal field labels
  - `15px` — modal headings, section titles

### Screenplay Page Typography
- Malayalam content: Noto Sans Malayalam or Manjari (user-selectable, already implemented)
- English content in screenplay: `'Courier Prime', 'Courier New', monospace` — standard screenplay font
- Page text sizes follow Hollywood spec (12pt Courier = ~16px at 96dpi)
- Line height on page: `1.6` minimum — critical for Malayalam rendering

### Typography Rules for Dark Mode
- Body text: `--text-primary` (#e8e6e1) — warm off-white, NOT pure white
- Secondary text: `--text-secondary` — use for labels, not body
- Avoid italic emphasis in dark UI — use `font-weight: 500` instead
- Letter spacing for status bar labels: `0.04em` — improves readability at 11px

---

## Layout Architecture

```
┌─────────────────────────────────────────────────────────┐
│  TitleBar (40px, draggable center zone)                 │
│  [New][Meta][Open] ··· [title] ··· [Font][Export][Save] │
├──────────────┬──────────────────────────────────────────┤
│              │                                          │
│  Scene       │      Editor Area                         │
│  Navigator   │   ┌─────────────────────┐               │
│  (240px)     │   │                     │               │
│              │   │   Screenplay Page   │               │
│  [Scene 1]   │   │   (680px wide)      │               │
│  [Scene 2]   │   │                     │               │
│  [Scene 3]   │   └─────────────────────┘               │
│              │                                          │
├──────────────┴──────────────────────────────────────────┤
│  Status Bar (28px)                                       │
│  [ENGLISH/MALAYALAM] [Mozhi|Inscript2|Inscript1] [element]│
└─────────────────────────────────────────────────────────┘
```

### TitleBar
- Height: `40px`
- Background: `--surface-elevated`
- Bottom border: `1px solid --border-subtle`
- `-webkit-app-region: drag` on center title zone ONLY — not on button zones
- Button groups: left group (document actions), right group (export/save)
- Buttons: `28px` height, `8px 12px` padding, `6px` border-radius
- Button style: ghost by default (no background), subtle hover fill (`--surface-hover`)
- Primary action (Save): filled with `--accent` background
- Separator between button groups: `1px solid --border-subtle`, `16px` vertical margin

### Scene Navigator
- Width: `220px`, fixed
- Background: `--surface-base` in dark, slightly lighter than main in light
- Right border: `1px solid --border-subtle`
- Scene items: `32px` height, `12px 16px` padding
- Active scene: `--accent-muted` background, `--accent` left border `2px`
- Scene number: `--text-muted`, `11px`, monospaced — `font-variant-numeric: tabular-nums`
- Scene text: `--text-secondary`, `12px`, truncated with ellipsis

### Editor Area
- Background: `--surface-base`
- Page centered, max-width `680px`, margin `auto`
- Page padding: `60px 72px` (standard screenplay margins)
- Page background: `--page-bg`
- Page shadow: `0 4px 24px --page-shadow, 0 1px 4px rgba(0,0,0,0.2)`
- Top/bottom page padding from viewport edge: `40px`
- Scroll behavior: smooth, scrollbar styled (thin, muted color)

### Status Bar
- Height: `28px`
- Background: `--surface-elevated`
- Top border: `1px solid --border-subtle`
- All text: `11px`, `letter-spacing: 0.04em`, uppercase
- Language indicator: colored — teal for MALAYALAM, muted for ENGLISH
- Scheme selector: only visible in Malayalam mode (already implemented — keep this)
- Element type: right-aligned, `--text-muted`

---

## Component Specifications

### Buttons

#### Ghost Button (default for most toolbar actions)
```css
.btn-ghost {
  height: 28px;
  padding: 0 10px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  transition: background 120ms ease, color 120ms ease;
}
.btn-ghost:hover {
  background: var(--surface-hover);
  color: var(--text-primary);
}
.btn-ghost:active {
  background: var(--surface-active);
}
```

#### Primary Button (Save)
```css
.btn-primary {
  background: var(--accent);
  color: white;
  /* same sizing as ghost */
}
.btn-primary:hover {
  background: var(--accent-hover);
}
```

#### Icon Button (for theme toggle, collapse panel)
```css
.btn-icon {
  width: 28px;
  height: 28px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  /* same hover/active as ghost */
}
```

#### Segmented Control (font selector, scheme selector)
```css
.segmented {
  display: flex;
  background: var(--surface-base);
  border-radius: 6px;
  padding: 2px;
  gap: 1px;
}
.segmented-item {
  padding: 3px 8px;
  border-radius: 4px;
  font-size: 11px;
  color: var(--text-muted);
  cursor: pointer;
  transition: background 100ms, color 100ms;
}
.segmented-item.active {
  background: var(--surface-elevated);
  color: var(--text-primary);
}
```

### Modal
- Backdrop: `rgba(0,0,0,0.6)` with `backdrop-filter: blur(4px)`
- Modal card: `--surface-float`, `12px` border-radius, `1px solid --border-medium`
- Width: `480px`, never full-screen on desktop
- Padding: `24px`
- Header: title `15px` semibold, close button top-right
- Fields: `--surface-base` background, `--border-medium` border, `8px` border-radius, `12px` padding
- Field focus: `--accent` border color, no box-shadow glow
- Footer buttons: right-aligned, Cancel (ghost) + Save (primary)
- Transition: fade + scale(0.97 → 1.0), 150ms ease-out

### Dirty Indicator
- Small dot `6px` diameter in the title zone
- Color: `--dirty` (#e8a04a) when unsaved
- Transparent when saved
- No text label needed — the dot is sufficient convention

---

## Interaction Patterns

### Theme Toggle
- Place in TitleBar right group, leftmost icon before font selector
- Sun icon (☀) in dark mode, moon icon (🌙) in light mode — or use text "Light"/"Dark"
- Instant CSS variable swap, no flash — use `transition: background 200ms, color 200ms` on `body`
- Do NOT animate the screenplay page background color — it causes jarring page flicker

### Navigator Collapse
- Ctrl+B (already implemented) — keep shortcut
- Collapse button (‹/›) at top of navigator
- Collapsed state: navigator is `0px` width with `overflow: hidden`, editor expands to full width
- Transition: `width 200ms cubic-bezier(0.4, 0, 0.2, 1)`

### Focus Mode (optional, Phase 2)
- Hide TitleBar and status bar, expand editor full window
- Triggered by Cmd+Shift+F or a button in TitleBar

### Hover States
- All interactive elements must have visible hover state
- Transition duration: `120ms` — fast enough to feel snappy, not jarring
- Never use opacity-only hover — change background instead

---

## Svelte Implementation Notes

### Theme Store Pattern
```typescript
// src/lib/stores/themeStore.svelte.ts
const STORAGE_KEY = 'scriptty-theme'
type Theme = 'dark' | 'light'

function getInitialTheme(): Theme {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored === 'dark' || stored === 'light') return stored
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

class ThemeStore {
  current = $state<Theme>('dark')
  
  init() {
    this.current = getInitialTheme()
    this.apply()
  }
  
  toggle() {
    this.current = this.current === 'dark' ? 'light' : 'dark'
    localStorage.setItem(STORAGE_KEY, this.current)
    this.apply()
  }
  
  apply() {
    document.documentElement.setAttribute('data-theme', this.current)
  }
}

export const themeStore = new ThemeStore()
```

### CSS Variable Scoping
```css
/* In app.css or +layout.svelte <style> */
[data-theme="dark"] {
  --surface-base: #1a1a1a;
  --text-primary: #e8e6e1;
  /* ... all dark tokens */
}

[data-theme="light"] {
  --surface-base: #f0ede8;
  --text-primary: #1a1916;
  /* ... all light tokens */
}

/* Default to dark if no attribute set */
:root {
  --surface-base: #1a1a1a;
  /* ... dark as default */
}
```

### ProseMirror Styling
All screenplay page styles MUST use `:global()` because ProseMirror generates its own DOM:
```css
:global(.ProseMirror) {
  color: var(--text-on-page);
  font-family: 'Courier Prime', 'Courier New', monospace;
  line-height: 1.6;
}
:global(.ProseMirror .scene-heading) {
  font-weight: bold;
  text-transform: uppercase;
  /* etc */
}
```

### Transitions
Body-level theme transition (prevents flash on toggle):
```css
body {
  transition: background-color 200ms ease, color 200ms ease;
}
/* Exclude page content from transition to prevent flicker */
.screenplay-page {
  transition: none !important;
}
```

---

## What NOT to Do

- Do not use `box-shadow: 0 0 20px var(--accent)` glow effects — wrong aesthetic
- Do not use gradient backgrounds in the app chrome — reserve gradients only for decorative splashes
- Do not add icons to every button — text labels are fine and clearer in a writing app
- Do not use `border-radius > 8px` on rectangular controls — looks toy-like
- Do not animate the screenplay page itself — only animate chrome elements
- Do not use more than 2 accent colors — teal is the accent, amber is for dirty state only
- Do not use Inter or system fonts for the screenplay page — must be monospace Courier
- Do not make the TitleBar taller than 42px — wastes vertical space on a writing app
- Do not add loading spinners for local operations — everything in Scriptty is instant/offline

---

## Reference: What the Best Apps Do

**Highland 2 strengths** (emulate these):
- Nothing cluttered — every element has a reason to be there
- Light, modern, text-first
- The navigator is clean and doesn't compete with the document

**Arc Studio Pro strengths** (emulate these):
- Clean visual design removes distractions
- Night mode feels purposefully designed, not just dark
- Element type indicators are subtle and don't interrupt writing flow

**What to avoid** (from Final Draft criticism):
- Dated icons
- Unpleasant color palette
- Too many options visible simultaneously
- Design that makes the writer think about the software instead of the story
