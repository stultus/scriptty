<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';

  let {
    onToggleSidebar,
    activeView = 'writing',
    onViewChange,
    onShowExport,
  } = $props<{
    onToggleSidebar?: () => void;
    activeView?: 'writing' | 'cards' | 'story';
    onViewChange?: (view: 'writing' | 'cards' | 'story') => void;
    onShowExport?: () => void;
  }>();

  // Derived display title — shows document title, or filename, or "Untitled"
  let displayTitle = $derived.by(() => {
    const title = documentStore.document?.meta.title;
    if (title) return title;
    const path = documentStore.currentPath;
    if (path) {
      const filename = path.split('/').pop() ?? path.split('\\').pop() ?? path;
      return filename.replace(/\.screenplay$/, '');
    }
    return 'Untitled';
  });

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
    <button
      class="btn-icon"
      onclick={onToggleSidebar}
      disabled={activeView !== 'writing'}
      title="Toggle Sidebar (⌘B) — Writing view only"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><line x1="9" y1="3" x2="9" y2="21"></line></svg>
    </button>
  </div>

  <div class="title-zone">
    <span class="title">{displayTitle}</span>
    {#if documentStore.isDirty}
      <span class="dirty-dot" title="Unsaved changes"></span>
    {/if}
    {#if statusMessage}
      <span class="status-message" class:visible={statusVisible} role="status" aria-live="polite">
        <svg class="status-tick" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
        {statusMessage}
      </span>
    {/if}
  </div>

  <div class="view-switcher">
    <button
      class="view-tab"
      class:active={activeView === 'writing'}
      onclick={() => onViewChange?.('writing')}
      title="Writing view"
    >
      <span class="tab-label">Writing</span>
    </button>
    <button
      class="view-tab"
      class:active={activeView === 'cards'}
      onclick={() => onViewChange?.('cards')}
      title="Scene Cards (⌘⇧K)"
    >
      <span class="tab-label">Cards</span>
    </button>
    <button
      class="view-tab"
      class:active={activeView === 'story'}
      onclick={() => onViewChange?.('story')}
      title="Story view (⌘⇧L)"
    >
      <span class="tab-label">Story</span>
    </button>
  </div>

  <div class="btn-group right">
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
  .title-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 32px;
    padding: 0 10px;
    background: var(--surface-elevated);
    border-bottom: 1px solid var(--border-subtle);
    font-family: system-ui, -apple-system, sans-serif;
    user-select: none;
    flex-shrink: 0;
  }

  .btn-group {
    display: flex;
    align-items: center;
    gap: 4px;
    -webkit-app-region: no-drag;
  }

  .title-zone {
    display: flex;
    align-items: center;
    gap: 8px;
    -webkit-app-region: drag;
    flex: 1;
    justify-content: center;
    min-width: 0;
  }

  .title {
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
  .view-switcher {
    display: flex;
    align-items: center;
    gap: 2px;
    background: var(--surface-base);
    border-radius: 6px;
    padding: 2px;
    border: 1px solid var(--border-subtle);
    -webkit-app-region: no-drag;
  }

  .view-tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    border-radius: 4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 500;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 100ms, color 100ms;
  }

  .view-tab:hover {
    color: var(--text-secondary);
  }

  .view-tab.active {
    background: var(--surface-elevated);
    color: var(--text-primary);
    box-shadow: 0 1px 2px var(--shadow-soft);
  }

  /* ─── Ghost button ─── */
  .btn-ghost {
    height: 24px;
    padding: 0 10px;
    border-radius: 5px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    font-family: system-ui, -apple-system, sans-serif;
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

  /* ─── Primary button (Save) ─── */
  .btn-primary {
    height: 24px;
    padding: 0 12px;
    border-radius: 5px;
    border: none;
    background: var(--accent);
    color: var(--text-on-accent);
    font-size: 12px;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 120ms ease;
  }
  .btn-primary:hover {
    background: var(--accent-hover);
  }

  /* ─── Icon button (theme toggle) ─── */
  .btn-icon {
    height: 24px;
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

  .btn-icon:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

</style>
