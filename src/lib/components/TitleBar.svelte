<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';

  let {
    onToggleSidebar,
    activeView = 'writing',
    onViewChange,
    onShowExport,
    onShowMetadata,
  } = $props<{
    onToggleSidebar?: () => void;
    activeView?: 'writing' | 'cards' | 'story';
    onViewChange?: (view: 'writing' | 'cards' | 'story') => void;
    onShowExport?: () => void;
    /** Open the title-page / cover-sheet metadata editor (#165). */
    onShowMetadata?: () => void;
  }>();

  /** "Untitled" state — a brand-new doc with no title yet. The metadata
   *  button gets a small accent dot to nudge the writer to fill in the
   *  cover sheet. (#165) */
  let metadataIncomplete = $derived(
    !(documentStore.activeMeta?.title?.trim()),
  );

  // Derived display title — shows document title, or filename, or "Untitled"
  let displayTitle = $derived.by(() => {
    // Series projects show the series title, not the per-episode title —
    // the episode label renders separately below.
    if (documentStore.isSeries) {
      return documentStore.document?.series?.title || 'Untitled Series';
    }
    const title = documentStore.document?.meta.title;
    if (title) return title;
    const path = documentStore.currentPath;
    if (path) {
      const filename = path.split('/').pop() ?? path.split('\\').pop() ?? path;
      return filename.replace(/\.screenplay$/, '');
    }
    return 'Untitled';
  });

  // ─── Episode popover (#173) ───
  // The episode badge is a button — opens a popover listing every
  // episode with the active one highlighted. Clicking an episode
  // switches active; "+ New episode" appends and switches.
  let episodePopoverOpen = $state(false);
  let episodePopoverTrigger = $state<HTMLButtonElement | null>(null);
  let episodePopoverEl = $state<HTMLDivElement | null>(null);

  $effect(() => {
    if (!episodePopoverOpen) return;
    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as Node | null;
      if (!target) return;
      if (episodePopoverEl?.contains(target)) return;
      if (episodePopoverTrigger?.contains(target)) return;
      episodePopoverOpen = false;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') episodePopoverOpen = false;
    };
    document.addEventListener('pointerdown', onPointerDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown);
      document.removeEventListener('keydown', onKey);
    };
  });

  function pickEpisode(index: number) {
    documentStore.setActiveEpisode(index);
    episodePopoverOpen = false;
  }

  async function addEpisodeFromPopover() {
    await documentStore.addEpisode('');
    episodePopoverOpen = false;
  }

  // Two-phase flag: `visible` drives CSS opacity so Svelte can transition
  // in (true) and out (false); we keep the text mounted for the fade-out
  // window, then blank it.
  let statusMessage = $state('');
  let statusVisible = $state(false);
  let statusTimeout: ReturnType<typeof setTimeout> | null = null;
  let statusClearTimeout: ReturnType<typeof setTimeout> | null = null;

  // `showStatus` is called from save + any future action that wants user feedback.
  // Duration bumped to 4.5s per UX feedback.
  function showStatus(message: string) {
    statusMessage = message;
    statusVisible = true;
    if (statusTimeout) clearTimeout(statusTimeout);
    if (statusClearTimeout) clearTimeout(statusClearTimeout);
    statusTimeout = setTimeout(() => {
      statusVisible = false;
      // Clear the text after the fade-out finishes so layout settles.
      statusClearTimeout = setTimeout(() => { statusMessage = ''; }, 260);
    }, 4500);
  }

  // Show a persistent "Saved" tick on the Save button for a short window
  // after a successful save, until the doc goes dirty again.
  let recentlySaved = $state(false);
  let savedResetTimeout: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    // Reset the tick the moment the document becomes dirty again.
    if (documentStore.isDirty && recentlySaved) {
      recentlySaved = false;
      if (savedResetTimeout) { clearTimeout(savedResetTimeout); savedResetTimeout = null; }
    }
  });

  async function handleSave() {
    const wasDirty = documentStore.isDirty;
    await documentStore.saveWithDialog();
    if (!documentStore.isDirty && wasDirty) {
      showStatus('Document saved');
      recentlySaved = true;
      if (savedResetTimeout) clearTimeout(savedResetTimeout);
      savedResetTimeout = setTimeout(() => { recentlySaved = false; }, 6000);
    }
  }

</script>

<div class="title-bar">
  <div class="btn-group left">
    <!-- App wordmark — "Scriptty" set in bold tracked-caps Courier
         to match the screenplay-cover wordmark used everywhere else
         (welcome screen, about modal, PDF cover). The Malayalam ഋ
         glyph that lived here previously was a one-off mark that
         didn't speak the same vocabulary as the rest of the app. -->
    <span class="app-wordmark" aria-hidden="true" title="Scriptty">Scriptty</span>
    <button
      class="btn-icon"
      onclick={onToggleSidebar}
      title="Toggle sidebar (⌃⌘B)"
      aria-label="Toggle sidebar"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><line x1="9" y1="3" x2="9" y2="21"></line></svg>
    </button>
  </div>

  <div class="title-zone">
    <span class="title">{displayTitle}</span>
    {#if documentStore.isSeries && documentStore.activeEpisode}
      <button
        class="episode-label"
        type="button"
        onclick={() => { episodePopoverOpen = !episodePopoverOpen; }}
        title="Switch episode"
        aria-haspopup="listbox"
        aria-expanded={episodePopoverOpen}
        bind:this={episodePopoverTrigger}
      >
        <span class="ep-badge-mini">{String(documentStore.activeEpisode.number).padStart(2, '0')}</span>
        <span class="ep-label-text">{documentStore.activeEpisode.title.trim() || 'Untitled'}</span>
        <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M6 9 L12 15 L18 9"/>
        </svg>
      </button>
    {/if}
    {#if documentStore.isDirty}
      <span class="dirty-dot" title="Unsaved changes"></span>
    {/if}
    {#if statusMessage}
      <span class="status-message" class:visible={statusVisible} role="status" aria-live="polite">
        <svg class="status-tick" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
        {statusMessage}
      </span>
    {/if}

    <!-- Episode popover — anchored to the badge above. List of episodes
         with active one highlighted; pick switches active, "+ New
         episode" appends and switches. Same data as SeriesEpisodeList
         in the sidebar (#173). -->
    {#if episodePopoverOpen && documentStore.isSeries && documentStore.document?.series}
      <div
        class="ep-popover"
        role="listbox"
        aria-label="Switch episode"
        bind:this={episodePopoverEl}
      >
        {#each documentStore.document.series.episodes as ep, idx (ep.id)}
          <button
            type="button"
            class="ep-pop-row"
            class:active={idx === documentStore.activeEpisodeIndex}
            role="option"
            aria-selected={idx === documentStore.activeEpisodeIndex}
            onclick={() => pickEpisode(idx)}
          >
            <span class="ep-pop-badge">{String(ep.number).padStart(2, '0')}</span>
            <span class="ep-pop-title">{ep.title.trim() || 'Untitled'}</span>
            {#if idx === documentStore.activeEpisodeIndex}
              <svg class="ep-pop-check" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
            {/if}
          </button>
        {/each}
        <div class="ep-pop-sep" aria-hidden="true"></div>
        <button class="ep-pop-add" type="button" onclick={addEpisodeFromPopover}>
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <line x1="12" y1="5" x2="12" y2="19"/>
            <line x1="5" y1="12" x2="19" y2="12"/>
          </svg>
          New episode
        </button>
      </div>
    {/if}
  </div>

  <div class="view-switcher">
    <button
      class="view-tab"
      class:active={activeView === 'writing'}
      onclick={() => onViewChange?.('writing')}
      title="Writing view"
    >
      <svg class="tab-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M16 3 L20 7 L8 19 L3 20 L4 15 Z"/>
      </svg>
      <span class="tab-label">Writing</span>
    </button>
    <button
      class="view-tab"
      class:active={activeView === 'cards'}
      onclick={() => onViewChange?.('cards')}
      title="Scene Cards (⌘⇧K)"
    >
      <svg class="tab-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <rect x="3" y="5" width="8" height="14" rx="1.5"/>
        <rect x="13" y="5" width="8" height="14" rx="1.5"/>
      </svg>
      <span class="tab-label">Cards</span>
    </button>
    <button
      class="view-tab"
      class:active={activeView === 'story'}
      onclick={() => onViewChange?.('story')}
      title="Story view (⌘⇧L)"
    >
      <svg class="tab-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <line x1="5" y1="7"  x2="19" y2="7"/>
        <line x1="5" y1="12" x2="19" y2="12"/>
        <line x1="5" y1="17" x2="14" y2="17"/>
      </svg>
      <span class="tab-label">Story</span>
    </button>
  </div>

  <div class="btn-group right">
    <button
      class="btn-icon meta-btn"
      class:incomplete={metadataIncomplete}
      onclick={() => onShowMetadata?.()}
      title="Document properties (cover sheet, draft info)"
      aria-label="Document properties"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <rect x="5" y="3" width="14" height="18" rx="1.5"/>
        <line x1="9" y1="8"  x2="15" y2="8"/>
        <line x1="8" y1="12" x2="16" y2="12"/>
        <line x1="9" y1="15" x2="15" y2="15"/>
      </svg>
      {#if metadataIncomplete}
        <span class="incomplete-dot" aria-hidden="true"></span>
      {/if}
    </button>
    <button class="btn-ghost" onclick={() => onShowExport?.()} title="Export document">Export</button>
    <button
      class="btn-primary"
      class:saved={recentlySaved && !documentStore.isDirty}
      onclick={handleSave}
      title={recentlySaved && !documentStore.isDirty ? 'Document saved' : 'Save (⌘S)'}
    >
      {#if recentlySaved && !documentStore.isDirty}
        <svg class="save-tick" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
      {/if}
      Save
    </button>
  </div>
</div>


<style>
  /* Bumped from 32px to 38px so the 26px buttons get proper vertical
     padding and the title text breathes (#170). */
  .title-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 38px;
    padding: 0 12px;
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border-subtle);
    font-family: system-ui, -apple-system, sans-serif;
    user-select: none;
    flex-shrink: 0;
  }

  /* App wordmark — "Scriptty" in bold tracked-caps Courier matches
     the screenplay-cover wordmark used by the welcome surface,
     about modal, and printed cover. The Courier face ties the
     title-bar identity to the rest of the editorial vocabulary. */
  .app-wordmark {
    display: inline-flex;
    align-items: center;
    height: 22px;
    margin: 0 4px 0 2px;
    padding: 0 4px;
    color: var(--accent);
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    line-height: 1;
    user-select: none;
    -webkit-app-region: drag;
  }

  .btn-group {
    display: flex;
    align-items: center;
    gap: 4px;
    -webkit-app-region: no-drag;
  }

  .title-zone {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    -webkit-app-region: drag;
    flex: 1;
    justify-content: center;
    min-width: 0;
  }

  /* Title set in Courier Prime so it reads as printed-on-paper rather
     than generic UI text — picks up Scriptty's typographic identity
     (#170). Mixed-script titles fall through to Manjari/Noto for
     non-Latin glyphs via the editor-font-ml chain. */
  .title {
    color: var(--text-primary);
    font-family: var(--editor-font-en), 'Manjari', 'Noto Sans Malayalam', ui-monospace, monospace;
    font-size: 12.5px;
    font-weight: 700;
    letter-spacing: 0.01em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Episode badge button — clickable popover trigger (#173). */
  .episode-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px 2px 4px;
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    font-family: var(--ui-font);
    font-size: 11px;
    color: var(--text-secondary);
    background: var(--surface-base);
    letter-spacing: 0.02em;
    white-space: nowrap;
    cursor: pointer;
    -webkit-app-region: no-drag;
    max-width: min(320px, 40vw);
    overflow: hidden;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
    flex-shrink: 1;
  }

  .episode-label:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
    border-color: var(--border-medium);
  }

  .episode-label[aria-expanded='true'] {
    background: var(--accent-muted);
    color: var(--accent);
    border-color: var(--accent);
  }

  .ep-badge-mini {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 16px;
    padding: 0 5px;
    border-radius: 4px;
    background: var(--accent-muted);
    color: var(--accent);
    font-family: var(--editor-font-en), var(--ui-font);
    font-size: 9.5px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.04em;
  }

  .episode-label[aria-expanded='true'] .ep-badge-mini {
    background: var(--accent);
    color: var(--text-on-accent);
  }

  .ep-label-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  /* ─── Episode popover ─── */
  .ep-popover {
    position: absolute;
    top: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
    min-width: 260px;
    max-width: 360px;
    max-height: 50vh;
    overflow-y: auto;
    padding: 4px;
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 8px;
    box-shadow:
      0 12px 32px var(--shadow-heavy),
      0 2px 8px var(--shadow-soft);
    z-index: 50;
    -webkit-app-region: no-drag;
    animation: ep-pop-in 120ms ease-out;
  }

  @keyframes ep-pop-in {
    from { opacity: 0; transform: translate(-50%, -4px); }
    to   { opacity: 1; transform: translate(-50%, 0); }
  }

  .ep-pop-row {
    width: 100%;
    display: grid;
    grid-template-columns: 28px 1fr 14px;
    align-items: center;
    gap: 8px;
    padding: 7px 8px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--ui-font);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    transition: background 100ms ease, color 100ms ease;
  }

  .ep-pop-row:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .ep-pop-row.active {
    background: var(--accent-muted);
    color: var(--accent);
  }

  .ep-pop-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 26px;
    height: 18px;
    padding: 0 6px;
    border-radius: 4px;
    background: var(--surface-elevated);
    color: var(--text-muted);
    font-family: var(--editor-font-en), var(--ui-font);
    font-size: 10px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.04em;
  }

  .ep-pop-row.active .ep-pop-badge {
    background: var(--accent);
    color: var(--text-on-accent);
  }

  .ep-pop-title {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .ep-pop-check {
    color: var(--accent);
    flex-shrink: 0;
  }

  .ep-pop-sep {
    height: 1px;
    background: var(--border-subtle);
    margin: 4px 6px;
  }

  .ep-pop-add {
    width: 100%;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 7px 8px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--accent);
    font-family: var(--ui-font);
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }

  .ep-pop-add:hover {
    background: var(--accent-muted);
  }

  .dirty-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--dirty);
    flex-shrink: 0;
  }

  .status-message {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--accent-warm);
    letter-spacing: 0.02em;
    opacity: 0;
    transform: translateY(-2px);
    transition: opacity 220ms ease, transform 220ms ease;
    pointer-events: none;
  }

  .status-message.visible {
    opacity: 1;
    transform: translateY(0);
  }

  .status-tick {
    color: var(--accent-warm);
    flex-shrink: 0;
  }

  .save-tick {
    margin-right: 4px;
    color: currentColor;
    vertical-align: -1px;
  }

  .btn-primary.saved {
    display: inline-flex;
    align-items: center;
  }

  /* ─── View switcher tabs ─── */
  /* Demoted vs the title-zone primary actions — smaller font, lighter
     padding, less prominent at-rest background — so Save/Export retain
     the right-side primary-action role (#107). */
  .view-switcher {
    display: flex;
    align-items: center;
    gap: 2px;
    background: transparent;
    border-radius: 5px;
    padding: 2px;
    border: 1px solid var(--border-subtle);
    -webkit-app-region: no-drag;
  }

  .view-tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px;
    border-radius: 4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 10.5px;
    font-weight: 500;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 100ms, color 100ms;
  }

  .view-tab:hover {
    color: var(--text-secondary);
  }

  .tab-icon {
    color: var(--text-muted);
    flex-shrink: 0;
    transition: color 100ms ease;
  }

  .view-tab:hover .tab-icon {
    color: var(--text-secondary);
  }

  .view-tab.active .tab-icon {
    color: var(--accent);
  }

  .view-tab.active {
    background: var(--surface-base);
    color: var(--text-primary);
    box-shadow: 0 1px 2px var(--shadow-soft);
  }

  /* ─── Export and Save are visual peers (#164) ───
     Both are completion actions — Save persists, Export ships. They get
     the same dimensions, weight, font, and corner radius. Save keeps
     the filled-accent treatment (it's the more frequent action); Export
     gets an accent-tinted soft fill — equal-but-distinguishable.
     Supersedes #107's "make Export visible" patch with a fuller
     statement of equality. */
  .btn-ghost {
    height: 26px;
    padding: 0 14px;
    border-radius: 5px;
    border: 1px solid var(--accent);
    background: var(--accent-muted);
    color: var(--accent);
    font-size: 12px;
    font-weight: 600;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }
  .btn-ghost:hover {
    background: var(--accent);
    color: var(--text-on-accent);
    border-color: var(--accent);
  }
  .btn-ghost:active {
    filter: brightness(0.96);
  }

  /* ─── Save — filled accent peer to Export ─── */
  .btn-primary {
    height: 26px;
    padding: 0 14px;
    border-radius: 5px;
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--text-on-accent);
    font-size: 12px;
    font-weight: 600;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .btn-primary:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  /* ─── Icon button (sidebar toggle, metadata, etc.) ─── */
  .btn-icon {
    position: relative;
    height: 26px;
    padding: 0 6px;
    border-radius: 5px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 11px;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  /* Metadata button — same shape as other icon buttons but a touch
     wider so the page glyph reads as a discrete action, not chrome (#165). */
  .meta-btn {
    width: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .meta-btn:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  /* Incomplete-metadata nudge — small accent-warm dot in the corner of
     the meta button when the cover sheet has no title yet. Mirrors the
     dirty-indicator pattern (amber dot) but drives a different signal. */
  .incomplete-dot {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-warm, var(--dirty));
    box-shadow: 0 0 0 2px var(--surface-elevated);
  }

  .btn-icon:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

</style>
