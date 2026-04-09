<script lang="ts">
  import { Fragment, type Node as PMNode } from 'prosemirror-model';
  import { flip } from 'svelte/animate';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';

  import { screenplaySchema } from '$lib/editor/schema';
  import { InputModeManager } from '$lib/editor/input/InputModeManager';
  import StatusBar from '$lib/components/StatusBar.svelte';

  // Map the font setting slug to a CSS font-family name
  let fontFamily = $derived(
    documentStore.currentFont === 'manjari' ? 'Manjari' : 'Noto Sans Malayalam'
  );

  const inputManager = InputModeManager.getInstance();

  /** Handle Malayalam input in card textareas */
  function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.code === 'Space') {
      event.preventDefault();
      inputManager.toggle();
      return;
    }
    if (!inputManager.isMalayalam) return;
    if (event.metaKey || event.ctrlKey) return;
    if (['Space', 'Enter', 'Backspace', 'Delete', 'ArrowLeft', 'ArrowRight',
         'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.code)) {
      inputManager.resetMozhi();
      return;
    }
    if (event.key.length === 1 && !event.altKey) {
      const result = inputManager.processKey(event.key);
      if (result !== null) {
        event.preventDefault();
        const ta = event.target as HTMLTextAreaElement;
        const start = ta.selectionStart;
        const end = ta.selectionEnd;
        const deleteFrom = start - result.deleteBack;
        const newValue = ta.value.substring(0, deleteFrom) + result.text + ta.value.substring(end);
        ta.value = newValue;
        const newPos = deleteFrom + result.text.length;
        ta.selectionStart = newPos;
        ta.selectionEnd = newPos;
        ta.dispatchEvent(new Event('input', { bubbles: true }));
      }
    }
  }

  /** Parsed scene data with auto-populated and manual fields */
  interface SceneCardData {
    sceneNumber: number;
    /** 0-based scene order for scene_cards lookup */
    sceneOrder: number;
    heading: string;
    location: string;
    time: string;
    characters: string[];
    pageEstimate: string;
    /** Index into the top-level ProseMirror content array */
    contentIndex: number;
    /** Manually written description */
    description: string;
    /** Manually written shoot notes */
    shootNotes: string;
  }

  // Drag state
  let dragFromScene = $state<number | null>(null);
  let dropTargetScene = $state<number | null>(null);
  let gridEl = $state<HTMLDivElement | null>(null);

  /** Parse INT./EXT., location, and time from a scene heading */
  function parseSceneHeading(heading: string): { location: string; time: string } {
    const match = heading.match(/^(?:INT\.|EXT\.|INT\.\/EXT\.)\s*(.+?)\s*-\s*(.+)$/i);
    if (match) {
      return { location: match[1].trim(), time: match[2].trim() };
    }
    return { location: heading, time: '' };
  }

  /** Build scene cards from the ProseMirror content and stored scene_cards data */
  let cards = $derived.by((): SceneCardData[] => {
    const doc = documentStore.document;
    if (!doc || !doc.content) return [];

    const content = doc.content as {
      type?: string;
      content?: Array<{
        type?: string;
        content?: Array<{ text?: string }>;
      }>;
    };
    if (!content.content) return [];

    const result: SceneCardData[] = [];
    const startNum = doc.settings?.scene_number_start ?? 1;
    let sceneNumber = startNum - 1;
    let sceneOrder = -1; // 0-based scene index for scene_cards lookup
    let currentCharacters: string[] = [];
    let currentCharCount = 0;
    let currentHeading = '';
    let currentLocation = '';
    let currentTime = '';
    let currentContentIndex = -1;

    function pushCurrentScene() {
      if (sceneNumber < startNum) return;
      const storedCard = doc.scene_cards.find((c) => c.scene_index === sceneOrder);
      const pages = Math.max(0.1, currentCharCount / 3000);
      result.push({
        sceneNumber,
        sceneOrder,
        heading: currentHeading,
        location: currentLocation,
        time: currentTime,
        characters: [...currentCharacters],
        pageEstimate: `~${pages.toFixed(1)} pages`,
        contentIndex: currentContentIndex,
        description: storedCard?.description ?? '',
        shootNotes: storedCard?.shoot_notes ?? '',
      });
    }

    for (let i = 0; i < content.content.length; i++) {
      const node = content.content[i];

      if (node.type === 'scene_heading') {
        pushCurrentScene();
        sceneNumber++;
        sceneOrder++;
        currentCharacters = [];
        currentCharCount = 0;
        currentContentIndex = i;

        currentHeading = (node.content ?? []).map((c) => c.text ?? '').join('');
        const parsed = parseSceneHeading(currentHeading);
        currentLocation = parsed.location;
        currentTime = parsed.time;
      } else if (node.type === 'character') {
        const name = (node.content ?? []).map((c) => c.text ?? '').join('').trim();
        if (name && !currentCharacters.includes(name)) {
          currentCharacters.push(name);
        }
      }

      if (node.content) {
        currentCharCount += node.content.reduce((sum, c) => sum + (c.text ?? '').length, 0);
      }
    }

    pushCurrentScene();
    return result;
  });

  /** Update the description for a scene card */
  function updateDescription(sceneIndex: number, value: string) {
    if (!documentStore.document) return;
    const existing = documentStore.document.scene_cards.find((c) => c.scene_index === sceneIndex);
    if (existing) {
      existing.description = value;
    } else {
      documentStore.document.scene_cards.push({
        scene_index: sceneIndex,
        description: value,
        shoot_notes: '',
      });
    }
    documentStore.markDirty();
  }

  /** Update the shoot notes for a scene card */
  function updateShootNotes(sceneIndex: number, value: string) {
    if (!documentStore.document) return;
    const existing = documentStore.document.scene_cards.find((c) => c.scene_index === sceneIndex);
    if (existing) {
      existing.shoot_notes = value;
    } else {
      documentStore.document.scene_cards.push({
        scene_index: sceneIndex,
        description: '',
        shoot_notes: value,
      });
    }
    documentStore.markDirty();
  }

  // --- Custom drag via mouse events ---
  // Same approach as SceneNavigator: mousedown/mousemove/mouseup
  // because HTML5 DnD is unreliable in Tauri's WebKit WebView.

  /**
   * Given x/y coordinates, determine which card the cursor is over
   * by checking the bounding rects of .card elements in the grid.
   */
  function sceneNumberAtPoint(clientX: number, clientY: number): number | null {
    if (!gridEl) return null;
    const cardEls = gridEl.querySelectorAll('.scene-card');
    for (let i = 0; i < cardEls.length; i++) {
      const rect = cardEls[i].getBoundingClientRect();
      if (clientX >= rect.left && clientX <= rect.right &&
          clientY >= rect.top && clientY <= rect.bottom) {
        return i + 1; // scene numbers are 1-based
      }
    }
    return null;
  }

  function handleMouseMove(event: MouseEvent) {
    if (dragFromScene === null) return;
    event.preventDefault();
    const target = sceneNumberAtPoint(event.clientX, event.clientY);
    if (target !== null && target !== dragFromScene) {
      dropTargetScene = target;
    } else {
      dropTargetScene = null;
    }
  }

  function handleMouseUp() {
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

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  }

  /**
   * Reorder a scene in the ProseMirror document.
   *
   * Same logic as SceneNavigator: a "scene" is a scene_heading node and all
   * nodes following it until the next scene_heading (or end of document).
   * Delete + insert in a single ProseMirror transaction for Cmd+Z undo.
   */
  function reorderScene(fromNumber: number, toNumber: number) {
    const view = editorStore.view;
    if (!view) return;

    const doc = view.state.doc;

    // Collect scene boundaries: { childIndex, offset } for each scene_heading
    const sceneBounds: { childIndex: number; offset: number }[] = [];
    doc.forEach((node, offset, index) => {
      if (node.type.name === 'scene_heading') {
        sceneBounds.push({ childIndex: index, offset });
      }
    });

    if (fromNumber < 1 || fromNumber > sceneBounds.length) return;
    if (toNumber < 1 || toNumber > sceneBounds.length) return;

    const fromIdx = fromNumber - 1;
    const toIdx = toNumber - 1;

    // Source scene child range
    const fromChildStart = sceneBounds[fromIdx].childIndex;
    const fromChildEnd = fromIdx + 1 < sceneBounds.length
      ? sceneBounds[fromIdx + 1].childIndex
      : doc.childCount;

    // Collect the source scene's nodes
    const sceneNodes: PMNode[] = [];
    for (let i = fromChildStart; i < fromChildEnd; i++) {
      sceneNodes.push(doc.child(i));
    }

    // Source scene position range
    const fromStartPos = sceneBounds[fromIdx].offset;
    const fromEndPos = fromIdx + 1 < sceneBounds.length
      ? sceneBounds[fromIdx + 1].offset
      : doc.content.size;

    // Insertion position
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
      tr.insert(insertPos, fragment);
      const shift = fragment.size;
      tr.delete(fromStartPos + shift, fromEndPos + shift);
    } else {
      tr.delete(fromStartPos, fromEndPos);
      const shift = fromEndPos - fromStartPos;
      tr.insert(insertPos - shift, fragment);
    }

    tr.scrollIntoView();
    view.dispatch(tr);

    // Remap scene_cards indices to follow the reordered scenes.
    // When a scene moves from fromIdx to toIdx, all indices in between shift.
    if (documentStore.document) {
      for (const card of documentStore.document.scene_cards) {
        if (card.scene_index === fromIdx) {
          card.scene_index = toIdx;
        } else if (fromIdx < toIdx) {
          // Moving forward: scenes between from+1 and to shift down by 1
          if (card.scene_index > fromIdx && card.scene_index <= toIdx) {
            card.scene_index--;
          }
        } else {
          // Moving backward: scenes between to and from-1 shift up by 1
          if (card.scene_index >= toIdx && card.scene_index < fromIdx) {
            card.scene_index++;
          }
        }
      }
    }

    documentStore.markDirty();
  }

  /** Add a new empty scene at the end of the ProseMirror document */
  function addScene() {
    const view = editorStore.view;
    if (!view) return;

    const doc = view.state.doc;
    const endPos = doc.content.size;
    const newHeading = screenplaySchema.node('scene_heading');
    const tr = view.state.tr.insert(endPos, newHeading);
    view.dispatch(tr);
    documentStore.setContent(view.state.doc.toJSON());
    documentStore.markDirty();
  }
</script>

<div class="scene-cards-view" style="--editor-font: '{fontFamily}'">
  <div class="cards-grid" bind:this={gridEl}>
    {#each cards as card (card.heading + card.sceneNumber)}
      <div
        class="card scene-card"
        class:dragging={dragFromScene === card.sceneNumber}
        class:drop-target={dropTargetScene === card.sceneNumber}
        animate:flip={{ duration: 300 }}
      >
        <div class="card-header">
          <!-- Scene number badge is the drag handle -->
          <span
            class="card-number"
            onmousedown={(e: MouseEvent) => startDrag(e, card.sceneNumber)}
            role="button"
            tabindex="-1"
            aria-label="Drag to reorder scene {card.sceneNumber}"
          >{card.sceneNumber}.</span>
          <span class="card-heading">{card.heading.toUpperCase()}</span>
        </div>
        {#if card.characters.length > 0}
          <div class="card-meta">
            <span class="meta-item">{card.characters.join(', ')}</span>
          </div>
        {/if}
        <div class="card-editable">
          <label class="field-label" for="desc-{card.sceneNumber}">Description</label>
          <textarea
            id="desc-{card.sceneNumber}"
            class="card-textarea"
            placeholder="What happens in this scene..."
            value={card.description}
            oninput={(e) => updateDescription(card.sceneOrder, (e.target as HTMLTextAreaElement).value)}
            onkeydown={handleKeydown}
          ></textarea>
          <label class="field-label" for="notes-{card.sceneNumber}">Notes</label>
          <textarea
            id="notes-{card.sceneNumber}"
            class="card-textarea"
            placeholder="Additional notes..."
            value={card.shootNotes}
            oninput={(e) => updateShootNotes(card.sceneOrder, (e.target as HTMLTextAreaElement).value)}
            onkeydown={handleKeydown}
          ></textarea>
        </div>
        <div class="card-footer">
          <span class="page-estimate">{card.pageEstimate}</span>
        </div>
      </div>
    {/each}

    <!-- Add Scene card -->
    <button class="card add-scene-card" onclick={addScene}>
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="12" y1="5" x2="12" y2="19"></line>
        <line x1="5" y1="12" x2="19" y2="12"></line>
      </svg>
      <span>Add Scene</span>
    </button>
  </div>
</div>

<style>
  .scene-cards-view {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    background: var(--surface-base);
    padding: 24px;
  }

  .cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    grid-auto-rows: 1fr;
    gap: 16px;
  }

  .card {
    background: var(--surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    overflow: hidden;
    transition: opacity 120ms ease, border-color 120ms ease;
    display: flex;
    flex-direction: column;
  }

  .card.dragging {
    opacity: 0.4;
  }

  .card.drop-target {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .card-header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .card-number {
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 12px;
    font-weight: 700;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
    cursor: grab;
    user-select: none;
    padding: 2px 4px;
    border-radius: 4px;
    transition: background 120ms ease;
  }

  .card-number:hover {
    background: var(--accent-muted);
  }

  .card-number:active {
    cursor: grabbing;
  }

  .card-heading {
    font-family: var(--editor-font), system-ui, -apple-system, sans-serif;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-meta {
    padding: 6px 16px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .meta-item {
    font-family: var(--editor-font), system-ui, -apple-system, sans-serif;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .card-footer {
    padding: 3px 16px;
    border-top: 1px solid var(--border-subtle);
    margin-top: auto;
  }

  .page-estimate {
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 10px;
    color: var(--text-muted);
    font-style: italic;
  }

  .card-editable {
    padding: 10px 16px 14px;
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .field-label {
    display: block;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 4px;
    margin-top: 8px;
  }

  .field-label:first-child {
    margin-top: 0;
  }

  .card-textarea {
    width: 100%;
    padding: 6px 8px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-primary);
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    font-family: var(--editor-font), system-ui, -apple-system, sans-serif;
    resize: none;
    box-sizing: border-box;
    transition: border-color 120ms ease;
    flex: 1;
    min-height: 60px;
  }

  .card-textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  .card-textarea::placeholder {
    color: var(--text-muted);
  }

  /* ─── Add Scene card ─── */
  .add-scene-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: 2px dashed var(--border-subtle);
    background: transparent;
    color: var(--text-muted);
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: border-color 120ms ease, color 120ms ease, background 120ms ease;
  }

  .add-scene-card:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-muted);
  }
</style>
