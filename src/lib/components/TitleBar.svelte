<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { writeFile } from '@tauri-apps/plugin-fs';
  import { documentStore } from '$lib/stores/documentStore.svelte';

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
    await documentStore.newDocument();
  }

  async function handleSave() {
    console.log('[TitleBar] Save clicked');
    await documentStore.saveWithDialog();
  }

  async function handleOpen() {
    const path = await open({
      multiple: false,
      filters: [{ name: 'Screenplay', extensions: ['screenplay'] }]
    });
    // open() returns string | string[] | null depending on `multiple`
    if (typeof path === 'string') {
      await documentStore.openDocument(path);
    }
  }

  /** Shared PDF export helper — calls the given Rust command and saves the result */
  async function exportPdf(command: string, defaultName: string) {
    if (!documentStore.document) {
      showStatus('No document to export');
      return;
    }
    try {
      showStatus('Generating PDF...');
      const bytes = await invoke<number[]>(command, { document: documentStore.document });
      const path = await save({
        defaultPath: defaultName,
        filters: [{ name: 'PDF', extensions: ['pdf'] }]
      });
      if (!path) {
        showStatus(''); // User cancelled
        return;
      }
      await writeFile(path, new Uint8Array(bytes));
      showStatus('PDF exported');
    } catch (e) {
      console.error('[TitleBar] Export failed:', e);
      showStatus('Export failed');
    }
  }

  async function handleExportHollywood() {
    await exportPdf('export_pdf', 'screenplay.pdf');
  }

  async function handleExportIndian() {
    await exportPdf('export_pdf_indian', 'screenplay_indian.pdf');
  }
</script>

<div class="title-bar">
  <div class="title-section">
    <span class="title">{displayTitle}</span>
    {#if documentStore.isDirty}
      <span class="dirty-indicator" title="Unsaved changes"></span>
    {/if}
  </div>
  <div class="font-selector">
    <button
      class="font-btn"
      class:active={documentStore.currentFont === 'noto-sans-malayalam'}
      onclick={() => documentStore.setFont('noto-sans-malayalam')}
    >Noto</button>
    <button
      class="font-btn"
      class:active={documentStore.currentFont === 'manjari'}
      onclick={() => documentStore.setFont('manjari')}
    >Manjari</button>
  </div>
  <div class="actions">
    <button onclick={handleNew}>New</button>
    <button onclick={handleOpen}>Open</button>
    <button onclick={handleSave}>Save</button>
    <button onclick={handleExportHollywood}>Export (Hollywood)</button>
    <button onclick={handleExportIndian}>Export (Indian)</button>
    {#if statusMessage}
      <span class="status-message">{statusMessage}</span>
    {/if}
  </div>
</div>

<style>
  .title-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    background: #222;
    border-bottom: 1px solid #333;
    font-family: system-ui, sans-serif;
    user-select: none;
  }

  .title-section {
    display: flex;
    align-items: center;
    gap: 8px;
    -webkit-app-region: drag;
  }

  .title {
    color: #ccc;
    font-size: 13px;
    font-weight: 500;
  }

  .dirty-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #f5a623;
  }

  .actions {
    display: flex;
    gap: 6px;
    -webkit-app-region: no-drag;
  }

  .actions button {
    padding: 4px 12px;
    font-size: 12px;
    color: #ccc;
    background: #333;
    border: 1px solid #444;
    border-radius: 4px;
    cursor: pointer;
  }

  .actions button:hover {
    background: #444;
    color: #fff;
  }

  .font-selector {
    display: flex;
    gap: 2px;
  }

  .font-btn {
    padding: 3px 8px;
    font-size: 11px;
    color: #888;
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    cursor: pointer;
    font-family: system-ui, sans-serif;
  }

  .font-btn:hover {
    color: #ccc;
    background: rgba(255, 255, 255, 0.08);
  }

  .font-btn.active {
    color: #4fc3f7;
    border-color: #4fc3f7;
  }

  .status-message {
    font-size: 11px;
    color: #888;
    font-family: system-ui, sans-serif;
  }
</style>
