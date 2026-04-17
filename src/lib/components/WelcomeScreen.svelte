<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import SeriesTitleDialog from './SeriesTitleDialog.svelte';

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
</script>

<div class="welcome">
  <div class="welcome-card">
    <img class="logo" src="/app-icon.png" alt="Scriptty" />
    <h1 class="title">Scriptty</h1>
    <p class="subtitle">Start a new project or open an existing one.</p>

    <div class="choice-row">
      <button class="choice" onclick={handleNewFilm}>
        <span class="choice-title">New Film</span>
        <span class="choice-desc">A single screenplay.</span>
      </button>
      <button class="choice" onclick={handleNewSeries}>
        <span class="choice-title">New Series</span>
        <span class="choice-desc">Multiple episodes in one project.</span>
      </button>
    </div>

    <button class="open-existing" onclick={onOpen}>Open Existing…</button>

    {#if recent.length > 0}
      <div class="recent">
        <div class="recent-label">Recent</div>
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

<style>
  .welcome {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface-base);
    z-index: 100;
    font-family: system-ui, -apple-system, sans-serif;
  }

  .welcome-card {
    width: 560px;
    max-width: 90vw;
    padding: 40px 36px 32px;
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 14px;
    box-shadow: 0 16px 48px var(--shadow-heavy);
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .logo {
    width: 72px;
    height: 72px;
    border-radius: 14px;
    margin-bottom: 12px;
  }

  .title {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.01em;
  }

  .subtitle {
    margin: 4px 0 24px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .choice-row {
    display: flex;
    gap: 12px;
    width: 100%;
    margin-bottom: 10px;
  }

  .choice {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 16px 18px;
    background: var(--surface-base);
    border: 1px solid var(--border-medium);
    border-radius: 10px;
    cursor: pointer;
    color: var(--text-primary);
    font-family: inherit;
    text-align: left;
    transition: border-color 120ms ease, background 120ms ease;
  }

  .choice:hover {
    background: var(--surface-hover);
    border-color: var(--accent);
  }

  .choice-title {
    font-size: 14px;
    font-weight: 600;
  }

  .choice-desc {
    font-size: 12px;
    color: var(--text-muted);
  }

  .open-existing {
    margin-top: 6px;
    height: 32px;
    padding: 0 16px;
    background: transparent;
    border: 1px solid var(--border-medium);
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 12.5px;
    font-family: inherit;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .open-existing:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .recent {
    width: 100%;
    margin-top: 28px;
    border-top: 1px solid var(--border-medium);
    padding-top: 18px;
    text-align: left;
  }

  .recent-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin-bottom: 8px;
  }

  .recent ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .recent-item {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 6px 10px;
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    color: var(--text-primary);
    font-family: inherit;
    text-align: left;
  }

  .recent-item:hover {
    background: var(--surface-hover);
  }

  .recent-name {
    font-size: 12.5px;
  }

  .recent-path {
    font-size: 10.5px;
    color: var(--text-muted);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
