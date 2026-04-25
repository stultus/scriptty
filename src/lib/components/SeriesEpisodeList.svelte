<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import SceneNavigator from './SceneNavigator.svelte';
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  let expanded = $state<Record<string, boolean>>({});
  let editingIndex = $state<number>(-1);
  let editingText = $state('');
  let editingSeries = $state(false);
  let seriesDraft = $state('');

  // ─── Drag-to-reorder (#144) ────────────────────────────────────────
  // HTML5 DnD is unreliable in Tauri's WebKit, so the same custom
  // mousedown/mousemove/mouseup pattern from SceneNavigator and
  // EpisodeCardsView. Tracks the source index and the current target
  // by hit-testing each .episode-row's bounding rect.
  let dragFromIndex = $state<number | null>(null);
  let dropTargetIndex = $state<number | null>(null);
  let listEl = $state<HTMLUListElement | null>(null);

  function rowIndexAtY(clientY: number): number | null {
    if (!listEl) return null;
    const rows = listEl.querySelectorAll<HTMLElement>('.episode-row');
    for (let i = 0; i < rows.length; i++) {
      const r = rows[i].getBoundingClientRect();
      if (clientY >= r.top && clientY <= r.bottom) return i;
    }
    return null;
  }

  function startDrag(event: MouseEvent, index: number) {
    event.preventDefault();
    dragFromIndex = index;
    dropTargetIndex = index;

    const onMove = (e: MouseEvent) => {
      const i = rowIndexAtY(e.clientY);
      if (i !== null) dropTargetIndex = i;
    };
    const onUp = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
      if (
        dragFromIndex !== null &&
        dropTargetIndex !== null &&
        dragFromIndex !== dropTargetIndex
      ) {
        documentStore.reorderEpisode(dragFromIndex, dropTargetIndex);
      }
      dragFromIndex = null;
      dropTargetIndex = null;
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

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
        aria-label="Rename series"
      >{seriesTitle || 'Untitled Series'}</button>
    {/if}
    <button class="icon-btn" onclick={addEpisode} title="Add episode" aria-label="Add episode">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <line x1="12" y1="5" x2="12" y2="19"></line>
        <line x1="5" y1="12" x2="19" y2="12"></line>
      </svg>
    </button>
  </header>

  <ul class="episode-list" bind:this={listEl}>
    {#each episodes as ep, index (ep.id)}
      {@const isActive = index === documentStore.activeEpisodeIndex}
      {@const isOpen = expanded[ep.id] ?? false}
      <li
        class="episode-li"
        class:active={isActive}
        class:dragging={dragFromIndex === index}
        class:drop-above={dropTargetIndex === index && dragFromIndex !== null && dragFromIndex > index}
        class:drop-below={dropTargetIndex === index && dragFromIndex !== null && dragFromIndex < index}
      >
        <div class="episode-row">
          <span
            class="drag-handle"
            onmousedown={(e: MouseEvent) => startDrag(e, index)}
            role="button"
            tabindex="-1"
            aria-label={`Drag to reorder Episode ${ep.number}`}
            title="Drag to reorder"
          >
            <svg width="8" height="14" viewBox="0 0 8 14" fill="currentColor" aria-hidden="true">
              <circle cx="2" cy="3" r="1"/>
              <circle cx="6" cy="3" r="1"/>
              <circle cx="2" cy="7" r="1"/>
              <circle cx="6" cy="7" r="1"/>
              <circle cx="2" cy="11" r="1"/>
              <circle cx="6" cy="11" r="1"/>
            </svg>
          </span>
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
              <span class="episode-number">{String(ep.number).padStart(2, '0')}</span>
              <span class="episode-title">{ep.title || 'Untitled'}</span>
            </button>
            <div class="episode-actions">
              <button class="tiny-btn" onclick={() => beginRename(index, ep.title)} title="Rename" aria-label={`Rename Episode ${ep.number}`}>
                <svg width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M12 2 L14 4 L5 13 L2 14 L3 11 Z" />
                </svg>
              </button>
              <button
                class="tiny-btn danger"
                onclick={() => removeEpisode(index)}
                disabled={episodes.length <= 1}
                title={episodes.length <= 1 ? 'Cannot delete the last episode' : 'Delete episode'}
                aria-label={`Delete Episode ${ep.number}`}
              >
                <svg width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M3 4 L13 4 M6 4 V2 H10 V4 M5 4 L6 14 H10 L11 4" />
                </svg>
              </button>
            </div>
          {/if}
        </div>

        {#if isOpen && isActive}
          <div
            class="episode-scenes"
            transition:slide={{ duration: 180, easing: cubicOut }}
          >
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
    position: relative;
    margin-bottom: 2px;
  }

  .episode-li.dragging {
    opacity: 0.45;
  }

  /* Drop indicators — accent line above or below the target row.
     Mirrors the SceneNavigator pattern (#144) so the two reorder
     gestures look the same. */
  .episode-li.drop-above::before {
    content: '';
    position: absolute;
    top: -1px;
    left: 22px;
    right: 6px;
    height: 2px;
    background: var(--accent);
    border-radius: 1px;
    z-index: 1;
    pointer-events: none;
  }

  .episode-li.drop-below::after {
    content: '';
    position: absolute;
    bottom: -1px;
    left: 22px;
    right: 6px;
    height: 2px;
    background: var(--accent);
    border-radius: 1px;
    z-index: 1;
    pointer-events: none;
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

  /* 6-dot drag handle on the left edge — matches SceneNavigator
     (#144). Subtler at rest, full opacity on hover. */
  .drag-handle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    flex-shrink: 0;
    color: var(--text-muted);
    cursor: grab;
    opacity: 0;
    transition: opacity 120ms ease;
    user-select: none;
  }

  .drag-handle:active {
    cursor: grabbing;
  }

  .episode-row:hover .drag-handle,
  .episode-li.active .drag-handle {
    opacity: 0.6;
  }

  .drag-handle:hover {
    opacity: 1 !important;
    color: var(--text-secondary);
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

  /* Zero-padded Courier chip — same identifier system EpisodeCardsView
     uses (#145). Active episode's badge picks up the accent fill. */
  .episode-number {
    flex-shrink: 0;
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

  .episode-li.active .episode-number {
    background: var(--accent);
    color: var(--text-on-accent);
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

  /* Actions stay visible at low opacity at rest so keyboard users can
     see them without hovering — full opacity on hover or any descendant
     focus (#113). The active row also surfaces them at full opacity
     since it's the row the writer is acting on. */
  .episode-actions {
    display: flex;
    align-items: center;
    gap: 1px;
    opacity: 0.55;
    transition: opacity 120ms ease;
  }

  .episode-li:hover .episode-actions,
  .episode-li.active .episode-actions,
  .episode-actions:focus-within {
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
    transition: background 120ms ease, color 120ms ease;
  }

  .tiny-btn:hover:not(:disabled) {
    background: var(--surface-float);
    color: var(--text-primary);
  }

  .tiny-btn.danger:hover:not(:disabled) {
    background: var(--error-muted, var(--accent-muted));
    color: var(--error);
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
