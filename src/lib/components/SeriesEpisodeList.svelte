<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import SceneNavigator from './SceneNavigator.svelte';

  let expanded = $state<Record<string, boolean>>({});
  let editingIndex = $state<number>(-1);
  let editingText = $state('');
  let editingSeries = $state(false);
  let seriesDraft = $state('');

  // Episode list is a reactive snapshot of the series' episodes array.
  let episodes = $derived(documentStore.document?.series?.episodes ?? []);
  let seriesTitle = $derived(documentStore.document?.series?.title ?? '');

  // The *active* episode's folder auto-expands so its scenes are visible.
  $effect(() => {
    const active = documentStore.activeEpisode;
    if (active && expanded[active.id] === undefined) {
      expanded[active.id] = true;
    }
  });

  function toggleExpanded(id: string) {
    expanded[id] = !expanded[id];
  }

  function activate(index: number) {
    documentStore.setActiveEpisode(index);
    const ep = documentStore.document?.series?.episodes?.[index];
    if (ep) expanded[ep.id] = true;
  }

  async function addEpisode() {
    await documentStore.addEpisode('');
  }

  function beginRename(index: number, currentTitle: string) {
    editingIndex = index;
    editingText = currentTitle;
  }

  function commitRename() {
    if (editingIndex >= 0) {
      documentStore.renameEpisode(editingIndex, editingText.trim());
    }
    editingIndex = -1;
    editingText = '';
  }

  function cancelRename() {
    editingIndex = -1;
    editingText = '';
  }

  function handleRenameKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      commitRename();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      cancelRename();
    }
  }

  function removeEpisode(index: number) {
    if (episodes.length <= 1) return;
    const ep = episodes[index];
    const label = ep.title.trim() ? `"${ep.title}"` : `Episode ${ep.number}`;
    if (!confirm(`Delete ${label}? This cannot be undone.`)) return;
    documentStore.removeEpisode(index);
  }

  function moveUp(index: number) {
    if (index > 0) documentStore.reorderEpisode(index, index - 1);
  }

  function moveDown(index: number) {
    if (index < episodes.length - 1) documentStore.reorderEpisode(index, index + 1);
  }

  function beginSeriesRename() {
    seriesDraft = seriesTitle;
    editingSeries = true;
  }

  function commitSeriesRename() {
    const trimmed = seriesDraft.trim() || 'Untitled Series';
    documentStore.renameSeries(trimmed);
    editingSeries = false;
  }

  function cancelSeriesRename() {
    editingSeries = false;
    seriesDraft = '';
  }

  function handleSeriesKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      commitSeriesRename();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      cancelSeriesRename();
    }
  }
</script>

<div class="series-panel">
  <header class="series-header">
    {#if editingSeries}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="series-title-input"
        bind:value={seriesDraft}
        onblur={commitSeriesRename}
        onkeydown={handleSeriesKeydown}
        autofocus
      />
    {:else}
      <button
        class="series-title"
        onclick={beginSeriesRename}
        title="Rename series"
      >{seriesTitle || 'Untitled Series'}</button>
    {/if}
    <button class="icon-btn" onclick={addEpisode} title="Add episode" aria-label="Add episode">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <line x1="12" y1="5" x2="12" y2="19"></line>
        <line x1="5" y1="12" x2="19" y2="12"></line>
      </svg>
    </button>
  </header>

  <ul class="episode-list">
    {#each episodes as ep, index (ep.id)}
      {@const isActive = index === documentStore.activeEpisodeIndex}
      {@const isOpen = expanded[ep.id] ?? false}
      <li class="episode-li" class:active={isActive}>
        <div class="episode-row">
          <button
            class="disclosure"
            onclick={() => toggleExpanded(ep.id)}
            aria-label={isOpen ? 'Collapse episode' : 'Expand episode'}
          >
            <svg
              width="10"
              height="10"
              viewBox="0 0 10 10"
              fill="currentColor"
              style:transform={isOpen ? 'rotate(90deg)' : 'rotate(0deg)'}
            >
              <path d="M3 2 L7 5 L3 8 Z" />
            </svg>
          </button>
          {#if editingIndex === index}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              class="episode-title-input"
              bind:value={editingText}
              onblur={commitRename}
              onkeydown={handleRenameKeydown}
              autofocus
            />
          {:else}
            <button
              class="episode-label"
              onclick={() => activate(index)}
              ondblclick={() => beginRename(index, ep.title)}
              title={ep.title ? `Episode ${ep.number} — ${ep.title}` : `Episode ${ep.number}`}
            >
              <span class="episode-number">Ep {ep.number}</span>
              <span class="episode-title">{ep.title || 'Untitled'}</span>
            </button>
            <div class="episode-actions">
              <button class="tiny-btn" onclick={() => moveUp(index)} disabled={index === 0} title="Move up" aria-label="Move up">
                <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor"><path d="M5 2 L9 7 L1 7 Z" /></svg>
              </button>
              <button class="tiny-btn" onclick={() => moveDown(index)} disabled={index === episodes.length - 1} title="Move down" aria-label="Move down">
                <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor"><path d="M5 8 L1 3 L9 3 Z" /></svg>
              </button>
              <button class="tiny-btn" onclick={() => beginRename(index, ep.title)} title="Rename" aria-label="Rename">
                <svg width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M12 2 L14 4 L5 13 L2 14 L3 11 Z" />
                </svg>
              </button>
              <button
                class="tiny-btn danger"
                onclick={() => removeEpisode(index)}
                disabled={episodes.length <= 1}
                title={episodes.length <= 1 ? 'Cannot delete the last episode' : 'Delete episode'}
                aria-label="Delete"
              >
                <svg width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M3 4 L13 4 M6 4 V2 H10 V4 M5 4 L6 14 H10 L11 4" />
                </svg>
              </button>
            </div>
          {/if}
        </div>

        {#if isOpen && isActive}
          <div class="episode-scenes">
            <SceneNavigator />
          </div>
        {/if}
      </li>
    {/each}
  </ul>
</div>

<style>
  .series-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    font-family: system-ui, -apple-system, sans-serif;
  }

  .series-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 10px 8px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .series-title {
    flex: 1;
    min-width: 0;
    padding: 4px 6px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: inherit;
    text-align: left;
    cursor: text;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .series-title:hover {
    background: var(--surface-hover);
  }

  .series-title-input {
    flex: 1;
    min-width: 0;
    padding: 4px 6px;
    height: 26px;
    background: var(--surface-base);
    border: 1px solid var(--accent);
    border-radius: 4px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: inherit;
    outline: none;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .icon-btn:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .episode-list {
    list-style: none;
    margin: 0;
    padding: 6px 6px 10px;
    overflow-y: auto;
    flex: 1;
  }

  .episode-li {
    margin-bottom: 2px;
  }

  .episode-row {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px 4px;
    border-radius: 6px;
    transition: background 120ms ease;
  }

  .episode-li.active .episode-row {
    background: var(--accent-muted);
  }

  .episode-li:not(.active) .episode-row:hover {
    background: var(--surface-hover);
  }

  .disclosure {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 22px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  .disclosure svg {
    transition: transform 120ms ease;
  }

  .episode-label {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 6px;
    height: 26px;
    padding: 0 4px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    overflow: hidden;
  }

  .episode-number {
    color: var(--text-muted);
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .episode-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .episode-title-input {
    flex: 1;
    min-width: 0;
    padding: 0 6px;
    height: 26px;
    background: var(--surface-base);
    border: 1px solid var(--accent);
    border-radius: 4px;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 12px;
    outline: none;
  }

  .episode-actions {
    display: flex;
    align-items: center;
    gap: 1px;
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .episode-li:hover .episode-actions,
  .episode-li.active .episode-actions {
    opacity: 1;
  }

  .tiny-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    background: transparent;
    border: none;
    border-radius: 3px;
    color: var(--text-muted);
    cursor: pointer;
  }

  .tiny-btn:hover:not(:disabled) {
    background: var(--surface-float);
    color: var(--text-primary);
  }

  .tiny-btn.danger:hover:not(:disabled) {
    color: #c44;
  }

  .tiny-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .episode-scenes {
    margin-left: 16px;
    padding-top: 4px;
    border-left: 1px solid var(--border-subtle);
  }

  .episode-scenes :global(.navigator-content) {
    padding: 4px 6px 6px 8px;
  }
</style>
