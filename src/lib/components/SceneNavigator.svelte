<script lang="ts">
  import { Fragment, type Node as PMNode } from 'prosemirror-model';
  import { TextSelection } from 'prosemirror-state';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';
  import { screenplaySchema } from '$lib/editor/schema';

  // Scene heading extracted from ProseMirror JSON content
  type SetLocation = 'INT' | 'EXT' | 'INT_EXT' | null;
  type TimeOfDay = 'DAY' | 'NIGHT' | 'DAWN' | 'DUSK' | 'EVENING' | 'MORNING' | 'AFTERNOON' | 'CONTINUOUS' | 'LATER' | null;
  interface SceneEntry {
    number: number;
    text: string;
    // Index of this scene_heading in the top-level content array
    index: number;
    setting: SetLocation;
    time: TimeOfDay;
    /** "0.8" — one decimal place, always 0.1 minimum */
    pages: string;
  }

  /** Extract INT/EXT prefix from the heading text. */
  function parseSetting(heading: string): SetLocation {
    const h = heading.trim().toUpperCase();
    if (/^INT\.?\/EXT\.?\b|^I\/E\b/.test(h)) return 'INT_EXT';
    if (/^INT\.?\b/.test(h)) return 'INT';
    if (/^EXT\.?\b/.test(h)) return 'EXT';
    return null;
  }

  /** Extract time-of-day, if present, from the heading's trailing segment. */
  function parseTime(heading: string): TimeOfDay {
    // Take the last segment after the final dash-like separator.
    const segments = heading.split(/\s[-–—]\s|\s-\s/);
    const tail = segments[segments.length - 1]?.trim().toUpperCase() ?? '';
    if (/\bNIGHT\b/.test(tail)) return 'NIGHT';
    if (/\bDAWN\b/.test(tail)) return 'DAWN';
    if (/\bDUSK\b/.test(tail)) return 'DUSK';
    if (/\bEVENING\b/.test(tail)) return 'EVENING';
    if (/\bMORNING\b/.test(tail)) return 'MORNING';
    if (/\bAFTERNOON\b/.test(tail)) return 'AFTERNOON';
    if (/\bCONTINUOUS\b/.test(tail)) return 'CONTINUOUS';
    if (/\bLATER\b/.test(tail)) return 'LATER';
    if (/\bDAY\b/.test(tail)) return 'DAY';
    return null;
  }

  // Drag state — managed via mousedown/mousemove/mouseup on the drag handle
  let dragFromScene = $state<number | null>(null);
  let dropTargetScene = $state<number | null>(null);
  let listEl = $state<HTMLUListElement | null>(null);

  // Extract scene headings from the ProseMirror JSON document.
  // Reads the *active* content — top-level for films, active episode for series —
  // so this component works for both project shapes without branching.
  let scenes = $derived.by(() => {
    const doc = documentStore.document;
    if (!doc) return [];
    const rawContent = documentStore.activeContent;
    if (!rawContent) return [];

    const content = rawContent as { type?: string; content?: Array<{ type?: string; content?: Array<{ text?: string }> }> };
    if (!content.content) return [];

    const entries: SceneEntry[] = [];
    const startNum = documentStore.activeSettings?.scene_number_start ?? 1;
    let sceneNumber = startNum - 1;

    // Accumulate body character count for the current scene so we can emit
    // a page estimate at the same moment we emit the next scene's entry.
    let currentEntry: SceneEntry | null = null;
    let currentChars = 0;

    const nodeText = (n: { content?: Array<{ text?: string }> }): string =>
      (n.content ?? []).map((c) => c.text ?? '').join('');

    const finalize = () => {
      if (currentEntry) {
        const pages = Math.max(0.1, currentChars / 3000);
        currentEntry.pages = pages.toFixed(1);
      }
    };

    content.content.forEach((node, index) => {
      if (node.type === 'scene_heading') {
        finalize();
        sceneNumber++;
        const text = nodeText(node) || '(empty)';
        currentEntry = {
          number: sceneNumber,
          text,
          index,
          setting: parseSetting(text),
          time: parseTime(text),
          pages: '0.1',
        };
        entries.push(currentEntry);
        // Include the heading text itself in the page count for the scene
        // so a scene with only a heading still registers a non-trivial size.
        currentChars = text.length;
      } else if (currentEntry) {
        currentChars += nodeText(node).length;
      }
    });
    finalize();

    return entries;
  });

  // Add a blank scene heading at the end of the document and focus it.
  // Mirrors the Cards view's "Add Scene" behavior so both entry points
  // stay in sync.
  function addScene() {
    const view = editorStore.view;
    if (!view) return;

    const doc = view.state.doc;
    const endPos = doc.content.size;
    const newHeading = screenplaySchema.node('scene_heading');
    const tr = view.state.tr.insert(endPos, newHeading);
    // Place cursor inside the new heading so the user can start typing
    // immediately (endPos + 1 lands inside the newly inserted node).
    tr.setSelection(TextSelection.create(tr.doc, endPos + 1));
    tr.scrollIntoView();
    view.dispatch(tr);
    documentStore.setContent(view.state.doc.toJSON());
    documentStore.markDirty();
    view.focus();
  }

  // Navigate to a scene heading in the editor
  function scrollToScene(sceneIndex: number) {
    const view = editorStore.view;
    if (!view) return;

    // Find the document position for cursor placement
    let targetPos = -1;
    const doc = view.state.doc;
    doc.forEach((_node, offset, index) => {
      if (index === sceneIndex) {
        targetPos = offset + 1;
      }
    });

    if (targetPos === -1) return;

    // Set the cursor on the target scene heading — do NOT call
    // scrollIntoView() on the transaction so it doesn't fight
    // with our manual scroll below.
    const tr = view.state.tr.setSelection(
      TextSelection.create(view.state.doc, targetPos)
    );
    view.dispatch(tr);
    view.focus();

    // Defer the scroll to the next frame so the DOM has settled
    // after the dispatch. Without this, getBoundingClientRect()
    // can return stale positions, causing the scroll to land wrong
    // or not fire at all on rapid clicks.
    requestAnimationFrame(() => {
      // Find the scene heading DOM elements after DOM has updated
      const headings = view.dom.querySelectorAll('.scene-heading');

      // Map the document node index to scene heading index
      let sceneCount = -1;
      let targetHeadingIndex = -1;
      view.state.doc.forEach((node, _offset, index) => {
        if (node.type.name === 'scene_heading') {
          sceneCount++;
          if (index === sceneIndex) {
            targetHeadingIndex = sceneCount;
          }
        }
      });

      const sceneEl = targetHeadingIndex >= 0 ? headings[targetHeadingIndex] : null;
      const scrollContainer = view.dom.closest('.editor-scroll') ?? view.dom.parentElement?.parentElement;

      if (scrollContainer && sceneEl) {
        const sceneRect = sceneEl.getBoundingClientRect();
        const containerRect = scrollContainer.getBoundingClientRect();
        const targetScroll = scrollContainer.scrollTop + (sceneRect.top - containerRect.top) - 20;
        scrollContainer.scrollTo({ top: Math.max(0, targetScroll), behavior: 'instant' });
      }
    });
  }

  // --- Custom drag via mouse events ---
  // HTML5 Drag and Drop doesn't work reliably in Tauri's WebKit WebView,
  // so we implement dragging with mousedown/mousemove/mouseup instead.

  /**
   * Given a Y coordinate, determine which scene the cursor is over
   * by checking the bounding rects of .scene-li elements.
   */
  function sceneNumberAtY(clientY: number): number | null {
    if (!listEl) return null;
    const items = listEl.querySelectorAll('.scene-li');
    for (let i = 0; i < items.length; i++) {
      const rect = items[i].getBoundingClientRect();
      if (clientY >= rect.top && clientY <= rect.bottom) {
        return i + 1; // scene numbers are 1-based
      }
    }
    return null;
  }

  function handleMouseMove(event: MouseEvent) {
    if (dragFromScene === null) return;
    event.preventDefault();
    const target = sceneNumberAtY(event.clientY);
    if (target !== null && target !== dragFromScene) {
      dropTargetScene = target;
    } else {
      dropTargetScene = null;
    }
  }

  function handleMouseUp(_event: MouseEvent) {
    if (dragFromScene === null) return;

    const from = dragFromScene;
    const to = dropTargetScene;

    // Reset state
    dragFromScene = null;
    dropTargetScene = null;

    // Remove listeners
    window.removeEventListener('mousemove', handleMouseMove);
    window.removeEventListener('mouseup', handleMouseUp);

    if (to !== null && from !== to) {
      reorderScene(from, to);
    }
  }

  function startDrag(event: MouseEvent, sceneNumber: number) {
    event.preventDefault();
    dragFromScene = sceneNumber;

    // Attach window-level listeners so dragging works even if cursor
    // leaves the list area — cleanup happens in mouseup.
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  }

  /**
   * Reorder a scene in the ProseMirror document.
   *
   * A "scene" is a scene_heading node and all following nodes until the next
   * scene_heading (or end of document). We cut the source scene's node range
   * and insert it at the target position — all in a single transaction so
   * Cmd+Z undoes it in one step.
   */
  function reorderScene(fromNumber: number, toNumber: number) {
    const view = editorStore.view;
    if (!view) return;

    const doc = view.state.doc;

    // Collect scene boundaries: each entry is { childIndex, offset }
    // where offset is the position before the scene_heading node
    // within the parent's content (= absolute doc position for top-level nodes).
    const sceneBounds: { childIndex: number; offset: number }[] = [];
    doc.forEach((node, offset, index) => {
      if (node.type.name === 'scene_heading') {
        sceneBounds.push({ childIndex: index, offset });
      }
    });

    if (fromNumber < 1 || fromNumber > sceneBounds.length) return;
    if (toNumber < 1 || toNumber > sceneBounds.length) return;

    // Convert 1-based scene numbers to 0-based indices
    const fromIdx = fromNumber - 1;
    const toIdx = toNumber - 1;

    // Source scene child range
    const fromChildStart = sceneBounds[fromIdx].childIndex;
    const fromChildEnd = fromIdx + 1 < sceneBounds.length
      ? sceneBounds[fromIdx + 1].childIndex
      : doc.childCount;

    // Collect the ProseMirror nodes that make up the source scene
    const sceneNodes: PMNode[] = [];
    for (let i = fromChildStart; i < fromChildEnd; i++) {
      sceneNodes.push(doc.child(i));
    }

    // Source scene position range
    const fromStartPos = sceneBounds[fromIdx].offset;
    const fromEndPos = fromIdx + 1 < sceneBounds.length
      ? sceneBounds[fromIdx + 1].offset
      : doc.content.size;

    // Insertion position:
    // Moving up → insert before target scene.
    // Moving down → insert after target scene (before the scene after target).
    let insertPos: number;
    if (toNumber < fromNumber) {
      insertPos = sceneBounds[toIdx].offset;
    } else {
      insertPos = toIdx + 1 < sceneBounds.length
        ? sceneBounds[toIdx + 1].offset
        : doc.content.size;
    }

    const fragment = Fragment.from(sceneNodes);
    const tr = view.state.tr;

    if (insertPos <= fromStartPos) {
      // Inserting before source — insert first, then delete (shifted positions)
      tr.insert(insertPos, fragment);
      const shift = fragment.size;
      tr.delete(fromStartPos + shift, fromEndPos + shift);
    } else {
      // Inserting after source — delete first, then insert (adjusted position)
      tr.delete(fromStartPos, fromEndPos);
      const shift = fromEndPos - fromStartPos;
      tr.insert(insertPos - shift, fragment);
    }

    tr.scrollIntoView();
    view.dispatch(tr);

    // Mark document dirty
    documentStore.markDirty();

    // Scroll to the moved scene after DOM updates
    const newSceneNumber = toNumber;
    requestAnimationFrame(() => {
      const newView = editorStore.view;
      if (!newView) return;

      const newDoc = newView.state.doc;
      let sceneCount = 0;
      let targetNodePos = -1;

      newDoc.forEach((node, offset) => {
        if (node.type.name === 'scene_heading') {
          sceneCount++;
          if (sceneCount === newSceneNumber) {
            targetNodePos = offset + 1;
          }
        }
      });

      if (targetNodePos === -1) return;

      const scrollTr = newView.state.tr.setSelection(
        TextSelection.create(newView.state.doc, targetNodePos)
      );
      scrollTr.scrollIntoView();
      newView.dispatch(scrollTr);
      newView.focus();
    });
  }
</script>

<div class="navigator-content">
  {#if scenes.length === 0}
    <div class="empty-state">
      <p class="empty-title">No scenes yet</p>
      <p class="empty-hint">Start with a scene heading like <em>INT. ROOM — DAY</em>.</p>
      <button class="empty-cta" onclick={addScene}>
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
        Add scene
      </button>
    </div>
  {:else}
    <ul class="scene-list" bind:this={listEl}>
      {#each scenes as scene (scene.index)}
        <li
          class="scene-li"
          class:drop-above={dropTargetScene === scene.number && dragFromScene !== null && dragFromScene > scene.number}
          class:drop-below={dropTargetScene === scene.number && dragFromScene !== null && dragFromScene < scene.number}
          class:dragging={dragFromScene === scene.number}
        >
          <!-- Drag handle: mousedown starts the custom drag operation -->
          <span
            class="drag-handle"
            onmousedown={(e: MouseEvent) => startDrag(e, scene.number)}
            role="button"
            tabindex="-1"
            aria-label="Drag to reorder scene {scene.number}"
          >
            <svg width="10" height="14" viewBox="0 0 10 14" fill="currentColor" aria-hidden="true">
              <circle cx="2" cy="3" r="1.1"/>
              <circle cx="8" cy="3" r="1.1"/>
              <circle cx="2" cy="7" r="1.1"/>
              <circle cx="8" cy="7" r="1.1"/>
              <circle cx="2" cy="11" r="1.1"/>
              <circle cx="8" cy="11" r="1.1"/>
            </svg>
          </span>
          <button
            class="scene-item"
            data-time={scene.time ?? ''}
            onclick={() => scrollToScene(scene.index)}
            title={scene.time ? `${scene.text.toUpperCase()} — ~${scene.pages} pages` : `${scene.text.toUpperCase()} — ~${scene.pages} pages`}
          >
            <span class="scene-number">{scene.number}.</span>
            <span class="scene-text">{scene.text.toUpperCase()}</span>
            <span class="page-pill" title="~{scene.pages} pages">{scene.pages}p</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .navigator-content {
    padding: 12px;
    overflow-y: auto;
    height: 100%;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 4px;
    font-family: system-ui, -apple-system, sans-serif;
  }

  .empty-title {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .empty-hint {
    margin: 0;
    font-size: 11px;
    line-height: 1.45;
    color: var(--text-muted);
  }

  .empty-hint em {
    font-style: normal;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 10.5px;
    color: var(--text-secondary);
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 3px;
    padding: 0 4px;
  }

  .empty-cta {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 2px;
    padding: 5px 10px;
    font-family: inherit;
    font-size: 11px;
    font-weight: 500;
    color: var(--accent);
    background: var(--accent-muted);
    border: 1px solid transparent;
    border-radius: 6px;
    cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease;
  }

  .empty-cta:hover {
    background: var(--accent);
    color: #fff;
  }

  .scene-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .scene-li {
    position: relative;
    display: flex;
    align-items: center;
  }

  .scene-li.dragging {
    opacity: 0.4;
  }

  /* Drop indicator line — teal line above or below the target item */
  .scene-li.drop-above::before {
    content: '';
    position: absolute;
    top: 0;
    left: 8px;
    right: 8px;
    height: 2px;
    background: var(--accent);
    border-radius: 1px;
    z-index: 1;
  }

  .scene-li.drop-below::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 8px;
    right: 8px;
    height: 2px;
    background: var(--accent);
    border-radius: 1px;
    z-index: 1;
  }

  .drag-handle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 32px;
    flex-shrink: 0;
    color: var(--text-muted);
    cursor: grab;
    opacity: 0.35;
    transition: opacity 120ms ease;
    user-select: none;
  }

  .drag-handle:active {
    cursor: grabbing;
  }

  .scene-li:hover .drag-handle {
    opacity: 1;
  }

  .scene-item {
    position: relative;
    display: flex;
    align-items: baseline;
    gap: 6px;
    flex: 1;
    min-width: 0;
    height: 32px;
    padding: 0 8px 0 8px;
    border: none;
    border-left: 2px solid transparent;
    border-radius: 0 4px 4px 0;
    background: transparent;
    color: var(--text-secondary);
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    line-height: 32px;
    transition: background 120ms ease, color 120ms ease;
  }

  /* Time-of-day stripe: a 2px bar at the left edge of the row so the
     outline reads as a rhythm of warm/cool blocks at a glance. No extra
     horizontal space is consumed. */
  .scene-item[data-time='DAY']::before,
  .scene-item[data-time='MORNING']::before,
  .scene-item[data-time='AFTERNOON']::before,
  .scene-item[data-time='DAWN']::before {
    content: '';
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 2px;
    border-radius: 1px;
    background: var(--accent-warm);
  }
  .scene-item[data-time='NIGHT']::before,
  .scene-item[data-time='DUSK']::before,
  .scene-item[data-time='EVENING']::before {
    content: '';
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 2px;
    border-radius: 1px;
    background: var(--accent-deep);
  }
  .scene-item[data-time='CONTINUOUS']::before,
  .scene-item[data-time='LATER']::before {
    content: '';
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 2px;
    border-radius: 1px;
    background: var(--text-muted);
    opacity: 0.5;
  }

  .scene-item:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .scene-item:active {
    background: var(--accent-muted);
    border-left-color: var(--accent);
    color: var(--text-primary);
  }

  .scene-number {
    color: var(--text-muted);
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
    font-size: 11px;
  }

  .scene-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  /* Page pill — compact marker at the far right. Muted at rest, shows
     on hover so it doesn't compete with the heading text at rest. */
  .page-pill {
    flex-shrink: 0;
    margin-left: auto;
    padding: 0 4px;
    border-radius: 6px;
    color: var(--text-muted);
    font-size: 10px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    line-height: 1.4;
    letter-spacing: 0.02em;
    opacity: 0;
    transition: opacity 120ms ease, color 120ms ease;
  }

  .scene-item:hover .page-pill,
  .scene-item:focus-visible .page-pill {
    opacity: 0.8;
    color: var(--text-secondary);
  }
</style>
