<script lang="ts">
  import { Fragment, type Node as PMNode } from 'prosemirror-model';
  import { TextSelection } from 'prosemirror-state';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';

  // Scene heading extracted from ProseMirror JSON content
  interface SceneEntry {
    number: number;
    text: string;
    // Index of this scene_heading in the top-level content array
    index: number;
  }

  // Drag state — managed via mousedown/mousemove/mouseup on the drag handle
  let dragFromScene = $state<number | null>(null);
  let dropTargetScene = $state<number | null>(null);
  let listEl = $state<HTMLUListElement | null>(null);

  // Extract scene headings from the ProseMirror JSON document.
  let scenes = $derived.by(() => {
    const doc = documentStore.document;
    if (!doc || !doc.content) return [];

    const content = doc.content as { type?: string; content?: Array<{ type?: string; content?: Array<{ text?: string }> }> };
    if (!content.content) return [];

    const entries: SceneEntry[] = [];
    let sceneNumber = 0;

    content.content.forEach((node, index) => {
      if (node.type === 'scene_heading') {
        sceneNumber++;
        let text = '';
        if (node.content) {
          text = node.content
            .map((child) => child.text ?? '')
            .join('');
        }
        entries.push({
          number: sceneNumber,
          text: text || '(empty)',
          index,
        });
      }
    });

    return entries;
  });

  // Navigate to a scene heading in the editor
  function scrollToScene(sceneIndex: number) {
    const view = editorStore.view;
    if (!view) return;

    const doc = view.state.doc;
    let targetPos = -1;

    doc.forEach((node, offset, index) => {
      if (index === sceneIndex) {
        targetPos = offset + 1;
      }
    });

    if (targetPos === -1) return;

    // Set the cursor on the target scene heading
    const tr = view.state.tr.setSelection(
      TextSelection.create(view.state.doc, targetPos)
    );
    view.dispatch(tr);

    // Scroll the scene heading to the top of the visible area instead of
    // ProseMirror's default which just makes the cursor barely visible
    // (often at the bottom of the viewport).
    const coords = view.coordsAtPos(targetPos);
    const scrollContainer = view.dom.closest('.editor-scroll') ?? view.dom.parentElement?.parentElement;
    if (scrollContainer) {
      const containerRect = scrollContainer.getBoundingClientRect();
      const scrollOffset = coords.top - containerRect.top - 40; // 40px padding from top
      scrollContainer.scrollBy({ top: scrollOffset, behavior: 'smooth' });
    }

    view.focus();
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
    <p class="empty-message">No scenes yet</p>
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
          >⠿</span>
          <button
            class="scene-item"
            onclick={() => scrollToScene(scene.index)}
          >
            <span class="scene-number">{scene.number}.</span>
            <span class="scene-text">{scene.text.toUpperCase()}</span>
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

  .empty-message {
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 12px;
    color: var(--text-muted);
    padding: 0 4px;
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
    font-size: 10px;
    color: var(--text-muted);
    cursor: grab;
    opacity: 0;
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
    display: flex;
    align-items: baseline;
    gap: 6px;
    flex: 1;
    min-width: 0;
    height: 32px;
    padding: 0 8px 0 4px;
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
  }
</style>
