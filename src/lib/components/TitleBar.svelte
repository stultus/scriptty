<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import MetadataModal from './MetadataModal.svelte';
  import ExportModal from './ExportModal.svelte';

  let showMetadata = $state(false);
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

  async function handleNew() {
    if (!(await documentStore.confirmIfDirty())) return;
    await documentStore.newDocument();
  }

  async function handleSave() {
    await documentStore.saveWithDialog();
  }

  async function handleOpen() {
    if (!(await documentStore.confirmIfDirty())) return;
    const path = await open({
      multiple: false,
      filters: [{ name: 'Screenplay', extensions: ['screenplay'] }]
    });
    if (typeof path === 'string') {
      await documentStore.openDocument(path);
    }
  }

</script>

<div class="title-bar">
  <div class="btn-group left">
    <button class="btn-ghost" onclick={handleNew}>New</button>
    <button class="btn-ghost" onclick={() => { showMetadata = true; }}>Meta</button>
    <button class="btn-ghost" onclick={handleOpen}>Open</button>
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
    <button class="btn-ghost" onclick={() => { showExport = true; }}>Export</button>
    <button class="btn-primary" onclick={handleSave}>Save</button>
  </div>
</div>

<MetadataModal bind:open={showMetadata} />
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
</style>
