<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
</script>

{@render children()}

<style>
  :global(*, *::before, *::after) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(html, body) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--surface-base);
    color: var(--text-primary);
    font-family: var(--ui-font, system-ui, -apple-system, sans-serif);
  }

  /* Body-level theme transition — prevents flash on toggle */
  :global(body) {
    transition: background-color 200ms ease, color 200ms ease;
  }

  /* ─── Dark mode (default) ─── */
  :global(:root),
  :global([data-theme="dark"]) {
    /* Surfaces — layered, never flat black */
    --surface-base: #1a1a1a;
    --surface-elevated: #222222;
    --surface-float: #2a2a2a;
    --surface-hover: #303030;
    --surface-active: #383838;

    /* The screenplay page — warm, paper-like */
    --page-bg: #f5f0e8;
    --page-shadow: rgba(0, 0, 0, 0.5);
    --page-shadow-close: rgba(0, 0, 0, 0.22);
    --page-edge-highlight: rgba(255, 255, 255, 0.04);
    /* Subtle paper grain — SVG fractal noise inlined as a data URL.
       Very low opacity so it reads as texture not pattern. Dark mode
       keeps it quiet to avoid muddying the cream page under app chrome. */
    --page-grain: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='240' height='240'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/><feColorMatrix values='0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 0.035 0'/></filter><rect width='100%25' height='100%25' filter='url(%23n)'/></svg>");

    /* Text — off-white hierarchy */
    --text-primary: #e8e6e1;
    --text-secondary: #9e9a94;
    --text-muted: #5e5a55;
    --text-caption: #9e9a94;
    --text-on-page: #1a1a1a;

    /* Accent — teal, consistent with app icon */
    --accent: #2d9b8a;
    --accent-hover: #35b5a2;
    --accent-muted: rgba(45, 155, 138, 0.15);

    /* Warm companion — amber, evokes Kerala lamp-light. Used for
       badges, save-success, subtle highlights. Same hue family as
       --dirty, so dirty indicator stays coherent. */
    --accent-warm: #e8a04a;
    --accent-warm-muted: rgba(232, 160, 74, 0.15);

    /* Deep companion — oxblood / kumkumam. Used for transitions,
       destructive confirms, and emphatic moments. */
    --accent-deep: #9b3a3a;
    --accent-deep-muted: rgba(155, 58, 58, 0.15);

    /* State colors */
    --dirty: #e8a04a;
    --error: #c0574a;
    --success: #4a9e6e;

    /* Borders */
    --border-subtle: rgba(255, 255, 255, 0.07);
    --border-medium: rgba(255, 255, 255, 0.12);

    /* Shadows and overlays */
    --shadow-soft: rgba(0, 0, 0, 0.2);
    --shadow-medium: rgba(0, 0, 0, 0.3);
    --shadow-heavy: rgba(0, 0, 0, 0.4);
    --backdrop: rgba(0, 0, 0, 0.6);

    /* Text on accent background (e.g. Save button) */
    --text-on-accent: #ffffff;

    /* Find/replace highlights */
    --find-match: rgba(255, 213, 79, 0.35);
    --find-match-current: rgba(45, 155, 138, 0.30);
  }

  /* ─── Light mode ─── */
  :global([data-theme="light"]) {
    --surface-base: #f0ede8;
    --surface-elevated: #e8e4de;
    --surface-float: #faf8f5;
    --surface-hover: #dedad4;
    --surface-active: #d2cdc7;

    --page-bg: #ffffff;
    --page-shadow: rgba(0, 0, 0, 0.18);
    --page-shadow-close: rgba(0, 0, 0, 0.08);
    --page-edge-highlight: rgba(255, 255, 255, 0.9);
    /* Light mode gets a slightly warmer grain so the white page
       reads as stock paper rather than screen. */
    --page-grain: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='240' height='240'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/><feColorMatrix values='0 0 0 0 0.08  0 0 0 0 0.06  0 0 0 0 0.03  0 0 0 0.045 0'/></filter><rect width='100%25' height='100%25' filter='url(%23n)'/></svg>");

    --text-primary: #1a1916;
    --text-secondary: #4a4740;
    --text-muted: #6b6860;
    --text-caption: #6b6860;
    --text-on-page: #1a1a1a;

    --accent: #1e8070;
    --accent-hover: #237a6a;
    --accent-muted: rgba(30, 128, 112, 0.1);

    --accent-warm: #b76d0f;
    --accent-warm-muted: rgba(183, 109, 15, 0.12);

    --accent-deep: #7a2b2b;
    --accent-deep-muted: rgba(122, 43, 43, 0.12);

    --dirty: #b76d0f;
    --error: #a83c30;
    --success: #2e7d52;

    --border-subtle: rgba(0, 0, 0, 0.14);
    --border-medium: rgba(0, 0, 0, 0.22);

    --shadow-soft: rgba(0, 0, 0, 0.1);
    --shadow-medium: rgba(0, 0, 0, 0.2);
    --shadow-heavy: rgba(0, 0, 0, 0.3);
    --backdrop: rgba(0, 0, 0, 0.5);

    --text-on-accent: #ffffff;

    --find-match: rgba(255, 200, 50, 0.4);
    --find-match-current: rgba(30, 128, 112, 0.22);
  }

  /* ─── Shared typography tokens for small form labels ───
     Used by SceneCardsView's Description/Notes labels and the annotation
     gutter in Editor.svelte so the same data reads as the same system in
     both places. */
  :global(:root) {
    --label-font-size: 10px;
    --label-font-weight: 700;
    --label-tracking: 0.06em;
    --label-color: var(--text-muted);

    /* ─── Three font roles (issue #66) ───
       --ui-font: chrome, menus, toolbars, buttons, dialogs
       --editor-font-en: Latin script inside the editor page — Courier Prime,
         the accepted typographic standard for screenplays
       --editor-font-ml: Malayalam runs — Noto Sans Malayalam / Manjari,
         set per-component to the user's selected font

       Font stacks compose per-glyph in CSS: a mixed-script run like
       "രമേഷ് Flat ലേക്ക്" renders Latin in Courier Prime and Malayalam
       in the fallback family automatically, no splitting needed.

       --editor-font-en holds a SINGLE family (no monospace fallback) so
       that Malayalam glyphs fall through to --editor-font-ml; otherwise a
       system monospace (which ships notdef/placeholder Malayalam glyphs
       on some platforms) would intercept the fallback chain. */
    --ui-font: system-ui, -apple-system, sans-serif;
    --editor-font-en: 'Courier Prime';
  }

  /* ─── Disabled button baseline ───
     Applies to every native <button> across the app so disabled state is
     always visible even if the component didn't author its own :disabled
     rule. Components may still add component-specific overrides, but this
     guarantees the baseline. `pointer-events: none` also neutralizes any
     :hover background changes on disabled buttons. */
  :global(button:disabled),
  :global(button[aria-disabled='true']) {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }

  /* ─── Global focus-visible ring ───
     Keyboard users tabbing through chrome buttons (TitleBar, StatusBar,
     CommandPalette, SeriesEpisodeList, etc.) need a visible focus indicator.
     `:focus-visible` only triggers on keyboard focus, so mouse clicks don't
     get an outline (matching native browser behavior). `border-radius: inherit`
     keeps the ring shaped like the button. Components can opt out with
     `outline: none` on focus-visible if they own a richer indicator. */
  :global(button:focus-visible),
  :global([role='button']:focus-visible),
  :global([role='tab']:focus-visible),
  :global([role='option']:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: inherit;
  }

  /* ─── Scrollbar styling ─── */
  :global(::-webkit-scrollbar) {
    width: 6px;
  }
  :global(::-webkit-scrollbar-track) {
    background: transparent;
  }
  :global(::-webkit-scrollbar-thumb) {
    background: var(--text-muted);
    border-radius: 3px;
  }
  :global(::-webkit-scrollbar-thumb:hover) {
    background: var(--text-secondary);
  }
</style>
