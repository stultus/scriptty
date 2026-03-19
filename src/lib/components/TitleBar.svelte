<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { themeStore } from '$lib/stores/themeStore.svelte';
  import ExportModal from './ExportModal.svelte';

  let { onToggleSidebar } = $props<{ onToggleSidebar?: () => void }>();
  
  let showExport = $state(false);

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
    <button class="btn-icon" onclick={onToggleSidebar} title="Toggle Sidebar">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><line x1="9" y1="3" x2="9" y2="21"></line></svg>
    </button>
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

  <div class="btn-group right">
    <!-- Theme toggle -->
    <button class="btn-icon" onclick={() => themeStore.toggle()} title={themeStore.isDark ? 'Switch to light mode' : 'Switch to dark mode'}>
      {themeStore.isDark ? 'Light' : 'Dark'}
    </button>

    <span class="separator"></span>

    <!-- Font selector — segmented control -->
    <div class="segmented">
      <button
        class="segmented-item"
        class:active={documentStore.currentFont === 'noto-sans-malayalam'}
        onclick={() => documentStore.setFont('noto-sans-malayalam')}
      >Noto</button>
      <button
        class="segmented-item"
        class:active={documentStore.currentFont === 'manjari'}
        onclick={() => documentStore.setFont('manjari')}
      >Manjari</button>
    </div>

    <span class="separator"></span>

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
    color: #fff;
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
  .btn-icon:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  /* ─── Separator ─── */
  .separator {
    width: 1px;
    height: 16px;
    background: var(--border-subtle);
    margin: 0 4px;
  }

  /* ─── Segmented control (font selector) ─── */
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
    border: none;
    font-size: 11px;
    font-family: system-ui, -apple-system, sans-serif;
    color: var(--text-muted);
    background: transparent;
    cursor: pointer;
    transition: background 100ms, color 100ms;
  }
  .segmented-item:hover {
    color: var(--text-secondary);
  }
  .segmented-item.active {
    background: var(--surface-elevated);
    color: var(--text-primary);
  }
</style>
