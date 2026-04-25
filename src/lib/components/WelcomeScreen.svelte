<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import SeriesTitleDialog from './SeriesTitleDialog.svelte';
  import PasteScriptDialog from './PasteScriptDialog.svelte';

  let {
    onOpen,
  } = $props<{
    /** Parent callback for "Open Existing" so the page can run its
     *  confirm-dirty / load sequence alongside the rest of the file flows. */
    onOpen: () => Promise<void> | void;
  }>();

  // Recent files are stored client-side in localStorage. We keep this
  // welcome-local instead of in the documentStore so it stays a pure UI
  // concern and the store doesn't grow a persistent-state dependency.
  let recent = $state<{ path: string; name: string }[]>(loadRecent());
  let showSeriesDialog = $state(false);
  let showPasteDialog = $state(false);

  function loadRecent() {
    try {
      const raw = localStorage.getItem('scriptty-recent-files');
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed.filter((p): p is { path: string; name: string } =>
        p && typeof p.path === 'string' && typeof p.name === 'string'
      ).slice(0, 6);
    } catch {
      return [];
    }
  }

  async function handleNewFilm() {
    if (!(await documentStore.confirmIfDirty())) return;
    await documentStore.newDocument();
  }

  function handleNewSeries() {
    showSeriesDialog = true;
  }

  async function handleCreateSeries(title: string) {
    showSeriesDialog = false;
    if (!(await documentStore.confirmIfDirty())) return;
    await documentStore.newSeries(title.trim() || 'Untitled Series');
  }

  async function handleOpenPath(path: string) {
    if (!(await documentStore.confirmIfDirty())) return;
    await documentStore.openDocument(path);
  }

  function handlePasteScript() {
    showPasteDialog = true;
  }

  async function handlePasteConfirm(content: unknown) {
    showPasteDialog = false;
    if (!(await documentStore.confirmIfDirty())) return;
    await documentStore.newDocumentFromContent(content);
  }
</script>

<div class="welcome">
  <div class="welcome-card">
    <!-- Editorial masthead — eyebrow with flanking hairlines, big
         tracked title, italic Manjari subtitle, asterism divider.
         Same vocabulary as SceneCardsView's hero and the title-page
         preview, so the writer's first impression and their work
         surface speak the same design system. -->
    <img class="logo" src="/app-icon.png" alt="" aria-hidden="true" />

    <div class="masthead-eyebrow" aria-hidden="true">
      <span class="eyebrow-rule"></span>
      <span class="eyebrow-text">A Screenplay Editor</span>
      <span class="eyebrow-rule"></span>
    </div>

    <h1 class="title">SCRIPTTY</h1>
    <p class="subtitle">for Malayalam &amp; English screenwriters</p>

    <div class="masthead-divider" aria-hidden="true"></div>

    <div class="choice-row">
      <!-- svelte-ignore a11y_autofocus -->
      <button class="choice primary" onclick={handleNewFilm} autofocus>
        <span class="choice-eyebrow">Begin</span>
        <span class="choice-title">New Film</span>
        <span class="choice-desc">A single screenplay</span>
      </button>
      <button class="choice" onclick={handleNewSeries}>
        <span class="choice-eyebrow">Begin</span>
        <span class="choice-title">New Series</span>
        <span class="choice-desc">Multiple episodes in one project</span>
      </button>
    </div>

    <div class="secondary-row">
      <button class="pill-cta" onclick={onOpen}>
        <svg class="pill-icon" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M3 7 V19 A2 2 0 0 0 5 21 H19 A2 2 0 0 0 21 19 V9 A2 2 0 0 0 19 7 H12 L10 5 H5 A2 2 0 0 0 3 7 Z"/>
        </svg>
        <span>Open Existing</span>
      </button>
      <button class="pill-cta" onclick={handlePasteScript}>
        <svg class="pill-icon" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <rect x="6" y="4" width="12" height="16" rx="1"/>
          <path d="M9 4 V3 A1 1 0 0 1 10 2 H14 A1 1 0 0 1 15 3 V4"/>
          <path d="M9 11 H15 M9 15 H13"/>
        </svg>
        <span>Paste Script</span>
      </button>
    </div>

    {#if recent.length > 0}
      <div class="recent">
        <div class="recent-eyebrow" aria-hidden="true">
          <span class="eyebrow-rule"></span>
          <span class="eyebrow-text">Recent</span>
          <span class="eyebrow-rule"></span>
        </div>
        <ul>
          {#each recent as item (item.path)}
            <li>
              <button class="recent-item" onclick={() => handleOpenPath(item.path)} title={item.path}>
                <span class="recent-name">{item.name}</span>
                <span class="recent-path">{item.path}</span>
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>
</div>

<SeriesTitleDialog bind:open={showSeriesDialog} onConfirm={handleCreateSeries} />
<PasteScriptDialog bind:open={showPasteDialog} onConfirm={handlePasteConfirm} />

<style>
  .welcome {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface-base);
    background-image: var(--page-grain);
    background-size: 240px 240px;
    z-index: 100;
    font-family: var(--ui-font);
  }

  .welcome-card {
    position: relative;
    width: 580px;
    max-width: 92vw;
    padding: 44px 56px 36px;
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 12px;
    box-shadow: 0 16px 48px var(--shadow-heavy),
                0 2px 8px var(--shadow-soft);
    display: flex;
    flex-direction: column;
    /* Classical magazine treatment: display masthead and feature
       spread sit on the page-centerline (frontispiece feel); reading
       content (the recent index) breaks left below the divider. */
    align-items: center;
    text-align: center;
  }

  .logo {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    margin-bottom: 22px;
    opacity: 0.92;
  }

  /* ─── Editorial masthead ──────────────────────────────────────────── */
  /* Frontispiece-style centered masthead: tracked-caps eyebrow
     flanked by hairline rules on both sides, big confident title,
     italic Manjari subtitle. The flanking rules give the eyebrow
     visual weight on a centered axis without it disappearing into
     the surrounding negative space. */
  .masthead-eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 14px;
  }

  .eyebrow-rule {
    display: inline-block;
    width: 28px;
    height: 1px;
    background: var(--border-medium);
  }

  .eyebrow-text {
    font-family: var(--ui-font);
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  /* SCRIPTTY in Courier Prime — the screenplay's own typeface. Sets
     the wordmark in the same vocabulary as a printed cover sheet,
     so the welcome reads as a title-page extract from the form of
     work the app produces. Sized to dominate the masthead block
     without crowding the eyebrow above or subtitle below. */
  .title {
    margin: 0;
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 44px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    line-height: 1;
  }

  .subtitle {
    margin: 12px 0 0;
    font-family: 'Manjari', var(--ui-font);
    font-size: 14px;
    font-style: italic;
    color: var(--text-secondary);
    line-height: 1.3;
    letter-spacing: 0.005em;
  }

  /* Horizontal section break — classical magazine divider:
     a hairline rule running edge-to-edge, broken in the middle by
     a centered middle-dot. Same vocabulary the SceneCardsView hero
     uses below its masthead. */
  .masthead-divider {
    position: relative;
    width: 100%;
    height: 1px;
    margin: 26px 0 28px;
    background: linear-gradient(
      to right,
      transparent 0,
      var(--border-medium) 8%,
      var(--border-medium) 46%,
      transparent 47.5%,
      transparent 52.5%,
      var(--border-medium) 54%,
      var(--border-medium) 92%,
      transparent 100%);
  }

  .masthead-divider::before {
    content: '·';
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 14px;
    height: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 14px;
    line-height: 1;
    background: var(--surface-float);
  }

  /* ─── Choice cards ─────────────────────────────────────────────────── */
  /* Two-card feature spread, centered as a balanced pair on the
     page-centerline. Text inside each card stays left-aligned for
     readability — a classical magazine layout combination
     (centered display + flush-left body). */
  .choice-row {
    display: flex;
    gap: 12px;
    width: 100%;
    margin-bottom: 16px;
    text-align: left;
  }

  /* Each choice is a typeset card: tracked-caps eyebrow, bold title,
     italic descriptor. Reads like a section header from a book. */
  .choice {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 16px 18px;
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    cursor: pointer;
    color: var(--text-primary);
    font-family: inherit;
    text-align: left;
    transition: border-color 160ms ease, background 160ms ease,
                box-shadow 200ms ease;
  }

  .choice:hover {
    border-color: var(--border-medium);
    background: var(--surface-hover);
    box-shadow: 0 4px 14px var(--shadow-soft);
  }

  .choice.primary {
    background: var(--accent-muted);
    border-color: transparent;
  }

  .choice.primary:hover {
    background: var(--accent-muted);
    filter: brightness(1.05);
    box-shadow: 0 4px 18px var(--shadow-soft);
  }

  .choice-eyebrow {
    font-family: var(--ui-font);
    font-size: 8.5px;
    font-weight: 700;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .choice.primary .choice-eyebrow {
    color: var(--accent);
    opacity: 0.7;
  }

  .choice-title {
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-primary);
  }

  .choice.primary .choice-title {
    color: var(--accent);
  }

  .choice-desc {
    font-family: var(--ui-font);
    font-size: 11.5px;
    font-style: italic;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  /* ─── Secondary CTAs — outlined typeset pills ─────────────────────── */
  /* Same pill vocabulary as the SceneCardsView hero toolbar so the
     two surfaces speak the same affordance. Hairline border at rest,
     accent border + accent-muted fill on hover, leading SVG glyph
     for visual weight. */
  .secondary-row {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    margin-top: 4px;
  }

  .pill-cta {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 14px 0 12px;
    background: transparent;
    border: 1px solid var(--border-medium);
    border-radius: 16px;
    color: var(--text-secondary);
    font-family: var(--ui-font);
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    cursor: pointer;
    transition: color 120ms ease,
                border-color 120ms ease,
                background 120ms ease,
                transform 120ms ease;
  }

  .pill-cta:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-muted);
  }

  .pill-cta:active {
    transform: translateY(0.5px);
  }

  .pill-icon {
    flex-shrink: 0;
    color: currentColor;
    opacity: 0.75;
    transition: opacity 120ms ease;
  }

  .pill-cta:hover .pill-icon {
    opacity: 1;
  }

  /* ─── Recent index ─────────────────────────────────────────────────── */
  /* Reading content. Eyebrow stays centered above the list with
     flanking rules (matches the masthead vocabulary), but the list
     itself reads left-aligned like a typeset table-of-contents. */
  .recent {
    width: 100%;
    margin-top: 28px;
    padding-top: 22px;
    border-top: 1px solid var(--border-subtle);
    text-align: left;
  }

  .recent-eyebrow {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    margin-bottom: 14px;
    text-align: center;
  }

  .recent-eyebrow .eyebrow-rule {
    flex: 1;
    width: auto;
    max-width: 60px;
  }

  .recent ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  /* Each recent file gets a Courier numeric prefix (counter) so the
     list reads like a typeset index rather than a row of buttons. */
  .recent ul {
    counter-reset: recent-counter;
  }

  .recent ul li + li {
    border-top: 1px dashed var(--border-subtle);
  }

  .recent-item {
    width: 100%;
    display: grid;
    grid-template-columns: 32px 1fr;
    align-items: baseline;
    gap: 12px;
    padding: 10px 8px;
    background: transparent;
    border: none;
    border-radius: 5px;
    cursor: pointer;
    color: var(--text-primary);
    font-family: inherit;
    text-align: left;
    transition: background 120ms ease;
  }

  .recent-item::before {
    counter-increment: recent-counter;
    content: counter(recent-counter, decimal-leading-zero);
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 11px;
    font-weight: 700;
    color: var(--text-muted);
    letter-spacing: 0.04em;
    text-align: right;
    transition: color 120ms ease;
  }

  .recent-item:hover {
    background: var(--surface-hover);
  }

  .recent-item:hover::before {
    color: var(--accent);
  }

  .recent-name {
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 12.5px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-primary);
  }

  .recent-path {
    grid-column: 2;
    margin-top: 2px;
    font-family: var(--ui-font);
    font-size: 10.5px;
    font-style: italic;
    color: var(--text-secondary);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.85;
  }
</style>
