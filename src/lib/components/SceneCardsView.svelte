<script lang="ts">
  import { Fragment, type Node as PMNode } from 'prosemirror-model';
  import { untrack } from 'svelte';
  import { flip } from 'svelte/animate';
  import { cubicInOut } from 'svelte/easing';
  import { confirm } from '@tauri-apps/plugin-dialog';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';

  import { screenplaySchema } from '$lib/editor/schema';
  import { InputModeManager } from '$lib/editor/input/InputModeManager';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import EpisodeCardsView from '$lib/components/EpisodeCardsView.svelte';

  // Map the font setting slug to a CSS font-family name
  let fontFamily = $derived(
    documentStore.currentFont === 'manjari' ? 'Manjari' : 'Noto Sans Malayalam'
  );

  const inputManager = InputModeManager.getInstance();

  // ─── Card view level (#134) ───────────────────────────────────────
  // For series projects, the Card View opens at the Episode level by
  // default — episodes-first matches how writers plan top-down. Clicking
  // an episode card drills into the per-episode scene grid; the
  // breadcrumb up top hands the writer back. For non-series projects we
  // skip the level entirely (always 'scenes').
  type CardLevel = 'episodes' | 'scenes';

  /** Storage key — per-project so each script remembers its own drill state.
   *  Untitled docs share one key; that's a small UX compromise but avoids
   *  losing state across launches for the dominant case. */
  function storageKey(): string {
    const path = documentStore.currentPath ?? '__untitled__';
    return `scriptty-card-level:${path}`;
  }

  function initialLevel(): CardLevel {
    if (!documentStore.isSeries) return 'scenes';
    if (typeof localStorage === 'undefined') return 'episodes';
    const stored = localStorage.getItem(storageKey());
    return stored === 'scenes' ? 'scenes' : 'episodes';
  }

  let cardLevel = $state<CardLevel>(initialLevel());

  /** Reset the level when the document changes (open / new). The path
   *  changes on those flows, so re-seeding from localStorage gives the
   *  newly-loaded project its own remembered drill state. */
  $effect(() => {
    documentStore.currentPath; // dependency
    documentStore.isSeries;    // dependency
    cardLevel = initialLevel();
  });

  /** Persist on change. */
  $effect(() => {
    if (typeof localStorage === 'undefined') return;
    if (documentStore.isSeries) {
      localStorage.setItem(storageKey(), cardLevel);
    }
  });

  function openEpisode(index: number) {
    documentStore.setActiveEpisode(index);
    cardLevel = 'scenes';
  }

  function backToEpisodes() {
    cardLevel = 'episodes';
  }

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
    /** Comma-separated non-speaking characters (extras present in the scene) */
    extraCharacters: string;
    /** Shoot date — ISO or free-form ("Day 3"). Empty = unscheduled (#124). */
    scheduledDate: string;
    /** Free-text grouping tag for shoot planning (#124). */
    locationGroup: string;
    /**
     * Stable identity for {#each}/animate:flip. Derived from the heading text
     * plus an occurrence counter so duplicate headings still get unique keys.
     * Must NOT depend on sceneNumber or sceneOrder — those change on reorder,
     * which would break FLIP tracking and skip the rearrangement animation.
     */
    key: string;
  }

  // Drag state
  let dragFromScene = $state<number | null>(null);
  let dropTargetScene = $state<number | null>(null);
  let gridEl = $state<HTMLDivElement | null>(null);

  /** When true, render cards clustered by Location group → Shoot date so
   *  the writer can see the shoot plan as a sequence rather than the
   *  document's narrative order. Drag-to-reorder is disabled in this
   *  mode (it would still mutate document order, which would be
   *  surprising when the visual order is sorted). (#124) */
  let groupByLocation = $state(false);

  /** Cards in the order they should render — either the raw cards array
   *  (document order, for normal editing) or grouped by location_group
   *  with empty-group cards last and within-group sort by shoot date. */
  let displayCards = $derived.by(() => {
    if (!groupByLocation) return cards;
    return [...cards].sort((a, b) => {
      // Ungrouped cards (empty location_group) go to the end.
      const aGroup = a.locationGroup.trim();
      const bGroup = b.locationGroup.trim();
      if (!aGroup && bGroup) return 1;
      if (aGroup && !bGroup) return -1;
      if (aGroup !== bGroup) return aGroup.localeCompare(bGroup);
      // Same group — sort by shoot date (string compare works for ISO
      // dates and stays sensible-ish for free-form values like "Day 3").
      const aDate = a.scheduledDate.trim();
      const bDate = b.scheduledDate.trim();
      if (!aDate && bDate) return 1;
      if (aDate && !bDate) return -1;
      if (aDate !== bDate) return aDate.localeCompare(bDate);
      // Final tiebreaker — preserve document order so the writer's
      // intended sequence shows through within a group/date.
      return a.sceneNumber - b.sceneNumber;
    });
  });

  // Floating "ghost" card that follows the cursor while dragging. The original
  // card stays in its grid slot (dimmed) so the layout doesn't collapse; the
  // ghost is a cloned copy positioned with fixed coordinates.
  let ghostEl = $state<HTMLDivElement | null>(null);
  let ghostVisible = $state(false);
  let ghostX = $state(0);
  let ghostY = $state(0);
  let ghostW = $state(0);
  let ghostH = $state(0);
  // Cursor offset within the dragged card, so the ghost tracks the exact
  // pick-up point instead of snapping its top-left corner to the cursor.
  let ghostOffsetX = 0;
  let ghostOffsetY = 0;

  /** Parse INT./EXT., location, and time from a scene heading */
  function parseSceneHeading(heading: string): { location: string; time: string } {
    const match = heading.match(/^(?:INT\.|EXT\.|INT\.\/EXT\.)\s*(.+?)\s*-\s*(.+)$/i);
    if (match) {
      return { location: match[1].trim(), time: match[2].trim() };
    }
    return { location: heading, time: '' };
  }

  /** Build scene cards from the ProseMirror content and stored scene_cards data.
   *
   *  Perf (#99): tracks ONLY contentVersionDebounced — the actual content,
   *  active settings, and stored scene_cards are read inside untrack() so the
   *  walk runs once per typing burst (~200ms idle) instead of once per
   *  keystroke. The view is only mounted when active, but the derive is also
   *  needed for the editor view (Editor.svelte reads `documentStore
   *  .activeSceneCards` for the gutter), so a real derive is still right —
   *  just one that doesn't churn. */
  let cards = $derived.by((): SceneCardData[] => {
    documentStore.contentVersionDebounced;
    return untrack(() => computeCards());
  });

  function computeCards(): SceneCardData[] {
    const doc = documentStore.document;
    if (!doc) return [];
    const rawContent = documentStore.activeContent;
    if (!rawContent) return [];

    // Capture into a local const so the inner closure sees a stable value.
    const sceneCards = documentStore.activeSceneCards;

    const content = rawContent as {
      type?: string;
      content?: Array<{
        type?: string;
        content?: Array<{ text?: string }>;
      }>;
    };
    if (!content.content) return [];

    const result: SceneCardData[] = [];
    const startNum = documentStore.activeSettings?.scene_number_start ?? 1;
    let sceneNumber = startNum - 1;
    let sceneOrder = -1; // 0-based scene index for scene_cards lookup
    let currentCharacters: string[] = [];
    let currentCharCount = 0;
    let currentHeading = '';
    let currentLocation = '';
    let currentTime = '';
    let currentContentIndex = -1;

    // Track occurrence counter per heading so duplicate headings still get
    // unique, position-independent keys (e.g. "INT. KITCHEN#0", "INT. KITCHEN#1").
    const headingCounts = new Map<string, number>();

    function pushCurrentScene() {
      if (sceneNumber < startNum) return;
      const storedCard = sceneCards.find((c) => c.scene_index === sceneOrder);
      const pages = Math.max(0.1, currentCharCount / 3000);
      const occurrence = headingCounts.get(currentHeading) ?? 0;
      headingCounts.set(currentHeading, occurrence + 1);
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
        extraCharacters: storedCard?.extra_characters ?? '',
        scheduledDate: storedCard?.scheduled_date ?? '',
        locationGroup: storedCard?.location_group ?? '',
        key: `${currentHeading}#${occurrence}`,
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
  }

  /** Defaults for a freshly-created SceneCard — keeps the create paths
   *  in sync as the schema grows (#124 added two new optional fields). */
  function blankCard(sceneIndex: number) {
    return {
      scene_index: sceneIndex,
      description: '',
      shoot_notes: '',
      extra_characters: '',
      scheduled_date: '',
      location_group: '',
    };
  }

  /** Update the description for a scene card */
  function updateDescription(sceneIndex: number, value: string) {
    if (!documentStore.document) return;
    const existing = documentStore.activeSceneCards.find((c) => c.scene_index === sceneIndex);
    if (existing) {
      existing.description = value;
    } else {
      documentStore.activeSceneCards.push({ ...blankCard(sceneIndex), description: value });
    }
    documentStore.markDirty();
  }

  /** Update the shoot notes for a scene card */
  function updateShootNotes(sceneIndex: number, value: string) {
    if (!documentStore.document) return;
    const existing = documentStore.activeSceneCards.find((c) => c.scene_index === sceneIndex);
    if (existing) {
      existing.shoot_notes = value;
    } else {
      documentStore.activeSceneCards.push({ ...blankCard(sceneIndex), shoot_notes: value });
    }
    documentStore.markDirty();
  }

  /** Update the non-speaking characters list for a scene card */
  function updateExtraCharacters(sceneIndex: number, value: string) {
    if (!documentStore.document) return;
    const existing = documentStore.activeSceneCards.find((c) => c.scene_index === sceneIndex);
    if (existing) {
      existing.extra_characters = value;
    } else {
      documentStore.activeSceneCards.push({ ...blankCard(sceneIndex), extra_characters: value });
    }
    documentStore.markDirty();
  }

  /** Update the scheduled shoot date for a scene card (#124). */
  function updateScheduledDate(sceneIndex: number, value: string) {
    if (!documentStore.document) return;
    const existing = documentStore.activeSceneCards.find((c) => c.scene_index === sceneIndex);
    if (existing) {
      existing.scheduled_date = value;
    } else {
      documentStore.activeSceneCards.push({ ...blankCard(sceneIndex), scheduled_date: value });
    }
    documentStore.markDirty();
  }

  /** Update the free-text location group for a scene card (#124). */
  function updateLocationGroup(sceneIndex: number, value: string) {
    if (!documentStore.document) return;
    const existing = documentStore.activeSceneCards.find((c) => c.scene_index === sceneIndex);
    if (existing) {
      existing.location_group = value;
    } else {
      documentStore.activeSceneCards.push({ ...blankCard(sceneIndex), location_group: value });
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
    // Move the floating ghost with the cursor
    ghostX = event.clientX - ghostOffsetX;
    ghostY = event.clientY - ghostOffsetY;

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

    // Tear down the ghost
    ghostVisible = false;
    if (ghostEl) ghostEl.replaceChildren();

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

    // Build the floating ghost from a deep clone of the source card so it
    // visually matches (avoids re-rendering the Svelte template inside a
    // portal). We strip state classes that would otherwise make the ghost
    // look semi-transparent or highlighted.
    const trigger = event.currentTarget as HTMLElement | null;
    const cardEl = trigger?.closest('.scene-card') as HTMLElement | null;
    if (cardEl && ghostEl) {
      const rect = cardEl.getBoundingClientRect();
      ghostOffsetX = event.clientX - rect.left;
      ghostOffsetY = event.clientY - rect.top;
      ghostW = rect.width;
      ghostH = rect.height;
      ghostX = event.clientX - ghostOffsetX;
      ghostY = event.clientY - ghostOffsetY;

      const clone = cardEl.cloneNode(true) as HTMLElement;
      clone.classList.remove('dragging', 'drop-target');
      clone.style.width = '100%';
      clone.style.height = '100%';
      clone.style.margin = '0';
      ghostEl.replaceChildren(clone);
      ghostVisible = true;
    }

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
      for (const card of documentStore.activeSceneCards) {
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

  /**
   * Delete a scene from the ProseMirror document.
   *
   * A "scene" is the scene_heading node plus every following top-level node
   * until the next scene_heading (or end of document). We delete that range
   * in a single transaction and then remap scene_cards indices so cards for
   * later scenes shift up to match their new 0-based positions.
   */
  async function deleteScene(sceneNumber: number, sceneOrder: number) {
    const view = editorStore.view;
    if (!view) return;

    let ok: boolean;
    try {
      ok = await confirm(
        `Delete scene ${sceneNumber} and its contents? This cannot be undone from the cards view.`,
        { title: 'Delete scene', kind: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }
      );
    } catch (err) {
      // Surface permission / plugin errors instead of silently dropping the click.
      console.error('Delete confirmation dialog failed', err);
      return;
    }
    if (!ok) return;

    const doc = view.state.doc;

    // Collect scene boundaries: { childIndex, offset } per scene_heading
    const sceneBounds: { childIndex: number; offset: number }[] = [];
    doc.forEach((node, offset, index) => {
      if (node.type.name === 'scene_heading') {
        sceneBounds.push({ childIndex: index, offset });
      }
    });

    if (sceneOrder < 0 || sceneOrder >= sceneBounds.length) return;

    const startPos = sceneBounds[sceneOrder].offset;
    const endPos = sceneOrder + 1 < sceneBounds.length
      ? sceneBounds[sceneOrder + 1].offset
      : doc.content.size;

    const tr = view.state.tr.delete(startPos, endPos);
    view.dispatch(tr);

    // If the doc is now empty (scene_heading was the only block), ProseMirror
    // enforces the `block+` rule on `doc` — we leave it as-is; next transaction
    // will add one. Actually: after deleting the only scene, doc.childCount === 0
    // which violates the schema. Insert an empty scene_heading placeholder.
    if (view.state.doc.childCount === 0) {
      const fillTr = view.state.tr.insert(0, screenplaySchema.node('scene_heading'));
      view.dispatch(fillTr);
    }

    // Remap scene_cards indices: drop the deleted one, shift later ones up
    if (documentStore.document) {
      documentStore.setActiveSceneCards(
        documentStore.activeSceneCards
          .filter((c) => c.scene_index !== sceneOrder)
          .map((c) => (c.scene_index > sceneOrder ? { ...c, scene_index: c.scene_index - 1 } : c))
      );
    }

    documentStore.setContent(view.state.doc.toJSON());
    documentStore.markDirty();
  }
</script>

<div class="scene-cards-view" style="--editor-font-ml: '{fontFamily}'">
  {#if documentStore.isSeries && cardLevel === 'episodes'}
    <!-- Episode level: top toolbar shows the series title + episode count.
         No "Group by location" here — that toggle is scene-scoped. -->
    <div class="cards-toolbar">
      <div class="toolbar-crumb">
        <span class="toolbar-eyebrow">Series</span>
        <span class="toolbar-title">{documentStore.document?.series?.title || 'Untitled Series'}</span>
      </div>
      <div class="toolbar-spacer"></div>
      <span class="toolbar-count">
        {(documentStore.document?.series?.episodes?.length ?? 0)}
        {(documentStore.document?.series?.episodes?.length ?? 0) === 1 ? 'episode' : 'episodes'}
      </span>
    </div>
  {:else if documentStore.isSeries && cardLevel === 'scenes'}
    <!-- Drilled-in: breadcrumb back to episodes + the active episode title. -->
    <div class="cards-toolbar with-breadcrumb">
      <button class="crumb-back" type="button" onclick={backToEpisodes} title="Back to episodes">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M15 18 L9 12 L15 6"/>
        </svg>
        <span>Episodes</span>
      </button>
      <span class="crumb-sep" aria-hidden="true">/</span>
      <div class="toolbar-crumb">
        <span class="toolbar-eyebrow">Episode {documentStore.activeEpisode?.number ?? ''}</span>
        <span class="toolbar-title">{documentStore.activeEpisode?.title?.trim() || 'Untitled'}</span>
      </div>
      <div class="toolbar-spacer"></div>
      <span class="toolbar-count">{cards.length} {cards.length === 1 ? 'scene' : 'scenes'}</span>
      <label class="toolbar-toggle" title="Cluster cards by their Location group, then by Shoot date">
        <input type="checkbox" bind:checked={groupByLocation} />
        <span>Group by location</span>
      </label>
    </div>
  {:else}
    <!-- Film project — no series, no breadcrumb, original toolbar shape. -->
    <div class="cards-toolbar">
      <span class="toolbar-count">{cards.length} {cards.length === 1 ? 'scene' : 'scenes'}</span>
      <div class="toolbar-spacer"></div>
      <label class="toolbar-toggle" title="Cluster cards by their Location group, then by Shoot date">
        <input type="checkbox" bind:checked={groupByLocation} />
        <span>Group by location</span>
      </label>
    </div>
  {/if}

  {#if documentStore.isSeries && cardLevel === 'episodes'}
    <!-- Episode-level grid (#134). The inner component handles its own
         drag-reorder, rename, delete, and add. Drilling is via callback. -->
    <div class="episodes-pane">
      <EpisodeCardsView onOpenEpisode={openEpisode} />
    </div>
  {:else}
  <div class="cards-grid" bind:this={gridEl}>
    {#each displayCards as card (card.key)}
      <div
        class="card scene-card"
        class:dragging={dragFromScene === card.sceneNumber}
        class:drop-target={dropTargetScene === card.sceneNumber}
        animate:flip={{ duration: 450, easing: cubicInOut }}
      >
        <div class="card-header">
          <!-- Scene number badge is the drag handle -->
          <span
            class="card-number"
            class:disabled={groupByLocation}
            onmousedown={(e: MouseEvent) => { if (!groupByLocation) startDrag(e, card.sceneNumber); }}
            role="button"
            tabindex="-1"
            aria-label={groupByLocation
              ? `Scene ${card.sceneNumber} (drag disabled while grouped)`
              : `Drag to reorder scene ${card.sceneNumber}`}
            title={groupByLocation ? 'Switch off "Group by location" to drag-reorder' : ''}
          >{String(card.sceneNumber).padStart(2, '0')}</span>
          <span class="card-heading">{card.heading.toUpperCase()}</span>
          <button
            class="card-delete"
            type="button"
            onclick={() => deleteScene(card.sceneNumber, card.sceneOrder)}
            aria-label="Delete scene {card.sceneNumber}"
            title="Delete scene"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="3 6 5 6 21 6"></polyline>
              <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"></path>
              <path d="M10 11v6"></path>
              <path d="M14 11v6"></path>
              <path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"></path>
            </svg>
          </button>
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
          <label class="field-label" for="extras-{card.sceneNumber}">Non-speaking characters</label>
          <input
            id="extras-{card.sceneNumber}"
            class="card-input"
            type="text"
            placeholder="Comma-separated, e.g. Extras, Guard"
            value={card.extraCharacters}
            oninput={(e) => updateExtraCharacters(card.sceneOrder, (e.target as HTMLInputElement).value)}
            onkeydown={handleKeydown}
          />
          <!-- Shoot scheduling fields (#124) — two thin inputs on one row to
               keep the card compact. Empty = unscheduled / ungrouped. -->
          <div class="schedule-row">
            <div class="schedule-field">
              <label class="field-label" for="locgroup-{card.sceneNumber}">Location group</label>
              <input
                id="locgroup-{card.sceneNumber}"
                class="card-input"
                type="text"
                placeholder="e.g. Main House, Studio Lot"
                value={card.locationGroup}
                oninput={(e) => updateLocationGroup(card.sceneOrder, (e.target as HTMLInputElement).value)}
                onkeydown={handleKeydown}
              />
            </div>
            <div class="schedule-field">
              <label class="field-label" for="schedule-{card.sceneNumber}">Shoot date</label>
              <input
                id="schedule-{card.sceneNumber}"
                class="card-input"
                type="text"
                placeholder="ISO date or 'Day 3'"
                value={card.scheduledDate}
                oninput={(e) => updateScheduledDate(card.sceneOrder, (e.target as HTMLInputElement).value)}
                onkeydown={handleKeydown}
              />
            </div>
          </div>
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

  <!-- Floating drag preview. A fixed-position container that follows the
       cursor while a card is being dragged. Children are injected via DOM
       cloneNode so the ghost exactly matches the source card. -->
  <div
    class="drag-ghost"
    class:visible={ghostVisible}
    bind:this={ghostEl}
    style="width: {ghostW}px; height: {ghostH}px; transform: translate({ghostX}px, {ghostY}px);"
    aria-hidden="true"
  ></div>
  {/if}
</div>

<style>
  .scene-cards-view {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    background: var(--surface-base);
    padding: 24px;
  }

  /* Toolbar above the grid — scene count on the left, "Group by location"
     toggle on the right (#124). Sticky-ish lightweight strip rather than
     a full sub-bar; the page already has its own status bar. */
  .cards-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 14px;
    padding: 0 4px;
    font-family: var(--ui-font);
    font-size: 11.5px;
    color: var(--text-muted);
  }

  .toolbar-count {
    font-variant-numeric: tabular-nums;
  }

  .toolbar-spacer {
    flex: 1;
  }

  .toolbar-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    color: var(--text-secondary);
    user-select: none;
  }

  .toolbar-toggle input[type='checkbox'] {
    accent-color: var(--accent);
    cursor: pointer;
  }

  /* ─── Breadcrumb / drilled-in toolbar (#134) ─── */
  .cards-toolbar.with-breadcrumb {
    margin-bottom: 18px;
  }

  .crumb-back {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px 4px 6px;
    border-radius: 6px;
    border: 1px solid var(--border-subtle);
    background: var(--surface-base);
    color: var(--text-secondary);
    font-family: var(--ui-font);
    font-size: 11.5px;
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease,
                border-color var(--motion-fast, 100ms) ease;
  }

  .crumb-back:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
    border-color: var(--border-medium);
  }

  .crumb-sep {
    color: var(--text-muted);
    font-size: 12px;
    opacity: 0.5;
  }

  .toolbar-crumb {
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }

  .toolbar-eyebrow {
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .toolbar-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    max-width: 280px;
  }

  /* Episode-cards pane just provides the same scrollable container as the
     scene grid; the inner EpisodeCardsView owns its own grid layout. */
  .episodes-pane {
    flex: 1;
    min-height: 0;
  }

  /* Drag handle becomes visually disabled while Group by location is on,
     since drag-reorder would mutate document order behind a sorted view. */
  .card-number.disabled {
    cursor: default;
    opacity: 0.6;
  }

  /* Two-up row holding the new scheduling fields (#124). */
  .schedule-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .schedule-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    grid-auto-rows: minmax(280px, auto);
    gap: 16px;
    align-items: stretch;
  }

  .card {
    background: var(--surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    overflow: hidden;
    transition: opacity 120ms ease, border-color 120ms ease;
    display: flex;
    flex-direction: column;
    min-height: 280px;
  }

  .card.dragging {
    opacity: 0.25;
    /* Mask the interior so the empty slot hints where the card came from
       without competing visually with the floating ghost. */
    filter: grayscale(0.4);
  }

  .card.drop-target {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
    transform: scale(0.985);
    transition: transform 120ms ease, border-color 120ms ease, box-shadow 120ms ease;
  }

  /* The floating ghost: a fixed-position overlay positioned at the cursor.
     Hidden by default so its DOM stays in the tree for bind:this to work. */
  .drag-ghost {
    position: fixed;
    top: 0;
    left: 0;
    pointer-events: none;
    z-index: 1000;
    display: none;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 16px 32px var(--shadow-heavy, rgba(0, 0, 0, 0.4));
    opacity: 0.96;
    /* Slight tilt gives the card a physical "picked up" feel. Keep the
       transform composable with the JS-driven translate by applying the
       rotation here via a nested wrapper — the outer translate lives on the
       style attribute so JS updates override cleanly. */
    will-change: transform;
  }

  .drag-ghost.visible {
    display: block;
  }

  /* Nested card inside the ghost inherits the card look verbatim (it's a
     clone), but we disable any hover opacity changes from the delete button. */
  .drag-ghost :global(.card-delete) {
    opacity: 0 !important;
  }

  .card-header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  /* Scene number badge — same zero-padded Courier chip the
     EpisodeCardsView uses (#161). Reads as both an identifier and a
     draggable affordance, not as plain typography in the heading row. */
  .card-number {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 30px;
    height: 24px;
    padding: 0 8px;
    border-radius: 5px;
    background: var(--accent-muted);
    color: var(--accent);
    font-family: var(--editor-font-en), var(--ui-font);
    font-size: 12px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.04em;
    cursor: grab;
    user-select: none;
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .card-number:hover {
    background: var(--accent);
    color: var(--text-on-accent);
  }

  .card-number:active {
    cursor: grabbing;
  }

  /* Scene heading on a card — Courier Prime bold, tracking matches the
     editor's scene-heading style so cards and pages read as the same system. */
  .card-heading {
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .card-delete {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: 4px;
    cursor: pointer;
    flex-shrink: 0;
    opacity: 0.4;
    transition: background 120ms ease, color 120ms ease, opacity 120ms ease;
  }

  .card:hover .card-delete,
  .card-delete:focus-visible {
    opacity: 1;
  }

  .card-delete:hover {
    background: var(--accent-muted);
    color: var(--accent);
    opacity: 1;
  }

  .card-delete:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
    opacity: 1;
  }

  .card-meta {
    padding: 6px 16px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .meta-item {
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
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

  /* Shared tokens — keep in lock-step with .ann-label in Editor.svelte */
  .field-label {
    display: block;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: var(--label-font-size);
    font-weight: var(--label-font-weight);
    color: var(--label-color);
    text-transform: uppercase;
    letter-spacing: var(--label-tracking);
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
    font-family: var(--editor-font-en), var(--editor-font-ml), var(--ui-font);
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

  /* Single-line input for non-speaking characters — shares surface + border
     treatment with the textarea so the fields read as the same type. */
  .card-input {
    width: 100%;
    padding: 6px 8px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-primary);
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    font-family: var(--editor-font-en), var(--editor-font-ml), var(--ui-font);
    box-sizing: border-box;
    transition: border-color 120ms ease;
  }

  .card-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .card-input::placeholder {
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
