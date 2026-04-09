<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';
  import { themeStore } from '$lib/stores/themeStore.svelte';
  import { toggleMark } from 'prosemirror-commands';
  import { screenplaySchema } from '$lib/editor/schema';
  import ExportModal from './ExportModal.svelte';

  let {
    onToggleSidebar,
    activeView = 'writing',
    onViewChange,
  } = $props<{
    onToggleSidebar?: () => void;
    activeView?: 'writing' | 'cards' | 'story';
    onViewChange?: (view: 'writing' | 'cards' | 'story') => void;
  }>();

  let showExport = $state(false);

  // Track which marks are active at the current cursor position.
  // Updated whenever the editor selection changes via editorStore.markState.
  let isBoldActive = $derived(editorStore.markState.bold);
  let isItalicActive = $derived(editorStore.markState.italic);
  let isUnderlineActive = $derived(editorStore.markState.underline);

  function applyMark(markName: 'bold' | 'italic' | 'underline') {
    const view = editorStore.view;
    if (!view) return;
    toggleMark(screenplaySchema.marks[markName])(view.state, view.dispatch);
    view.focus();
  }

  // Derived display title — shows document title or "Untitled"
  let displayTitle = $derived(
    documentStore.document?.meta.title || 'Untitled'
  );

  let statusMessage = $state('');
  let statusTimeout: ReturnType<typeof setTimeout> | null = null;

  function showStatus(message: string) {
    statusMessage = message;
    if (statusTimeout) clearTimeout(statusTimeout);
    statusTimeout = setTimeout(() => { statusMessage = ''; }, 3000);
  }

  async function handleSave() {
    await documentStore.saveWithDialog();
  }

</script>

<div class="title-bar">
  <div class="btn-group left">
    <button
      class="btn-icon"
      class:disabled={activeView !== 'writing'}
      onclick={onToggleSidebar}
      disabled={activeView !== 'writing'}
      title="Toggle Sidebar (Writing view only)"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><line x1="9" y1="3" x2="9" y2="21"></line></svg>
    </button>
    {#if activeView === 'writing'}
      <span class="separator"></span>
      <div class="format-group">
        <button
          class="btn-format"
          class:active={isBoldActive}
          onclick={() => applyMark('bold')}
          title="Bold (Cmd+B)"
        ><span class="fmt-bold">B</span></button>
        <button
          class="btn-format"
          class:active={isItalicActive}
          onclick={() => applyMark('italic')}
          title="Italic (Cmd+I)"
        ><span class="fmt-italic">I</span></button>
        <button
          class="btn-format"
          class:active={isUnderlineActive}
          onclick={() => applyMark('underline')}
          title="Underline (Cmd+U)"
        ><span class="fmt-underline">U</span></button>
      </div>
    {/if}
  </div>

  <div class="title-zone">
    <span class="title">{displayTitle}</span>
    {#if documentStore.isDirty}
      <span class="dirty-dot" title="Unsaved changes"></span>
    {/if}
    {#if statusMessage}
      <span class="status-message">{statusMessage}</span>
    {/if}
  </div>

  <div class="view-switcher">
    <button
      class="view-tab"
      class:active={activeView === 'writing'}
      onclick={() => onViewChange?.('writing')}
    >Writing</button>
    <button
      class="view-tab"
      class:active={activeView === 'cards'}
      onclick={() => onViewChange?.('cards')}
    >Cards</button>
    <button
      class="view-tab"
      class:active={activeView === 'story'}
      onclick={() => onViewChange?.('story')}
    >Story</button>
  </div>

  <div class="btn-group right">
    <button class="btn-ghost" onclick={() => { showExport = true; }}>Export</button>
    <button class="btn-primary" onclick={handleSave}>Save</button>
  </div>
</div>

<ExportModal bind:open={showExport} />

<style>
  .title-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
    padding: 0 12px;
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
    font-size: 11px;
    color: var(--text-muted);
    letter-spacing: 0.02em;
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
    height: 28px;
    padding: 0 10px;
    border-radius: 6px;
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
    height: 28px;
    padding: 0 12px;
    border-radius: 6px;
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
    height: 28px;
    padding: 0 8px;
    border-radius: 6px;
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

  .btn-icon:disabled {
    opacity: 0.3;
    cursor: default;
  }

  /* ─── Separator between sidebar toggle and format buttons ─── */
  .separator {
    width: 1px;
    height: 16px;
    background: var(--border-subtle);
    margin: 0 4px;
  }

  /* ─── Format button group (B, I, U) — tighter spacing ─── */
  .format-group {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  /* ─── Format buttons ─── */
  .btn-format {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 13px;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-format:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .btn-format:active {
    background: var(--surface-active);
  }

  .btn-format.active {
    background: var(--accent-muted);
    color: var(--accent);
  }

  .btn-format.active:hover {
    background: var(--accent-muted);
  }

  /* Format label styling — using CSS instead of HTML tags to avoid
     browser default style interference */
  .fmt-bold {
    font-weight: 700;
  }

  .fmt-italic {
    font-style: italic;
  }

  .fmt-underline {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
</style>
