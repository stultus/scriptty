<script lang="ts">
  import { untrack } from 'svelte';
  import { Fragment, type Node as PMNode } from 'prosemirror-model';
  import { TextSelection } from 'prosemirror-state';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';
  import { screenplaySchema } from '$lib/editor/schema';

  // For series projects in IDE-explorer mode (#156), one SceneNavigator
  // can be rendered per *expanded* episode — including episodes that
  // aren't currently active. Pass the episode's index via this prop and
  // the navigator reads its content directly. Omit the prop and it
  // behaves as before, reading the active episode (or top-level film
  // content). Add-scene and drag-reorder are disabled for non-active
  // episodes (they need editorStore.view); click-to-jump still works
  // and triggers an episode switch.
  let { episodeIndex }: { episodeIndex?: number } = $props();

  let isActiveEpisode = $derived(
    episodeIndex === undefined ||
      episodeIndex === documentStore.activeEpisodeIndex,
  );

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

  /** Strip the leading INT./EXT./INT./EXT. prefix from a heading. The
   *  navigator's setting-tag glyph already encodes that information,
   *  so duplicating it in the text just eats space the location and
   *  time-of-day need to differentiate similar scenes. (#148) */
  function stripSettingPrefix(heading: string): string {
    return heading.replace(/^\s*(INT\.?\/EXT\.?|I\/E|INT\.?|EXT\.?)\s+/i, '');
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

  // ─── Keyboard navigation (#139) ─────────────────────────────────────
  // Roving tabindex over the scene items: only one .scene-item button is
  // Tab-reachable at a time. ArrowUp/Down move focus by one row,
  // Home/End jump to first/last. Enter activates (the button's own
  // click handler).
  //
  // `focusedSceneIdx === -1` means the writer hasn't started keyboarding
  // yet — we fall back to the editor's currentSceneIndex (or 0) so Tab
  // into the list lands on the active scene by default.
  let focusedSceneIdx = $state(-1);

  let effectiveFocused = $derived(
    focusedSceneIdx >= 0
      ? focusedSceneIdx
      : editorStore.currentSceneIndex >= 0
        ? editorStore.currentSceneIndex
        : 0,
  );

  function handleListKey(event: KeyboardEvent) {
    const total = scenes.length;
    if (total === 0) return;
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        focusedSceneIdx = Math.min(total - 1, effectiveFocused + 1);
        break;
      case 'ArrowUp':
        event.preventDefault();
        focusedSceneIdx = Math.max(0, effectiveFocused - 1);
        break;
      case 'Home':
        event.preventDefault();
        focusedSceneIdx = 0;
        break;
      case 'End':
        event.preventDefault();
        focusedSceneIdx = total - 1;
        break;
    }
  }

  // After focusedSceneIdx changes, move DOM focus to the corresponding
  // .scene-item button — but only if focus is already inside the list
  // (so we don't steal focus from elsewhere when the editor's active
  // scene changes).
  $effect(() => {
    if (focusedSceneIdx < 0) return;
    const active = document.activeElement;
    const insideList = !!(active && listEl?.contains(active));
    if (!insideList) return;
    const items = listEl?.querySelectorAll<HTMLButtonElement>('.scene-item');
    items?.[focusedSceneIdx]?.focus();
  });

  // Reset focusedSceneIdx if the scenes list shrinks past the focused
  // index (e.g. scene deleted in the editor).
  $effect(() => {
    if (focusedSceneIdx >= scenes.length) {
      focusedSceneIdx = scenes.length > 0 ? scenes.length - 1 : -1;
    }
  });

  // Extract scene headings from the ProseMirror JSON document.
  // Reads the *active* content — top-level for films, active episode for series —
  // so this component works for both project shapes without branching.
  //
  // Perf: We track ONLY `contentVersionDebounced` (#98). The actual content
  // is read inside `untrack()` so we don't subscribe to per-keystroke
  // mutations of `activeContent` — the navigator catches up ~200ms after the
  // last edit, which is invisible to the user but skips ~90% of the
  // recompute storm during active typing on a long screenplay.
  let scenes = $derived.by(() => {
    documentStore.contentVersionDebounced;
    return untrack(() => computeScenes());
  });

  function computeScenes(): SceneEntry[] {
    const doc = documentStore.document;
    if (!doc) return [];

    // When `episodeIndex` is set we're rendering a non-active episode's
    // scene list; pull content + scene_number_start from that episode
    // directly. Falls back to the active accessors otherwise.
    let rawContent: unknown;
    let startNum: number;
    if (episodeIndex !== undefined) {
      const ep = doc.series?.episodes?.[episodeIndex];
      rawContent = ep?.content ?? null;
      startNum = ep?.settings?.scene_number_start ?? 1;
    } else {
      rawContent = documentStore.activeContent;
      startNum = documentStore.activeSettings?.scene_number_start ?? 1;
    }
    if (!rawContent) return [];

    const content = rawContent as { type?: string; content?: Array<{ type?: string; content?: Array<{ text?: string }> }> };
    if (!content.content) return [];

    const entries: SceneEntry[] = [];
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
  }

  // Add a blank scene heading at the end of the document and focus it.
  // Mirrors the Cards view's "Add Scene" behavior so both entry points
  // stay in sync. Only the active episode's navigator can add scenes —
  // editorStore.view points at the active episode's content. (#156)
  function addScene() {
    if (!isActiveEpisode) return;
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

  // Navigate to a scene heading in the editor. For non-active episodes,
  // switch active first so the editor renders the right content, then
  // poll for the new view and apply the scroll. (#156)
  function scrollToScene(sceneIndex: number) {
    if (episodeIndex !== undefined && episodeIndex !== documentStore.activeEpisodeIndex) {
      documentStore.setActiveEpisode(episodeIndex);
      // Wait for the editor to remount on the new episode's content.
      // rAF loop with a short ceiling — usually resolves within 1–2
      // frames; if anything's wrong we don't want to spin forever.
      const start = performance.now();
      const tick = () => {
        if (editorStore.view && documentStore.activeEpisodeIndex === episodeIndex) {
          doScrollToScene(sceneIndex);
          return;
        }
        if (performance.now() - start > 1000) return;
        requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
      return;
    }
    doScrollToScene(sceneIndex);
  }

  function doScrollToScene(sceneIndex: number) {
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
  <!-- Sticky header strip — gives the rail a clear identity ("Scenes")
       plus a count and a quick add-scene action. Mirrors the eyebrow
       style of the Stats / Export modals so the chrome reads as one
       system across the app. -->
  <header class="nav-header">
    <div class="nav-header-text">
      <span class="nav-eyebrow">Scenes</span>
      <span class="nav-count">{scenes.length}</span>
    </div>
    {#if scenes.length > 0 && isActiveEpisode}
      <button class="nav-add" onclick={addScene} title="Add a new scene at the end" aria-label="Add scene">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="12" y1="5" x2="12" y2="19"/>
          <line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
      </button>
    {/if}
  </header>

  {#if scenes.length === 0}
    <!-- Empty state — composed, single glyph, friendly aphorism. -->
    <div class="empty-state">
      <div class="empty-glyph" aria-hidden="true">
        <svg width="44" height="44" viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
          <rect x="10" y="6" width="28" height="36" rx="2"/>
          <line x1="14" y1="14" x2="34" y2="14"/>
          <line x1="14" y1="20" x2="30" y2="20"/>
          <line x1="14" y1="26" x2="34" y2="26"/>
          <line x1="14" y1="32" x2="26" y2="32"/>
        </svg>
      </div>
      <p class="empty-title">No scenes yet</p>
      <p class="empty-hint">Start with a scene heading like <em>INT. ROOM — DAY</em>.</p>
      {#if isActiveEpisode}
        <button class="empty-cta" onclick={addScene}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
          Add scene
        </button>
      {/if}
    </div>
  {:else}
    <ul
      class="scene-list"
      bind:this={listEl}
      onkeydown={handleListKey}
      role="listbox"
      aria-label="Scene navigator"
    >
      {#each scenes as scene, sceneArrayIdx (scene.index)}
        {@const sceneOrder = scene.number - (episodeIndex !== undefined
          ? (documentStore.document?.series?.episodes?.[episodeIndex]?.settings?.scene_number_start ?? 1)
          : (documentStore.activeSettings?.scene_number_start ?? 1))}
        {@const isActive = isActiveEpisode && sceneOrder === editorStore.currentSceneIndex}
        {@const isFocusable = sceneArrayIdx === effectiveFocused}
        <li
          class="scene-li"
          class:active={isActive}
          class:drop-above={dropTargetScene === scene.number && dragFromScene !== null && dragFromScene > scene.number}
          class:drop-below={dropTargetScene === scene.number && dragFromScene !== null && dragFromScene < scene.number}
          class:dragging={dragFromScene === scene.number}
        >
          {#if isActiveEpisode}
            <!-- Drag handle only on the active episode — drag-reorder
                 needs editorStore.view, which only points at the active
                 episode's content. Cross-episode reordering is a
                 follow-up. (#156) -->
            <span
              class="drag-handle"
              onmousedown={(e: MouseEvent) => startDrag(e, scene.number)}
              role="button"
              tabindex="-1"
              aria-label="Drag to reorder scene {scene.number}"
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
          {:else}
            <span class="drag-handle drag-spacer" aria-hidden="true"></span>
          {/if}
          <button
            class="scene-item"
            data-time={scene.time ?? ''}
            data-setting={scene.setting ?? ''}
            role="option"
            tabindex={isFocusable ? 0 : -1}
            aria-selected={isActive}
            aria-posinset={sceneArrayIdx + 1}
            aria-setsize={scenes.length}
            onclick={() => scrollToScene(scene.index)}
            onfocus={() => { focusedSceneIdx = sceneArrayIdx; }}
            title="Scene {scene.number} · {scene.text.toUpperCase()} · ~{scene.pages} pages"
          >
            <!-- Scene number is the row's identifying chip — tabular,
                 zero-padded, accent-tinted on the active row. The
                 INT/EXT typographic tag is gone (the time stripe on
                 the left edge already signals setting via color). -->
            <span class="scene-number">{String(scene.number).padStart(2, '0')}</span>
            <!-- Title gets maximum real estate. -->
            <span class="scene-text">{stripSettingPrefix(scene.text).toUpperCase() || '(empty)'}</span>
            <!-- Time-of-day glyph — sun for daytime, moon for night.
                 Replaces the page-size pill, which wasn't useful at
                 the navigator's narrow width. -->
            {#if scene.time === 'DAY' || scene.time === 'MORNING' || scene.time === 'AFTERNOON' || scene.time === 'DAWN'}
              <svg class="time-glyph time-day" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-label="Day">
                <circle cx="12" cy="12" r="4"/>
                <path d="M12 2 V4 M12 20 V22 M2 12 H4 M20 12 H22 M4.9 4.9 L6.3 6.3 M17.7 17.7 L19.1 19.1 M4.9 19.1 L6.3 17.7 M17.7 6.3 L19.1 4.9"/>
              </svg>
            {:else if scene.time === 'NIGHT' || scene.time === 'DUSK' || scene.time === 'EVENING'}
              <svg class="time-glyph time-night" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-label="Night">
                <path d="M21 12.8 A9 9 0 1 1 11.2 3 a7 7 0 0 0 9.8 9.8 z"/>
              </svg>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  /* The navigator reads as a "writer's index" — borrows the editor's
     Courier Prime so each scene heading feels like an entry on a
     printed table-of-contents page, while the chrome stays in the
     UI font for clear hierarchy. */
  .navigator-content {
    overflow-y: auto;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--surface-base);
  }

  /* ─── Sticky header strip ─── */
  .nav-header {
    position: sticky;
    top: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 14px 10px;
    background: var(--surface-base);
    border-bottom: 1px solid var(--border-subtle);
  }

  .nav-header-text {
    flex: 1;
    display: inline-flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }

  /* Marker eyebrow — Courier in marker color, matches the dept
     vocabulary across the rest of the app (Settings sections,
     Stats rail, About credits). */
  .nav-eyebrow {
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--marker-color);
  }

  .nav-count {
    font-size: 10.5px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    padding: 1px 7px;
    border-radius: 9px;
    background: var(--surface-elevated);
    letter-spacing: 0.02em;
  }

  .nav-add {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .nav-add:hover {
    background: var(--accent-muted);
    color: var(--accent);
  }

  /* ─── Empty state ─── */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 10px;
    padding: 32px 18px;
    font-family: var(--ui-font);
  }

  .empty-glyph {
    color: var(--border-medium);
    margin-bottom: 4px;
  }

  .empty-title {
    margin: 0;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .empty-hint {
    margin: 0;
    font-size: 11px;
    line-height: 1.45;
    color: var(--text-muted);
    max-width: 200px;
  }

  .empty-hint em {
    font-style: normal;
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 10.5px;
    color: var(--text-secondary);
    background: var(--surface-float);
    border: 1px solid var(--border-subtle);
    border-radius: 3px;
    padding: 0 4px;
  }

  .empty-cta {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-top: 6px;
    padding: 6px 12px;
    font-family: var(--ui-font);
    font-size: 11.5px;
    font-weight: 500;
    color: var(--accent);
    background: var(--accent-muted);
    border: 1px solid transparent;
    border-radius: 6px;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .empty-cta:hover {
    background: var(--accent);
    color: var(--text-on-accent);
  }

  /* ─── Scene list ─── */
  .scene-list {
    list-style: none;
    margin: 0;
    padding: 6px 6px 12px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .scene-li {
    position: relative;
    display: flex;
    align-items: stretch;
  }

  .scene-li.dragging {
    opacity: 0.45;
  }

  /* Drop indicator — teal line above or below the target item. */
  .scene-li.drop-above::before {
    content: '';
    position: absolute;
    top: -1px;
    left: 22px;
    right: 6px;
    height: 2px;
    background: var(--accent);
    border-radius: 1px;
    z-index: 1;
  }

  .scene-li.drop-below::after {
    content: '';
    position: absolute;
    bottom: -1px;
    left: 22px;
    right: 6px;
    height: 2px;
    background: var(--accent);
    border-radius: 1px;
    z-index: 1;
  }

  .drag-handle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    flex-shrink: 0;
    color: var(--text-muted);
    cursor: grab;
    opacity: 0;
    transition: opacity var(--motion-fast, 100ms) ease;
    user-select: none;
  }

  .drag-handle:active {
    cursor: grabbing;
  }

  /* Non-active episodes (#156) — keep the column reserved so scenes
     align across all expanded episodes, but no grab affordance. */
  .drag-handle.drag-spacer {
    cursor: default;
    pointer-events: none;
  }

  .scene-li:hover .drag-handle,
  .scene-li.active .drag-handle {
    opacity: 0.6;
  }

  .drag-handle:hover {
    opacity: 1 !important;
    color: var(--text-secondary);
  }

  /* ─── Scene item ─── */
  /* Card-like row — the soft surface and rounded corners signal that
     each scene is a discrete unit the writer can act on. The left edge
     carries the time-of-day stripe; the active row has an accent left
     bar and accent-muted bg. */
  .scene-item {
    position: relative;
    display: grid;
    grid-template-columns: 28px 1fr 16px;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    min-height: 38px;
    padding: 7px 10px 7px 12px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
    font-size: 11.5px;
    text-align: left;
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .scene-item:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  /* Keyboard focus ring — visible against the active accent bg too. */
  .scene-item:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 2px var(--accent);
  }

  /* Active scene — the one the editor cursor is currently inside. Stays
     prominent so the writer always knows where they are in the outline. */
  .scene-li.active .scene-item {
    background: var(--accent-muted);
    color: var(--text-primary);
    box-shadow: inset 2px 0 0 var(--accent);
  }

  /* Scene number chip — zero-padded Courier badge, mirrors the
     EpisodeCardsView number badge so cards and rail speak the same
     identifier system. Active row's chip picks up the accent fill. */
  .scene-number {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 26px;
    height: 18px;
    padding: 0 5px;
    border-radius: 4px;
    background: var(--surface-elevated);
    color: var(--text-secondary);
    font-family: var(--editor-font-en), var(--ui-font);
    font-size: 10px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.04em;
    line-height: 1;
  }

  .scene-li.active .scene-number {
    background: var(--accent);
    color: var(--text-on-accent);
  }

  .scene-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    letter-spacing: 0.02em;
    line-height: 1.35;
  }

  /* Time-of-day glyph — sun for daytime, moon for night. The shape
     carries the primary signal; the warm/cool tint on the icon is a
     subtle reinforcement (sun in lamp-amber, moon in oxblood-deep)
     that doesn't compete with the rest of the row. The previous
     full-height colored stripe at the left edge was confusing as a
     parallel channel; tinting only the glyph keeps the signal
     localized to the icon itself. */
  .time-glyph {
    flex-shrink: 0;
    color: var(--text-muted);
    transition: color var(--motion-fast, 100ms) ease;
  }

  .time-glyph.time-day {
    color: var(--accent-warm);
  }

  .time-glyph.time-night {
    color: var(--accent-deep);
  }

  .scene-li.active .time-glyph {
    color: var(--accent);
  }

</style>
