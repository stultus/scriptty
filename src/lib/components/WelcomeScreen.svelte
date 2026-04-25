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

    <div class="asterism" aria-hidden="true">
      <span class="asterism-glyph">·</span>
      <span class="asterism-glyph">·</span>
      <span class="asterism-glyph">·</span>
    </div>

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
    padding: 36px 44px 32px;
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 12px;
    box-shadow: 0 16px 48px var(--shadow-heavy),
                0 2px 8px var(--shadow-soft);
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .logo {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    margin-bottom: 18px;
    opacity: 0.92;
  }

  /* ─── Editorial masthead ──────────────────────────────────────────── */
  /* Eyebrow with flanking hairlines — same vocabulary as the
     SceneCardsView hero, the title-page preview, and the PDF cover. */
  .masthead-eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 14px;
  }

  .eyebrow-rule {
    display: inline-block;
    width: 32px;
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
     work the app produces. */
  .title {
    margin: 0;
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 36px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: 0.06em;
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

  /* Asterism — three middle-dots, classical print divider. Each
     glyph gets its own span so we can space them confidently
     without relying on letter-spacing collapsing in some fonts. */
  .asterism {
    display: inline-flex;
    gap: 16px;
    margin: 26px 0 28px;
    color: var(--text-muted);
    font-size: 16px;
    line-height: 1;
    user-select: none;
  }

  .asterism-glyph {
    display: inline-block;
    line-height: 1;
  }

  /* ─── Choice cards ─────────────────────────────────────────────────── */
  .choice-row {
    display: flex;
    gap: 10px;
    width: 100%;
    margin-bottom: 14px;
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
  .recent {
    width: 100%;
    margin-top: 28px;
    text-align: left;
  }

  /* Eyebrow row mirrors the masthead — flanking hairlines around the
     "Recent" label so the section break reads as typeset, not as a
     lazy header. */
  .recent-eyebrow {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 10px;
    justify-content: center;
  }

  .recent-eyebrow .eyebrow-rule {
    flex: 1;
    width: auto;
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
