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
    font-family: system-ui, -apple-system, sans-serif;
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

    /* Text — off-white hierarchy */
    --text-primary: #e8e6e1;
    --text-secondary: #9e9a94;
    --text-muted: #5e5a55;
    --text-on-page: #1a1a1a;

    /* Accent — teal, consistent with app icon */
    --accent: #2d9b8a;
    --accent-hover: #35b5a2;
    --accent-muted: rgba(45, 155, 138, 0.15);

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
    --find-match-current: rgba(255, 152, 0, 0.55);
  }

  /* ─── Light mode ─── */
  :global([data-theme="light"]) {
    --surface-base: #f0ede8;
    --surface-elevated: #e8e4de;
    --surface-float: #faf8f5;
    --surface-hover: #dedad4;
    --surface-active: #d2cdc7;

    --page-bg: #ffffff;
    --page-shadow: rgba(0, 0, 0, 0.12);

    --text-primary: #1a1916;
    --text-secondary: #5c5852;
    --text-muted: #9c9891;
    --text-on-page: #1a1a1a;

    --accent: #1e8070;
    --accent-hover: #237a6a;
    --accent-muted: rgba(30, 128, 112, 0.1);

    --dirty: #c47f28;
    --error: #a83c30;
    --success: #2e7d52;

    --border-subtle: rgba(0, 0, 0, 0.08);
    --border-medium: rgba(0, 0, 0, 0.14);

    --shadow-soft: rgba(0, 0, 0, 0.08);
    --shadow-medium: rgba(0, 0, 0, 0.15);
    --shadow-heavy: rgba(0, 0, 0, 0.25);
    --backdrop: rgba(0, 0, 0, 0.4);

    --text-on-accent: #ffffff;

    --find-match: rgba(255, 200, 50, 0.4);
    --find-match-current: rgba(255, 140, 0, 0.5);
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
