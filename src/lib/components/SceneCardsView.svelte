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
  import DatePicker from '$lib/components/DatePicker.svelte';

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
    // Single-episode series — skip the Episode level entirely (#140).
    // Drilling-into nothing is meaningless until the writer adds a
    // second episode; show scenes directly. The level switcher in the
    // hero header still works, so the writer can flip back to Episodes
    // and add a new one when they're ready.
    const epCount = documentStore.document?.series?.episodes?.length ?? 0;
    if (epCount <= 1) return 'scenes';
    if (typeof localStorage === 'undefined') return 'episodes';
    const stored = localStorage.getItem(storageKey());
    return stored === 'scenes' ? 'scenes' : 'episodes';
  }

  let cardLevel = $state<CardLevel>(initialLevel());

  // ─── Compact mode for the Scene grid (#159) ─────────────────────────
  // Mirror of EpisodeCardsView's compact toggle: collapses each scene
  // card to a single row (number · setting · heading · cast · pages).
  // Persisted to localStorage so the writer's choice sticks.
  const SCENE_COMPACT_KEY = 'scriptty-scenes-compact';
  let sceneCompact = $state(false);
  if (typeof localStorage !== 'undefined') {
    sceneCompact = localStorage.getItem(SCENE_COMPACT_KEY) === '1';
  }
  $effect(() => {
    if (typeof localStorage === 'undefined') return;
    localStorage.setItem(SCENE_COMPACT_KEY, sceneCompact ? '1' : '0');
  });

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

  // Hero context flags + count, derived once so the template stays
  // declarative.
  let heroIsEpisodesLevel = $derived(documentStore.isSeries && cardLevel === 'episodes');
  let heroIsSeriesScenes = $derived(documentStore.isSeries && cardLevel === 'scenes');

  /** Series-wide totals for the Episode-level hero (#140). Producers
   *  scanning the series want scope at a glance — total scenes, total
   *  pages — alongside the episode count. Cheap derived: walks every
   *  episode's content once, counts scene_heading nodes and total text
   *  length. */
  let seriesTotals = $derived.by<{ scenes: number; pages: string } | null>(() => {
    if (!documentStore.isSeries) return null;
    documentStore.contentVersionDebounced; // re-derive on edits
    const eps = documentStore.document?.series?.episodes ?? [];
    let scenes = 0;
    let chars = 0;
    for (const ep of eps) {
      const content = ep.content as { content?: Array<{ type?: string; content?: Array<{ text?: string }> }> } | null;
      if (!content?.content) continue;
      for (const node of content.content) {
        if (node.type === 'scene_heading') scenes++;
        const inner = node.content ?? [];
        for (const c of inner) chars += (c.text ?? '').length;
      }
    }
    const pages = Math.max(0.1, chars / 3000).toFixed(1);
    return { scenes, pages };
  });

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

  // Count + noun shown in the masthead right column. Reads from cards
  // when at scenes level, episodes when at episodes level.
  let heroCountValue = $derived(
    heroIsEpisodesLevel
      ? (documentStore.document?.series?.episodes?.length ?? 0)
      : cards.length,
  );
  let heroCountNoun = $derived(
    heroIsEpisodesLevel
      ? heroCountValue === 1 ? 'episode' : 'episodes'
      : heroCountValue === 1 ? 'scene' : 'scenes',
  );

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

  /** Duplicate a scene (#163). Clones the scene_heading + every
   *  following top-level node until the next scene_heading, inserts
   *  the clone immediately after the source, and copies the source
   *  scene_card's authored fields onto the new card. All other cards
   *  with scene_index > source shift up by one to track the new
   *  ordering. */
  function duplicateScene(sceneNumber: number, sceneOrder: number) {
    const view = editorStore.view;
    if (!view) return;
    const doc = view.state.doc;

    // Collect scene boundaries.
    const bounds: { childIndex: number; offset: number }[] = [];
    doc.forEach((node, offset, index) => {
      if (node.type.name === 'scene_heading') bounds.push({ childIndex: index, offset });
    });
    if (sceneNumber < 1 || sceneNumber > bounds.length) return;
    const idx = sceneNumber - 1;

    const childStart = bounds[idx].childIndex;
    const childEnd = idx + 1 < bounds.length ? bounds[idx + 1].childIndex : doc.childCount;
    const startPos = bounds[idx].offset;
    const endPos = idx + 1 < bounds.length ? bounds[idx + 1].offset : doc.content.size;

    // Collect and copy the source scene's nodes. PMNode.copy() preserves
    // the node's content and marks; that's what we want for a true
    // duplicate (same heading text, same body lines).
    const sceneNodes: PMNode[] = [];
    for (let i = childStart; i < childEnd; i++) {
      sceneNodes.push(doc.child(i));
    }
    const fragment = Fragment.from(sceneNodes);

    const tr = view.state.tr.insert(endPos, fragment);
    tr.scrollIntoView();
    view.dispatch(tr);
    documentStore.setContent(view.state.doc.toJSON());

    // scene_cards: shift indices > source up by 1 so the slots for the
    // new duplicate fit, then push a copy of the source card at
    // sourceIndex + 1.
    const cards = documentStore.activeSceneCards;
    const source = cards.find((c) => c.scene_index === sceneOrder);
    for (const c of cards) {
      if (c.scene_index > sceneOrder) c.scene_index += 1;
    }
    if (source) {
      cards.push({
        ...source,
        scene_index: sceneOrder + 1,
        // Reset shoot scheduling on the duplicate — same heading +
        // description make sense to clone, but two scenes shouldn't
        // share a shoot date by default (writer almost certainly wants
        // to re-schedule).
        scheduled_date: '',
      });
    }

    documentStore.markDirty();
  }

  // ─── Inline scene-heading edit ──────────────────────────────────────
  // Click the heading on a scene card to rename it without leaving the
  // grid. Commits on Enter / blur; Escape reverts. Writes the new text
  // back into the underlying scene_heading node so the editor view
  // sees the same value.
  let editingHeadingFor = $state<number | null>(null);
  let headingDraft = $state('');

  function beginHeadingEdit(sceneNumber: number, current: string) {
    editingHeadingFor = sceneNumber;
    headingDraft = current;
  }

  function cancelHeadingEdit() {
    editingHeadingFor = null;
    headingDraft = '';
  }

  function commitHeadingEdit(sceneNumber: number) {
    if (editingHeadingFor !== sceneNumber) return;
    const next = headingDraft.trim().toUpperCase();
    writeSceneHeading(sceneNumber, next);
    editingHeadingFor = null;
    headingDraft = '';
  }

  /** Replace the Nth scene_heading's text content with `newText`.
   *  Mirrors how the editor's autoUppercase plugin treats the heading
   *  — uppercase on commit. Empty string is allowed (writer wants to
   *  re-author it). */
  function writeSceneHeading(sceneNumber: number, newText: string) {
    const view = editorStore.view;
    if (!view) return;
    const doc = view.state.doc;
    let count = 0;
    let targetPos = -1;
    let targetSize = 0;
    doc.forEach((node, offset) => {
      if (node.type.name === 'scene_heading') {
        count++;
        if (count === sceneNumber) {
          targetPos = offset;
          targetSize = node.content.size;
        }
      }
    });
    if (targetPos < 0) return;
    const startPos = targetPos + 1; // inside the node
    const endPos = startPos + targetSize;
    const tr = view.state.tr;
    if (newText) {
      tr.replaceWith(startPos, endPos, view.state.schema.text(newText));
    } else {
      tr.delete(startPos, endPos);
    }
    view.dispatch(tr);
    documentStore.setContent(view.state.doc.toJSON());
    documentStore.markDirty();
  }

  function handleHeadingKeydown(event: KeyboardEvent, sceneNumber: number) {
    if (event.key === 'Enter') {
      event.preventDefault();
      commitHeadingEdit(sceneNumber);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      cancelHeadingEdit();
    }
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
  <!-- Hero header — makes the current level (Episodes vs Scenes)
       impossible to miss (#150). The big title states what level you're
       at; the subtitle gives context (series name or episode title).
       For series projects in the Scenes level, a breadcrumb above the
       title hands the writer back to Episodes — it's a drill-down
       hierarchy, not a parallel-view toggle. -->
  <!-- Hero — editorial masthead. Eyebrow context (FILM / SERIES / ←
       EPISODES), big title, Manjari italic project subtitle, then
       right cluster with the count as a big stat and the view
       toggles in a typeset toolbar row. Ornamented bottom rule. -->
  <header class="cards-hero">
    <div class="hero-left">
      <div class="hero-eyebrow">
        {#if heroIsSeriesScenes}
          <button
            type="button"
            class="hero-back"
            onclick={backToEpisodes}
            aria-label="Back to episodes"
          >
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M15 18 L9 12 L15 6"/></svg>
            <span>Episodes</span>
          </button>
          <span class="eyebrow-sep" aria-hidden="true">·</span>
          <span class="eyebrow-context">Episode {String(documentStore.activeEpisode?.number ?? 1).padStart(2, '0')}</span>
        {:else if heroIsEpisodesLevel}
          <span class="eyebrow-rule" aria-hidden="true"></span>
          <span class="eyebrow-context">Series</span>
          <span class="eyebrow-rule" aria-hidden="true"></span>
        {:else}
          <span class="eyebrow-rule" aria-hidden="true"></span>
          <span class="eyebrow-context">Film</span>
          <span class="eyebrow-rule" aria-hidden="true"></span>
        {/if}
      </div>

      <h1 class="hero-title">
        {#if heroIsEpisodesLevel}Episodes{:else}Scenes{/if}
      </h1>

      <p class="hero-subtitle">
        {#if heroIsEpisodesLevel}
          <span class="subtitle-name">{documentStore.document?.series?.title || 'Untitled Series'}</span>
          {#if seriesTotals}
            <span class="subtitle-meta">
              <span class="meta-sep" aria-hidden="true">·</span>
              <strong>{seriesTotals.scenes}</strong> {seriesTotals.scenes === 1 ? 'scene' : 'scenes'}
              <span class="meta-sep" aria-hidden="true">·</span>
              <strong>~{seriesTotals.pages}</strong> pages
            </span>
          {/if}
        {:else if heroIsSeriesScenes}
          <span class="subtitle-name">{documentStore.activeEpisode?.title?.trim() || 'Untitled episode'}</span>
        {:else}
          <span class="subtitle-name">{documentStore.activeMeta?.title?.trim() || 'Untitled screenplay'}</span>
        {/if}
      </p>
    </div>

    <div class="hero-right">
      <div class="hero-stat">
        <strong class="stat-value">{heroCountValue}</strong>
        <span class="stat-label">{heroCountNoun}</span>
      </div>

      {#if cardLevel === 'scenes'}
        <div class="hero-toolbar">
          <label class="toolbar-toggle" title="Compact view collapses each card to a single row — useful for at-a-glance episode planning">
            <input type="checkbox" bind:checked={sceneCompact} />
            <span>Compact</span>
          </label>
          <span class="toolbar-divider" aria-hidden="true"></span>
          <label class="toolbar-toggle" title="Cluster cards by their Location group, then by Shoot date">
            <input type="checkbox" bind:checked={groupByLocation} />
            <span>Group by location</span>
          </label>
        </div>
      {/if}
    </div>
  </header>

  {#if documentStore.isSeries && cardLevel === 'episodes'}
    <!-- Episode-level grid (#134). The inner component handles its own
         drag-reorder, rename, delete, and add. Drilling is via callback. -->
    <div class="episodes-pane">
      <EpisodeCardsView onOpenEpisode={openEpisode} />
    </div>
  {:else}
  <div class="cards-grid" class:compact={sceneCompact} bind:this={gridEl}>
    {#each displayCards as card (card.key)}
      {@const headingUpper = card.heading.toUpperCase()}
      {@const cardSetting = headingUpper.startsWith('INT./EXT.') || headingUpper.startsWith('INT/EXT')
        ? 'INT_EXT'
        : headingUpper.startsWith('INT.') || headingUpper.startsWith('INT ')
          ? 'INT'
          : headingUpper.startsWith('EXT.') || headingUpper.startsWith('EXT ')
            ? 'EXT'
            : ''}
      <!-- A card with no authored content (no description, no notes, no
           extras, no schedule) reads as a "skeleton" so the writer can
           tell at a glance which scenes are outlined vs untouched (#162).
           Adding any field makes it a real card. -->
      {@const isSkeleton = !card.description.trim()
        && !card.shootNotes.trim()
        && !card.extraCharacters.trim()
        && !card.scheduledDate.trim()
        && !card.locationGroup.trim()}
      {@const hasProduction = !!(card.extraCharacters.trim() || card.locationGroup.trim() || card.scheduledDate.trim())}
      {@const settingWord = cardSetting === 'INT' ? 'INTERIOR'
        : cardSetting === 'EXT' ? 'EXTERIOR'
        : cardSetting === 'INT_EXT' ? 'INT · EXT'
        : ''}
      {@const timeWord = (card.time ?? '').toUpperCase()}
      <div
        class="card scene-card"
        class:dragging={dragFromScene === card.sceneNumber}
        class:drop-target={dropTargetScene === card.sceneNumber}
        class:skeleton={isSkeleton}
        data-time={timeWord}
        animate:flip={{ duration: 450, easing: cubicInOut }}
      >
        <!-- Gutter — the dominant typographic anchor of the card. The
             zero-padded scene number reads as a chapter marker, set in
             the editor's Courier Prime so it visually rhymes with the
             scene-number gutter on the printed shooting script. The
             1px right rule is hand-set, not a border, so it stops at
             the footer hairline. -->
        <div
          class="card-gutter"
          class:disabled={groupByLocation}
          onmousedown={(e: MouseEvent) => { if (!groupByLocation) startDrag(e, card.sceneNumber); }}
          role="button"
          tabindex="-1"
          aria-label={groupByLocation
            ? `Scene ${card.sceneNumber} (drag disabled while grouped)`
            : `Drag to reorder scene ${card.sceneNumber}`}
          title={groupByLocation ? 'Switch off "Group by location" to drag-reorder' : 'Drag to reorder'}
        >
          <span class="gutter-number">{String(card.sceneNumber).padStart(2, '0')}</span>
        </div>

        <div class="card-body">
          <header class="card-header">
            <!-- Eyebrow: "INTERIOR · DAY" / "EXTERIOR · NIGHT". Replaces
                 the colored stripe and the I/E typographic chip — one
                 line of typeset metadata reads cleaner than two
                 ornaments at narrow column widths. -->
            <div class="card-eyebrow">
              {#if settingWord}<span class="eb-setting">{settingWord}</span>{/if}
              {#if settingWord && timeWord}<span class="eb-sep" aria-hidden="true">·</span>{/if}
              {#if timeWord}<span class="eb-time">{timeWord}</span>{/if}
              {#if !settingWord && !timeWord}<span class="eb-empty">No slug yet</span>{/if}
            </div>

            <div class="card-actions-cluster">
              <button
                class="card-delete"
                type="button"
                onclick={() => duplicateScene(card.sceneNumber, card.sceneOrder)}
                aria-label="Duplicate scene {card.sceneNumber}"
                title="Duplicate scene"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="9" y="9" width="11" height="11" rx="2"/>
                  <path d="M5 15 H4 A1 1 0 0 1 3 14 V4 A1 1 0 0 1 4 3 H14 A1 1 0 0 1 15 4 V5"/>
                </svg>
              </button>
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
          </header>

          <!-- Slug — the scene heading in big Courier. Click-to-rename. -->
          {#if editingHeadingFor === card.sceneNumber}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              class="card-heading-input"
              bind:value={headingDraft}
              onblur={() => commitHeadingEdit(card.sceneNumber)}
              onkeydown={(e) => handleHeadingKeydown(e, card.sceneNumber)}
              placeholder="INT. LOCATION — TIME"
              autofocus
            />
          {:else}
            <button
              class="card-heading"
              type="button"
              onclick={() => beginHeadingEdit(card.sceneNumber, card.heading)}
              title="Click to rename"
            >{headingUpper || 'Untitled scene'}</button>
          {/if}

          {#if card.characters.length > 0}
            <p class="card-cast"><span class="cast-mark" aria-hidden="true">w/</span> {card.characters.join(' · ')}</p>
          {/if}

          <div class="card-editable">
            <textarea
              class="card-textarea"
              placeholder="What happens in this scene…"
              aria-label="Scene description"
              value={card.description}
              oninput={(e) => updateDescription(card.sceneOrder, (e.target as HTMLTextAreaElement).value)}
              onkeydown={handleKeydown}
            ></textarea>

            <!-- Production-prep fields collapsed behind a disclosure
                 (#159). Restyled to match the typeset tear-sheet
                 aesthetic — small caps eyebrow, ruled hairline, italic
                 hint of the populated values. -->
            <details class="production-disclosure" open={hasProduction}>
              <summary class="production-summary">
                <svg class="caret" width="9" height="9" viewBox="0 0 10 10" fill="currentColor" aria-hidden="true">
                  <path d="M3 2 L7 5 L3 8 Z" />
                </svg>
                <span class="production-label">Production</span>
                {#if hasProduction}
                  <span class="production-hint">
                    {#if card.locationGroup.trim()}<span>{card.locationGroup}</span>{/if}
                    {#if card.scheduledDate.trim()}<span>· {card.scheduledDate}</span>{/if}
                    {#if card.extraCharacters.trim()}<span>· extras</span>{/if}
                  </span>
                {:else}
                  <span class="production-hint muted">cast extras · location group · shoot date</span>
                {/if}
              </summary>
              <div class="production-body">
                <div class="prod-field">
                  <label class="prod-label" for="extras-{card.sceneNumber}">Cast extras</label>
                  <input
                    id="extras-{card.sceneNumber}"
                    class="prod-input"
                    type="text"
                    placeholder="e.g. Extras, Guard"
                    value={card.extraCharacters}
                    oninput={(e) => updateExtraCharacters(card.sceneOrder, (e.target as HTMLInputElement).value)}
                    onkeydown={handleKeydown}
                  />
                </div>
                <div class="prod-row">
                  <div class="prod-field">
                    <label class="prod-label" for="locgroup-{card.sceneNumber}">Location group</label>
                    <input
                      id="locgroup-{card.sceneNumber}"
                      class="prod-input"
                      type="text"
                      placeholder="e.g. Main House"
                      value={card.locationGroup}
                      oninput={(e) => updateLocationGroup(card.sceneOrder, (e.target as HTMLInputElement).value)}
                      onkeydown={handleKeydown}
                    />
                  </div>
                  <div class="prod-field">
                    <span class="prod-label">Shoot date</span>
                    <DatePicker
                      value={card.scheduledDate}
                      onChange={(v: string) => updateScheduledDate(card.sceneOrder, v)}
                      placeholder="Pick a date"
                      compact={true}
                    />
                  </div>
                </div>
              </div>
            </details>

            <textarea
              class="card-textarea card-notes"
              placeholder="Notes for the floor…"
              aria-label="Scene notes"
              value={card.shootNotes}
              oninput={(e) => updateShootNotes(card.sceneOrder, (e.target as HTMLTextAreaElement).value)}
              onkeydown={handleKeydown}
            ></textarea>
          </div>

          <footer class="card-footer">
            <span class="page-estimate">{card.pageEstimate}</span>
          </footer>
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
  /* Cards canvas reads as the writer's planning desk — borrows the
     editor's --page-grain texture so the surface feels like the same
     paper environment, scaled to desk-size rather than page-size (#155).
     The grain is a CSS variable that adapts to the active theme so
     light and dark modes both get a subtle, theme-appropriate texture. */
  .scene-cards-view {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    background-color: var(--surface-base);
    background-image: var(--page-grain);
    background-repeat: repeat;
    background-size: 360px 360px;
    background-attachment: local;
    padding: 24px;
  }

  /* ─── Hero header (#150) ───
     The big "EPISODES" or "SCENES" title makes the current view level
     impossible to miss. Subtitle gives context (series name or active
     episode). The right-side cluster carries the level switcher,
     count, and per-level toggles. */
  /* ─── Hero — editorial masthead ─────────────────────────────────────
     Section title-page treatment, like a page from a typeset
     shooting script. Two columns: left holds eyebrow + title +
     subtitle; right holds a big stat number + view toolbar. The
     bottom border is broken with a small ornament so the divider
     reads as a typeset rule rather than a plain hairline. */
  .cards-hero {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: end;
    gap: 32px;
    margin-bottom: 28px;
    padding: 4px 4px 22px;
    font-family: var(--ui-font);
  }

  .cards-hero::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 1px;
    background: linear-gradient(
      to right,
      transparent 0,
      var(--border-subtle) 8%,
      var(--border-subtle) 48%,
      transparent 49.5%,
      transparent 50.5%,
      var(--border-subtle) 52%,
      var(--border-subtle) 92%,
      transparent 100%);
  }

  .cards-hero::before {
    content: '·';
    position: absolute;
    left: 50%;
    bottom: -7px;
    transform: translateX(-50%);
    width: 14px;
    height: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 14px;
    line-height: 1;
    background: var(--page-base, var(--surface-base));
  }

  .hero-left {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  /* Eyebrow row — context indicator above the title. Shows
     "— FILM —" / "— SERIES —" with hairline rules flanking the word
     so it reads as a typeset section header. For the scenes-level
     view inside a series, the rules are replaced by a back-button
     breadcrumb + episode badge. */
  .hero-eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    height: 14px;
    font-family: var(--ui-font);
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .eyebrow-context {
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .eyebrow-rule {
    display: inline-block;
    width: 28px;
    height: 1px;
    background: var(--border-medium);
  }

  .eyebrow-sep {
    color: var(--border-medium);
    font-weight: 400;
    letter-spacing: 0;
  }

  .hero-back {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 6px 2px 4px;
    margin-left: -4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    text-transform: inherit;
    letter-spacing: inherit;
    border-radius: 4px;
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .hero-back:hover {
    background: var(--surface-hover);
    color: var(--accent);
  }

  /* Title — confident, larger weight, slightly tighter letter
     spacing than the eyebrow. The dominant typographic mark on the
     page. */
  .hero-title {
    margin: 0;
    font-family: var(--ui-font);
    font-size: 30px;
    font-weight: 700;
    line-height: 1;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--text-primary);
  }

  /* Subtitle — Manjari italic for the project / episode title so
     the bilingual identity comes through as more than a caption.
     Series totals get a subdued numeric register beside it. */
  .hero-subtitle {
    margin: 0;
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
    font-family: 'Manjari', var(--ui-font);
    font-size: 14px;
    font-style: italic;
    line-height: 1.3;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 60vw;
    letter-spacing: 0.005em;
  }

  .subtitle-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .subtitle-meta {
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
    font-family: var(--ui-font);
    font-style: normal;
    font-size: 11px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
  }

  .subtitle-meta strong {
    color: var(--text-secondary);
    font-weight: 600;
  }

  .meta-sep {
    color: var(--border-medium);
  }

  /* Right column — stacked: big stat over toolbar. */
  .hero-right {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 10px;
    flex-shrink: 0;
  }

  /* The count rendered as a typeset stat: large Courier numeral, tiny
     tracked label below. Editorial sidebar number, not a chip. */
  .hero-stat {
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
    font-family: var(--editor-font-en), ui-monospace, monospace;
  }

  .stat-value {
    font-size: 22px;
    font-weight: 700;
    line-height: 1;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
  }

  .stat-label {
    font-family: var(--ui-font);
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  /* Toolbar — Compact + Group toggles in their own row, with a
     hairline divider between them so they read as discrete options
     in a typeset toolbar, not bunched into the count's whitespace. */
  .hero-toolbar {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    padding: 4px 0 0;
  }

  .toolbar-divider {
    width: 1px;
    height: 12px;
    background: var(--border-medium);
  }

  .toolbar-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    color: var(--text-secondary);
    user-select: none;
    font-size: 11px;
    letter-spacing: 0.04em;
    transition: color var(--motion-fast, 100ms) ease;
  }

  .toolbar-toggle:hover {
    color: var(--text-primary);
  }

  .toolbar-toggle input[type='checkbox'] {
    accent-color: var(--accent);
    cursor: pointer;
  }

  /* Episode-cards pane just provides the same scrollable container as the
     scene grid; the inner EpisodeCardsView owns its own grid layout. */
  .episodes-pane {
    flex: 1;
    min-height: 0;
  }



  .cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    grid-auto-rows: minmax(280px, auto);
    gap: 16px;
    align-items: stretch;
  }

  /* Compact mode (#159) — one card per row, single horizontal stripe.
     Hides the editable body (only the header strip + a thin meta row
     remain) so a long episode fits in one viewport for at-a-glance
     review. The whole row stays clickable to drill back into the
     full card by toggling Compact off. */
  .cards-grid.compact {
    grid-template-columns: 1fr;
    grid-auto-rows: auto;
    gap: 4px;
  }

  .cards-grid.compact .card {
    min-height: 0;
    grid-template-columns: 48px 1fr;
  }

  .cards-grid.compact .card-gutter {
    padding: 8px 0;
  }

  .cards-grid.compact .gutter-number {
    font-size: 18px;
  }

  .cards-grid.compact .card-body {
    padding: 8px 14px;
  }

  .cards-grid.compact .card-cast,
  .cards-grid.compact .card-editable {
    display: none;
  }

  .cards-grid.compact .card-heading {
    margin-bottom: 0;
  }

  .cards-grid.compact .card-footer {
    padding: 0;
    margin-top: 4px;
    border-top: none;
  }

  .cards-grid.compact .add-scene-card {
    min-height: 44px;
    flex-direction: row;
    gap: 8px;
    padding: 8px 14px;
  }

  /* ─── Scene card — typeset tear-sheet aesthetic ─────────────────────
     The card is structured like a slug-numbered tear-sheet from a
     printed shooting script: a dedicated left gutter holds a hero
     scene number set in Courier, separated from the body by a 1px
     rule. The body opens with a small-caps eyebrow ("INTERIOR · DAY")
     and the heading slug below it, flowing into the editable section.
     No filled INT/EXT chip, no full-height color stripe — those
     ornaments competed with the slug. */
  .card {
    position: relative;
    display: grid;
    grid-template-columns: 56px 1fr;
    align-items: stretch;
    background: var(--surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    overflow: hidden;
    min-height: 240px;
    transition: opacity 160ms ease,
                border-color 160ms ease,
                box-shadow 200ms ease,
                transform 200ms ease;
  }

  /* Hover lift via shadow only — NOT transform. A transform property
     creates a stacking context, which traps `position: fixed`
     descendants (the DatePicker popover uses fixed positioning to
     escape the card's overflow:hidden). With a transform here, the
     popover would compute viewport coords but render relative to the
     card — invisibly off-screen — even though the click + Enter still
     selected a date. */
  .card:hover {
    border-color: var(--border-medium);
    box-shadow: 0 6px 18px var(--shadow-soft, rgba(0, 0, 0, 0.05));
  }

  .card.dragging {
    opacity: 0.25;
    /* Mask the interior so the empty slot hints where the card came from
       without competing visually with the floating ghost. */
    filter: grayscale(0.4);
  }

  /* Skeleton cards (#162) — no authored content yet. The gutter
     number stays full-strength (the writer needs to see which slot
     they're filling); the rest dims to a dashed-border placeholder
     state so a scan of the grid distinguishes outlined from still-
     untouched scenes. Adding any field flips the card back. */
  .card.skeleton {
    background: transparent;
    border-style: dashed;
    border-color: var(--border-medium);
    box-shadow: none;
  }

  .card.skeleton:hover {
    box-shadow: 0 4px 14px var(--shadow-soft, rgba(0, 0, 0, 0.04));
  }

  .card.skeleton .card-textarea {
    background: transparent;
  }

  .card.skeleton .card-textarea::placeholder {
    font-style: italic;
  }

  .card.drop-target {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
    transform: scale(0.985);
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

  /* Gutter — the dominant typographic anchor. A 1px right rule reads
     as the page-margin gutter on a printed shooting script. Skeleton
     cards keep the full-strength gutter; only the body greys out. */
  .card-gutter {
    position: relative;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding: 18px 0 12px;
    border: none;
    background: transparent;
    cursor: grab;
    user-select: none;
    transition: background 160ms ease;
  }

  .card-gutter::after {
    content: '';
    position: absolute;
    top: 14px;
    bottom: 14px;
    right: 0;
    width: 1px;
    background: var(--border-subtle);
    pointer-events: none;
  }

  .card-gutter:active {
    cursor: grabbing;
  }

  .card:hover .card-gutter {
    background: linear-gradient(180deg,
      var(--surface-base) 0%,
      transparent 100%);
  }

  .card-gutter.disabled {
    cursor: default;
  }

  .card-gutter.disabled .gutter-number {
    opacity: 0.55;
  }

  /* The hero numeral — Courier Prime, tabular, set big enough to
     anchor the card and read as an editorial chapter mark. Time-of-day
     tints the digit warm or cool so the gutter still carries that
     signal even though the colored stripe is gone. */
  .gutter-number {
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 26px;
    font-weight: 700;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
    color: var(--text-secondary);
    transition: color 160ms ease;
  }

  .card[data-time='DAY'] .gutter-number,
  .card[data-time='MORNING'] .gutter-number,
  .card[data-time='AFTERNOON'] .gutter-number,
  .card[data-time='DAWN'] .gutter-number {
    color: var(--accent-warm, #b58136);
  }

  .card[data-time='NIGHT'] .gutter-number,
  .card[data-time='DUSK'] .gutter-number,
  .card[data-time='EVENING'] .gutter-number {
    color: var(--accent-deep, #2a4d5c);
  }

  .card.skeleton .gutter-number {
    color: var(--text-muted);
  }

  /* Body — every right-side region of the card. */
  .card-body {
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 14px 18px 0;
  }

  .card-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 4px;
  }

  /* Eyebrow — small caps "INTERIOR · DAY" line that replaces both the
     I/E typographic chip and the colored time stripe. Tracks
     generously so it reads as typeset metadata, not a label. */
  .card-eyebrow {
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
    flex: 1;
    min-width: 0;
    font-family: var(--ui-font);
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--text-muted);
    line-height: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .eb-setting {
    color: var(--text-secondary);
  }

  .eb-time {
    color: var(--text-muted);
  }

  .card[data-time='DAY'] .eb-time,
  .card[data-time='MORNING'] .eb-time,
  .card[data-time='AFTERNOON'] .eb-time,
  .card[data-time='DAWN'] .eb-time {
    color: var(--accent-warm, #b58136);
  }

  .card[data-time='NIGHT'] .eb-time,
  .card[data-time='DUSK'] .eb-time,
  .card[data-time='EVENING'] .eb-time {
    color: var(--accent-deep, #2a4d5c);
  }

  .eb-sep {
    color: var(--border-medium);
    font-weight: 400;
    letter-spacing: 0;
  }

  .eb-empty {
    font-style: italic;
    font-weight: 500;
    letter-spacing: 0.04em;
    text-transform: none;
    color: var(--text-muted);
    opacity: 0.7;
  }

  /* Slug — the scene heading. Courier, bold, set larger than before
     so it carries the card's identity. Click-to-rename keeps a
     hover-reveal of a soft surface as the editable affordance. */
  .card-heading {
    display: block;
    width: calc(100% + 8px);
    margin: 0 -4px 8px;
    padding: 4px 4px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
    font-size: 13px;
    font-weight: 700;
    line-height: 1.3;
    letter-spacing: 0.02em;
    color: var(--text-primary);
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: text;
    transition: background 120ms ease;
  }

  .card-heading:hover {
    background: var(--surface-hover);
  }

  .card.skeleton .card-heading {
    color: var(--text-muted);
    font-style: italic;
    font-weight: 500;
  }

  .card-heading-input {
    display: block;
    width: calc(100% + 8px);
    margin: 0 -4px 8px;
    padding: 4px;
    background: var(--surface-base);
    border: 1px solid var(--accent);
    border-radius: 4px;
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.02em;
    color: var(--text-primary);
    text-transform: uppercase;
    outline: none;
  }

  /* Cast line — italic, small, prefixed with a typeset "w/" mark. The
     mark tells the writer this is a cast list, not just floating
     names; "w/" is the call-sheet shorthand and reads instantly to
     anyone who's read a shooting schedule. */
  .card-cast {
    margin: 0 0 10px;
    font-family: var(--ui-font);
    font-size: 11px;
    line-height: 1.4;
    color: var(--text-secondary);
  }

  .cast-mark {
    display: inline-block;
    margin-right: 4px;
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 10px;
    font-weight: 700;
    color: var(--text-muted);
    letter-spacing: 0.05em;
  }

  /* Action cluster — duplicate + delete. Hidden at rest (opacity 0)
     so the eyebrow has room to breathe, surface on hover. The bigger
     rest-state typography deserves a quieter chrome. */
  .card-actions-cluster {
    display: inline-flex;
    align-items: center;
    gap: 1px;
    flex-shrink: 0;
    margin: -4px -6px 0 0;
    opacity: 0;
    transition: opacity 160ms ease;
  }

  .card:hover .card-actions-cluster,
  .card-actions-cluster:focus-within {
    opacity: 1;
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
    transition: background 120ms ease, color 120ms ease;
  }

  .card-delete:hover {
    background: var(--accent-muted);
    color: var(--accent);
  }

  .card-delete:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  /* Footer — page estimate as a small Courier note in the right
     corner with a hairline rule above. Reads as a marginal
     annotation, not a separate UI strip. */
  .card-footer {
    margin-top: auto;
    padding: 8px 0 12px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    justify-content: flex-end;
  }

  .page-estimate {
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    text-transform: uppercase;
  }

  .card-editable {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding-bottom: 10px;
  }

  /* Production disclosure (#159) — collapses extras / location group
     / shoot date behind a single tight summary row. Auto-opens when
     any production field has content so populated cards never hide
     the writer's data; collapsed by default for empty cards so the
     eyebrow wall is gone. */
  /* Production disclosure — typeset eyebrow + hairline rules above
     and below the row, so the disclosure reads as a section break in
     the body, not as a separate UI element. The italic hint of the
     populated values keeps the closed state informative. */
  .production-disclosure {
    margin: 6px 0 10px;
    border-top: 1px solid var(--border-subtle);
  }

  .production-disclosure[open] {
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 8px;
  }

  .production-summary {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 8px 0 6px;
    cursor: pointer;
    list-style: none;
    user-select: none;
  }

  .production-summary::-webkit-details-marker {
    display: none;
  }

  .production-summary .caret {
    color: var(--text-muted);
    transition: transform 160ms ease;
    flex-shrink: 0;
    transform: translateY(-1px);
  }

  .production-disclosure[open] .production-summary .caret {
    transform: rotate(90deg) translateX(-1px);
  }

  .production-label {
    font-family: var(--ui-font);
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .production-hint {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    flex: 1;
    font-family: var(--ui-font);
    font-size: 10.5px;
    font-style: italic;
    color: var(--text-muted);
    letter-spacing: 0.01em;
  }

  .production-hint.muted {
    opacity: 0.65;
  }

  .production-hint span + span {
    margin-left: 4px;
  }

  .production-body {
    padding: 6px 0 4px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  /* Production-scoped fields — distinct from the body notepad style.
     These are scheduling metadata (extras, location, date), so they
     read as compact form cells with tiny tracked labels above and
     bordered input boxes below. The labels are scoped to inside the
     disclosure, so they don't add to the top-level eyebrow wall. */
  .prod-field {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .prod-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    align-items: stretch;
  }

  .prod-label {
    font-family: var(--ui-font);
    font-size: 8.5px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .prod-input {
    width: 100%;
    height: 32px;
    padding: 0 9px;
    font-size: 11.5px;
    line-height: 1.3;
    color: var(--text-primary);
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 5px;
    font-family: var(--editor-font-en), var(--editor-font-ml), var(--ui-font);
    box-sizing: border-box;
    transition: border-color 120ms ease, background 120ms ease;
  }

  .prod-input:hover {
    border-color: var(--border-medium);
  }

  .prod-input:focus {
    outline: none;
    border-color: var(--accent);
    background: var(--surface-float);
  }

  .prod-input::placeholder {
    color: var(--text-muted);
    font-style: italic;
    opacity: 0.7;
  }


  /* Textareas + inputs: notepad treatment. No border at rest — a soft
     lower hairline reads as a ruled writing line. Hover lifts the
     hairline; focus surfaces the field with a subtle bg + accent
     underline. Far less rectangle-noise than the bordered version. */
  .card-textarea {
    width: 100%;
    padding: 6px 0;
    font-size: 12px;
    line-height: 1.55;
    color: var(--text-primary);
    background: transparent;
    border: none;
    border-bottom: 1px solid transparent;
    border-radius: 0;
    font-family: var(--editor-font-en), var(--editor-font-ml), var(--ui-font);
    resize: none;
    box-sizing: border-box;
    transition: border-color 160ms ease, background 160ms ease;
    flex: 1;
    min-height: 56px;
    overflow: auto;
    /* Tame the WebKit scrollbar so it reads as a hairline groove in
       the page rather than a separate scrollable-region affordance. */
    scrollbar-width: thin;
    scrollbar-color: var(--border-medium) transparent;
  }

  .card-textarea::-webkit-scrollbar {
    width: 4px;
  }

  .card-textarea::-webkit-scrollbar-track {
    background: transparent;
  }

  .card-textarea::-webkit-scrollbar-thumb {
    background: transparent;
    border-radius: 2px;
  }

  .card-textarea:hover::-webkit-scrollbar-thumb,
  .card-textarea:focus::-webkit-scrollbar-thumb {
    background: var(--border-medium);
  }

  .card-textarea.card-notes {
    flex: 0 0 auto;
    min-height: 36px;
    font-style: italic;
    color: var(--text-secondary);
    font-size: 11.5px;
  }

  .card-textarea:hover {
    border-bottom-color: var(--border-subtle);
  }

  .card-textarea:focus {
    outline: none;
    background: var(--surface-base);
    border-bottom-color: var(--accent);
    padding: 6px 8px;
    margin: 0 -8px;
    width: calc(100% + 16px);
    border-radius: 4px 4px 0 0;
  }

  .card-textarea::placeholder {
    color: var(--text-muted);
  }


  /* ─── Add Scene placeholder ─── */
  .add-scene-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 240px;
    border: 1px dashed var(--border-medium);
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    font-family: var(--ui-font);
    font-size: 12.5px;
    font-weight: 500;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition: border-color 200ms ease, color 200ms ease, background 200ms ease;
  }

  .add-scene-card:hover {
    border-color: var(--accent);
    border-style: solid;
    color: var(--accent);
    background: var(--accent-muted);
  }
</style>
