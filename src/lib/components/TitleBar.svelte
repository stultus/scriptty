<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { documentStore } from '$lib/stores/documentStore.svelte';

  // Derived display title — shows document title or "Untitled"
  let displayTitle = $derived(
    documentStore.document?.meta.title || 'Untitled'
  );

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
</script>

<div class="title-bar">
  <div class="title-section">
    <span class="title">{displayTitle}</span>
    {#if documentStore.isDirty}
      <span class="dirty-indicator" title="Unsaved changes"></span>
    {/if}
  </div>
  <div class="actions">
    <button onclick={handleNew}>New</button>
    <button onclick={handleOpen}>Open</button>
    <button onclick={handleSave}>Save</button>
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
</style>
