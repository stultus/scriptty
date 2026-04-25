<script lang="ts">
  // Episode-level Card View for series projects (#134).
  //
  // Sister component to SceneCardsView's scene-level grid. Renders the
  // series' episodes as cards — each one shows number, title, a synopsis
  // teaser, and aggregate metrics (scenes / pages). Clicking a card hands
  // off to the parent (SceneCardsView), which sets the active episode and
  // drills down into the per-episode scene grid.
  //
  // Episode CRUD reuses the existing documentStore methods
  // (addEpisode / removeEpisode / renameEpisode / reorderEpisode), so this
  // view shares state with SeriesEpisodeList — a rename here updates the
  // sidebar there immediately.

  import { flip } from 'svelte/animate';
  import { cubicInOut } from 'svelte/easing';
  import { confirm } from '@tauri-apps/plugin-dialog';
  import { documentStore, type Episode, type EpisodeStatus } from '$lib/stores/documentStore.svelte';

  /** Ordered cycle for the status pill — clicking the pill walks
   *  Outline → Draft → Revision → Final → Outline … (#141). */
  const STATUS_CYCLE: EpisodeStatus[] = ['outline', 'draft', 'revision', 'final'];

  function statusLabel(s: EpisodeStatus): string {
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  function nextStatus(current: EpisodeStatus): EpisodeStatus {
    const i = STATUS_CYCLE.indexOf(current);
    return STATUS_CYCLE[(i + 1) % STATUS_CYCLE.length];
  }

  function cycleStatus(index: number, current: EpisodeStatus) {
    documentStore.setEpisodeStatus(index, nextStatus(current));
  }

  let { onOpenEpisode }: { onOpenEpisode: (index: number) => void } = $props();

  // ─── Compact mode (#153) ────────────────────────────────────────────
  // Toggleable density preference. Default (roomy) is the writer's
  // reading view; compact is the at-a-glance arc-planning view that
  // collapses each card to a single row so a 12-episode series fits
  // in one viewport. Persisted to localStorage so the writer's choice
  // sticks across sessions.
  const COMPACT_KEY = 'scriptty-episodes-compact';
  let compact = $state(false);
  if (typeof localStorage !== 'undefined') {
    compact = localStorage.getItem(COMPACT_KEY) === '1';
  }
  $effect(() => {
    if (typeof localStorage === 'undefined') return;
    localStorage.setItem(COMPACT_KEY, compact ? '1' : '0');
  });

  /** Live snapshot of the series episodes — reactive via documentStore.
   *  Empty array for non-series (the parent guards against rendering us
   *  in that case, but we double-check to be defensive). */
  let episodes = $derived<Episode[]>(documentStore.document?.series?.episodes ?? []);

  /** Aggregate scene count for an episode by counting `scene_heading`
   *  nodes in its content. Cheap enough to compute per render — episode
   *  count is small (single digits typically). */
  function sceneCountFor(ep: Episode): number {
    const content = ep.content as { content?: Array<{ type?: string }> } | null;
    if (!content?.content) return 0;
    let n = 0;
    for (const node of content.content) {
      if (node.type === 'scene_heading') n++;
    }
    return n;
  }

  /** First N scene headings as plain strings. Renders into the card's
   *  scene-peek list so the writer can scan what's inside each episode
   *  without drilling in (#152). */
  function sceneHeadingsFor(ep: Episode, limit: number): string[] {
    const content = ep.content as {
      content?: Array<{ type?: string; content?: Array<{ text?: string }> }>;
    } | null;
    if (!content?.content) return [];
    const out: string[] = [];
    for (const node of content.content) {
      if (node.type !== 'scene_heading') continue;
      const text = (node.content ?? []).map((c) => c.text ?? '').join('').trim();
      out.push(text || '(empty)');
      if (out.length >= limit) break;
    }
    return out;
  }

  /** Rough page estimate using the same 3000-char-per-page heuristic the
   *  scene cards use. Sums all text length across the episode. */
  function pageEstimateFor(ep: Episode): string {
    const content = ep.content as { content?: Array<{ content?: Array<{ text?: string }> }> } | null;
    if (!content?.content) return '0';
    let chars = 0;
    for (const node of content.content) {
      const inner = node.content ?? [];
      for (const c of inner) chars += (c.text ?? '').length;
    }
    const pages = Math.max(0.1, chars / 3000);
    return pages.toFixed(1);
  }

  // ─── Inline rename state ───────────────────────────────────────────
  let editingIndex = $state<number>(-1);
  let editingText = $state('');

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

  // ─── Inline synopsis (story.idea) editor ───────────────────────────
  function updateIdea(index: number, value: string) {
    const ep = episodes[index];
    if (!ep) return;
    if (!ep.story) {
      // Defensive — older series files may have an empty episode without
      // the story object initialized; create it so the writer's edit lands.
      ep.story = { idea: '', synopsis: '', treatment: '', narrative: '' };
    }
    ep.story.idea = value;
    documentStore.markDirty();
  }

  // ─── Drag-to-reorder ───────────────────────────────────────────────
  let dragFrom = $state<number | null>(null);
  let dropTarget = $state<number | null>(null);
  let gridEl = $state<HTMLDivElement | null>(null);

  function cardIndexAt(x: number, y: number): number | null {
    if (!gridEl) return null;
    const cards = gridEl.querySelectorAll<HTMLElement>('.ep-card');
    for (let i = 0; i < cards.length; i++) {
      const r = cards[i].getBoundingClientRect();
      if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) return i;
    }
    return null;
  }

  function startDrag(event: MouseEvent, index: number) {
    event.preventDefault();
    dragFrom = index;
    dropTarget = index;

    const onMove = (e: MouseEvent) => {
      const i = cardIndexAt(e.clientX, e.clientY);
      if (i !== null) dropTarget = i;
    };
    const onUp = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
      if (dragFrom !== null && dropTarget !== null && dragFrom !== dropTarget) {
        documentStore.reorderEpisode(dragFrom, dropTarget);
      }
      dragFrom = null;
      dropTarget = null;
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  // ─── CRUD ──────────────────────────────────────────────────────────
  async function addEpisode() {
    await documentStore.addEpisode('');
  }

  async function removeEpisode(index: number) {
    if (episodes.length <= 1) return;
    const ep = episodes[index];
    const label = ep.title.trim() ? `"${ep.title}"` : `Episode ${ep.number}`;
    const ok = await confirm(`Delete ${label}? This cannot be undone.`, {
      title: 'Delete episode',
      kind: 'warning',
    });
    if (ok) documentStore.removeEpisode(index);
  }

  function openEpisode(index: number) {
    onOpenEpisode(index);
  }
</script>

<div class="episodes-pane-inner">
<div class="episodes-toolbar">
  <label class="density-toggle" title="Compact view collapses each card to a single row — useful for arc-planning a long series">
    <input type="checkbox" bind:checked={compact} />
    <span>Compact</span>
  </label>
</div>

<div class="episodes-grid" class:compact bind:this={gridEl}>
  {#each episodes as ep, index (ep.id)}
    {@const sceneCount = sceneCountFor(ep)}
    {@const pages = pageEstimateFor(ep)}
    {@const idea = ep.story?.idea ?? ''}
    {@const peekHeadings = sceneHeadingsFor(ep, 3)}
    {@const hiddenCount = sceneCount - peekHeadings.length}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <article
      class="ep-card"
      class:active={index === documentStore.activeEpisodeIndex}
      class:dragging={dragFrom === index}
      class:drop-target={dropTarget === index && dragFrom !== null && dragFrom !== index}
      animate:flip={{ duration: 350, easing: cubicInOut }}
      onclick={(e) => {
        // Click anywhere on the card body drills into scenes (#154) —
        // editable controls (textarea, input, buttons) opt out via
        // closest() so the writer doesn't drill while editing the
        // logline or renaming the episode.
        const t = e.target as HTMLElement;
        if (t.closest('input, textarea, button, [role="button"]')) return;
        openEpisode(index);
      }}
    >
      <header class="ep-header">
        <span
          class="ep-number"
          onmousedown={(e: MouseEvent) => startDrag(e, index)}
          role="button"
          tabindex="-1"
          aria-label="Drag to reorder Episode {ep.number}"
          title="Drag to reorder"
        >{String(ep.number).padStart(2, '0')}</span>

        {#if editingIndex === index}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="ep-title-input"
            bind:value={editingText}
            onblur={commitRename}
            onkeydown={handleRenameKeydown}
            placeholder="Episode title"
            autofocus
          />
        {:else}
          <button
            class="ep-title"
            class:untitled={!ep.title.trim()}
            onclick={() => beginRename(index, ep.title)}
            title="Click to rename"
          >{ep.title.trim() || 'Untitled'}</button>
        {/if}

        <div class="ep-actions">
          <button
            class="ep-icon-btn"
            type="button"
            onclick={() => beginRename(index, ep.title)}
            aria-label="Rename Episode {ep.number}"
            title="Rename"
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M16 3 L20 7 L8 19 L3 20 L4 15 Z"/>
            </svg>
          </button>
          <button
            class="ep-icon-btn danger"
            type="button"
            disabled={episodes.length <= 1}
            onclick={() => removeEpisode(index)}
            aria-label="Delete Episode {ep.number}"
            title={episodes.length <= 1 ? 'Cannot delete the last episode' : 'Delete episode'}
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="3 6 5 6 21 6"/>
              <path d="M19 6 l-1 14 a2 2 0 0 1-2 2 H8 a2 2 0 0 1-2-2 L5 6"/>
              <path d="M10 11 v6"/>
              <path d="M14 11 v6"/>
              <path d="M9 6 V4 a1 1 0 0 1 1-1 h4 a1 1 0 0 1 1 1 v2"/>
            </svg>
          </button>
        </div>
      </header>

      <div class="ep-body">
        <label class="ep-field-label" for="ep-idea-{ep.id}">Logline</label>
        <textarea
          id="ep-idea-{ep.id}"
          class="ep-textarea"
          placeholder="A line or two about what this episode is about..."
          value={idea}
          oninput={(e) => updateIdea(index, (e.target as HTMLTextAreaElement).value)}
        ></textarea>

        <!-- Scene peek (#152) — first three scene headings so the writer
             can scan progress without drilling in. Empty episodes show a
             single muted line rather than collapsing the section. -->
        <div class="ep-peek">
          <span class="ep-field-label">Scenes</span>
          {#if peekHeadings.length === 0}
            <p class="ep-peek-empty">No scenes yet — drill in to start outlining.</p>
          {:else}
            <ol class="ep-peek-list">
              {#each peekHeadings as heading, i}
                <li class="ep-peek-row">
                  <span class="ep-peek-num">{String(i + 1).padStart(2, '0')}</span>
                  <span class="ep-peek-heading">{heading.toUpperCase()}</span>
                </li>
              {/each}
            </ol>
            {#if hiddenCount > 0}
              <p class="ep-peek-more">
                + {hiddenCount} more {hiddenCount === 1 ? 'scene' : 'scenes'}
              </p>
            {/if}
          {/if}
        </div>
      </div>

      <footer class="ep-footer">
        <div class="ep-stats">
          <button
            type="button"
            class="ep-status status-{ep.status ?? 'outline'}"
            onclick={(e) => { e.stopPropagation(); cycleStatus(index, ep.status ?? 'outline'); }}
            title="Click to cycle status"
            aria-label="Episode status: {statusLabel(ep.status ?? 'outline')}"
          >
            <span class="ep-status-dot" aria-hidden="true"></span>
            {statusLabel(ep.status ?? 'outline')}
          </button>
          <span class="ep-stat">
            <strong>{sceneCount}</strong>
            <span class="ep-stat-label">{sceneCount === 1 ? 'scene' : 'scenes'}</span>
          </span>
          <span class="ep-stat">
            <strong>~{pages}</strong>
            <span class="ep-stat-label">pages</span>
          </span>
        </div>
        <button class="ep-open" type="button" onclick={() => openEpisode(index)}>
          Open scenes
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 6 L15 12 L9 18"/>
          </svg>
        </button>
      </footer>
    </article>
  {/each}

  <!-- Add Episode card — sits at the end of the grid as a dotted-border
       placeholder so it reads as "create one more" rather than another
       data card. Click anywhere on it appends and (via documentStore)
       focuses the new episode. -->
  <button class="ep-add-card" type="button" onclick={addEpisode}>
    <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
      <line x1="12" y1="5" x2="12" y2="19"/>
      <line x1="5" y1="12" x2="19" y2="12"/>
    </svg>
    <span class="ep-add-label">New episode</span>
    <span class="ep-add-hint">Append to the series</span>
  </button>
</div>
</div>

<style>
  .episodes-pane-inner {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* Toolbar with Compact toggle (#153). Right-aligned so it doesn't
     compete with the SceneCardsView hero header above; the toggle is
     the only control here today. */
  .episodes-toolbar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    padding: 0 4px;
    font-family: var(--ui-font);
    font-size: 11.5px;
    color: var(--text-muted);
  }

  .density-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    color: var(--text-secondary);
    user-select: none;
  }

  .density-toggle input[type='checkbox'] {
    accent-color: var(--accent);
    cursor: pointer;
  }

  .episodes-grid {
    display: grid;
    /* Roomy default fits 4 cards on a 1500px window (was 3 with 320px
       min). Compact mode bumps it to a much wider column flow that
       prefers a single column when the rail's narrow but folds to two
       wide stripes once the workspace can fit it. (#153) */
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
    align-items: stretch;
  }

  .episodes-grid.compact {
    grid-template-columns: 1fr;
    gap: 6px;
  }

  @media (min-width: 1200px) {
    .episodes-grid.compact {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  .ep-card {
    display: flex;
    flex-direction: column;
    background: var(--surface-float);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    box-shadow: 0 1px 2px var(--shadow-soft);
    overflow: hidden;
    min-height: 180px;
    /* Cursor on the body signals "click to drill in" (#154); editable
       controls inside the card override back to default via specific
       selectors below so they read as "type here, not navigate." */
    cursor: pointer;
    transition: border-color var(--motion-fast, 100ms) ease,
                box-shadow var(--motion-fast, 100ms) ease,
                transform var(--motion-fast, 100ms) ease;
  }

  .ep-card:hover {
    border-color: var(--border-medium);
    box-shadow: 0 2px 8px var(--shadow-soft);
  }

  /* Restore the default cursor on regions that aren't navigation. */
  .ep-card :is(input, textarea, button, [role='button']) {
    cursor: auto;
  }
  .ep-card .ep-textarea,
  .ep-card .ep-title-input {
    cursor: text;
  }
  .ep-card :is(.ep-icon-btn, .ep-open) {
    cursor: pointer;
  }
  .ep-card .ep-number {
    cursor: grab;
  }

  .ep-card.dragging {
    opacity: 0.4;
  }

  .ep-card.drop-target {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-muted);
  }

  /* Active episode — the one currently being edited (matches
     activeEpisodeIndex). Picks up an inset accent left bar and an
     accent-tinted number badge, mirroring the active scene card and
     active sidebar row. (#151) */
  .ep-card.active {
    border-color: var(--accent);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .ep-card.active .ep-number {
    background: var(--accent);
    color: var(--text-on-accent);
  }

  /* ─── Header ─── */
  .ep-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--surface-base);
  }

  /* Episode number set in Courier Prime to match the rest of the
     screenplay typography — and zero-padded ("01", "02") so the badge
     reads as a chapter marker rather than a runtime integer. */
  .ep-number {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 30px;
    height: 26px;
    padding: 0 8px;
    border-radius: 5px;
    background: var(--accent-muted);
    color: var(--accent);
    font-family: var(--editor-font-en), var(--ui-font);
    font-size: 12.5px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.04em;
    cursor: grab;
    user-select: none;
  }

  .ep-number:active {
    cursor: grabbing;
  }

  .ep-title {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 4px 8px;
    margin-left: -4px;
    color: var(--text-primary);
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
    font-size: 13.5px;
    font-weight: 700;
    text-align: left;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    cursor: text;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ep-title:hover {
    background: var(--surface-hover);
  }

  .ep-title.untitled {
    color: var(--text-muted);
    font-style: italic;
    font-weight: 500;
    text-transform: none;
  }

  .ep-title-input {
    flex: 1;
    min-width: 0;
    padding: 4px 8px;
    margin-left: -4px;
    background: var(--surface-float);
    border: 1px solid var(--accent);
    border-radius: 4px;
    color: var(--text-primary);
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
    font-size: 13.5px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    outline: none;
  }

  .ep-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    opacity: 0.55;
    transition: opacity var(--motion-fast, 100ms) ease;
  }

  .ep-card:hover .ep-actions,
  .ep-actions:focus-within {
    opacity: 1;
  }

  .ep-icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .ep-icon-btn:hover:not(:disabled) {
    background: var(--surface-elevated);
    color: var(--text-primary);
  }

  .ep-icon-btn.danger:hover:not(:disabled) {
    background: var(--error-muted, var(--accent-muted));
    color: var(--error);
  }

  .ep-icon-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  /* ─── Body ─── */
  .ep-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 14px;
    min-height: 0;
  }

  .ep-field-label {
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .ep-textarea {
    flex: 1;
    width: 100%;
    min-height: 80px;
    padding: 8px 10px;
    background: var(--surface-base);
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--text-primary);
    font-family: var(--ui-font);
    font-size: 12.5px;
    line-height: 1.5;
    resize: vertical;
    box-sizing: border-box;
    transition: border-color var(--motion-fast, 100ms) ease,
                background var(--motion-fast, 100ms) ease;
  }

  .ep-textarea:focus {
    outline: none;
    border-color: var(--accent);
    background: var(--surface-float);
  }

  .ep-textarea::placeholder {
    color: var(--text-muted);
    font-style: italic;
  }

  /* ─── Scene peek (#152) ─── */
  /* Designed to read like a call-sheet: a slim ruled column of scene
     headings, each row showing a Courier number flush-right against a
     1px hairline gutter and the slugline laid out in the same monospace
     as the editor. The footer ("+ N more scenes") sits below the list,
     italic and de-weighted so it reads as meta rather than data. */
  .ep-peek {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 4px;
  }

  .ep-peek-empty {
    margin: 0;
    font-size: 11px;
    color: var(--text-muted);
    font-style: italic;
    line-height: 1.4;
  }

  .ep-peek-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    /* Hairline divider between rows to give the list typeset rhythm
       without visual noise. Each row also picks up a tiny hover lift. */
    border-top: 1px solid var(--border-subtle);
  }

  .ep-peek-row {
    display: grid;
    grid-template-columns: 26px 1px 1fr;
    align-items: center;
    gap: 10px;
    padding: 5px 4px 5px 0;
    border-bottom: 1px solid var(--border-subtle);
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
    font-size: 10.5px;
    color: var(--text-secondary);
    line-height: 1.35;
    transition: background var(--motion-fast, 100ms) ease;
  }

  /* Synthetic vertical hairline between number column and slugline —
     subtle architectural cue, costs nothing, makes the list feel
     deliberately typeset rather than generic. */
  .ep-peek-row::before {
    content: '';
    grid-column: 2;
    align-self: stretch;
    background: var(--border-subtle);
  }

  .ep-card:hover .ep-peek-row {
    background: linear-gradient(to right, transparent, var(--surface-hover) 30%, var(--surface-hover) 100%);
  }

  .ep-peek-num {
    grid-column: 1;
    color: var(--text-muted);
    font-size: 9.5px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.04em;
    text-align: right;
  }

  .ep-peek-heading {
    grid-column: 3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    letter-spacing: 0.02em;
  }

  .ep-peek-more {
    margin: 0;
    padding-left: 36px;
    font-family: var(--ui-font);
    font-size: 10.5px;
    font-style: italic;
    color: var(--text-muted);
    letter-spacing: 0.01em;
  }

  /* ─── Footer ─── */
  .ep-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    background: var(--surface-base);
    border-top: 1px solid var(--border-subtle);
  }

  .ep-stats {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 11px;
    color: var(--text-muted);
  }

  /* Status pill — small clickable lifecycle marker (#141). Click cycles
     Outline → Draft → Revision → Final → Outline. Color-coded:
     Outline neutral, Draft warm, Revision accent, Final accent-deep. */
  .ep-status {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 9px;
    border: 1px solid transparent;
    border-radius: 11px;
    font-family: var(--ui-font);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    cursor: pointer;
    transition: filter var(--motion-fast, 100ms) ease,
                transform var(--motion-fast, 100ms) ease;
  }

  .ep-status:hover {
    filter: brightness(1.05);
    transform: translateY(-0.5px);
  }

  .ep-status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    flex-shrink: 0;
  }

  .ep-status.status-outline {
    background: var(--surface-elevated);
    color: var(--text-muted);
    border-color: var(--border-subtle);
  }

  .ep-status.status-draft {
    background: rgba(214, 161, 74, 0.14);
    color: var(--accent-warm, #c47e1f);
    border-color: rgba(214, 161, 74, 0.28);
  }

  .ep-status.status-revision {
    background: var(--accent-muted);
    color: var(--accent);
    border-color: var(--accent-muted);
  }

  .ep-status.status-final {
    background: var(--accent);
    color: var(--text-on-accent);
    border-color: var(--accent);
  }

  .ep-stat {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
  }

  .ep-stat strong {
    color: var(--text-primary);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    font-size: 12.5px;
  }

  .ep-stat-label {
    font-size: 10.5px;
    letter-spacing: 0.04em;
  }

  .ep-open {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border-radius: 5px;
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--text-on-accent);
    font-family: var(--ui-font);
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                border-color var(--motion-fast, 100ms) ease;
  }

  .ep-open:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  /* ─── Compact mode (#153) ───────────────────────────────────────────
     Collapses each card to a single row: badge · title · logline
     snippet · scene count · open arrow. Hides the body (textarea +
     scene peek) and the per-card status pill keeps its place inline.
     A 12-episode series fits in one viewport. */
  .episodes-grid.compact .ep-card {
    min-height: 0;
    flex-direction: row;
    align-items: stretch;
  }

  .episodes-grid.compact .ep-header {
    flex-shrink: 0;
    width: auto;
    min-width: 220px;
    max-width: 38%;
    padding: 8px 12px;
    border-bottom: none;
    border-right: 1px solid var(--border-subtle);
    background: var(--surface-base);
  }

  .episodes-grid.compact .ep-body {
    padding: 8px 12px;
    gap: 4px;
    min-height: 0;
  }

  .episodes-grid.compact .ep-field-label,
  .episodes-grid.compact .ep-peek {
    display: none;
  }

  .episodes-grid.compact .ep-textarea {
    /* Replace the multi-line textarea with a single-line truncated
       preview; the writer can still drill in to edit the full logline.
       Read-only inline edit isn't worth the complexity for compact. */
    min-height: 0;
    max-height: 22px;
    padding: 2px 6px;
    border: 1px solid transparent;
    background: transparent;
    overflow: hidden;
    resize: none;
    font-style: italic;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .episodes-grid.compact .ep-textarea:focus {
    background: var(--surface-base);
    border-color: var(--border-subtle);
  }

  .episodes-grid.compact .ep-footer {
    flex-shrink: 0;
    background: transparent;
    border-top: none;
    border-left: 1px solid var(--border-subtle);
    padding: 8px 12px;
    gap: 10px;
  }

  /* In compact mode the actions appear inside the header; keep them
     visible at full opacity since the row is shorter and there's
     less space to fade them. */
  .episodes-grid.compact .ep-actions {
    opacity: 1;
  }

  /* ─── Add-episode placeholder ─── */
  .ep-add-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 180px;
    background: transparent;
    border: 2px dashed var(--border-medium);
    border-radius: 10px;
    color: var(--text-muted);
    font-family: var(--ui-font);
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                border-color var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .episodes-grid.compact .ep-add-card {
    flex-direction: row;
    min-height: 48px;
    gap: 10px;
    padding: 8px 14px;
    border-width: 1px;
  }

  .episodes-grid.compact .ep-add-card svg {
    width: 16px;
    height: 16px;
  }

  .episodes-grid.compact .ep-add-hint {
    display: none;
  }

  .ep-add-card:hover {
    background: var(--accent-muted);
    border-color: var(--accent);
    color: var(--accent);
  }

  .ep-add-label {
    font-size: 12.5px;
    font-weight: 600;
  }

  .ep-add-hint {
    font-size: 10.5px;
    color: var(--text-muted);
    letter-spacing: 0.02em;
  }

  .ep-add-card:hover .ep-add-hint {
    color: var(--accent);
    opacity: 0.8;
  }
</style>
