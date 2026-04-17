<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState, Plugin, PluginKey } from 'prosemirror-state';
  import { EditorView, Decoration, DecorationSet } from 'prosemirror-view';
  import { history } from 'prosemirror-history';
  import { baseKeymap } from 'prosemirror-commands';
  import { keymap } from 'prosemirror-keymap';
  import { Node as ProseMirrorNode } from 'prosemirror-model';
  import { screenplaySchema } from '$lib/editor/schema';
  import { screenplayKeymap } from '$lib/editor/keymap';
  import { autoUppercasePlugin } from '$lib/editor/autoUppercase';
  import { characterAutocompletePlugin, autocompleteKey } from '$lib/editor/characterAutocomplete';
  import { characterListPlugin, characterListKey } from '$lib/editor/characterList';
  import { findReplacePlugin } from '$lib/editor/findReplace';
  import FindReplaceBar from '$lib/components/FindReplaceBar.svelte';
  import FormatBubble from '$lib/components/FormatBubble.svelte';
  import { InputModeManager } from '$lib/editor/input/InputModeManager';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';

  let {
    findReplaceOpen = $bindable(false),
    findReplaceMode = $bindable<'find' | 'replace'>('find'),
    showAnnotations = true,
    isActive = true,
  } = $props<{
    findReplaceOpen: boolean;
    findReplaceMode: 'find' | 'replace';
    showAnnotations?: boolean;
    isActive?: boolean;
  }>();

  // Recalculate spacers when the editor becomes visible again
  // (e.g. switching from Cards/Story back to Writing view)
  $effect(() => {
    if (isActive && showAnnotations) {
      scheduleAnnotationUpdate();
    }
  });

  // Push the current Show-Characters setting + per-scene extras into the
  // characterList plugin state whenever either changes. Reading both here
  // creates the reactive dependency; we also re-measure annotations since
  // the widget line changes the editor's vertical layout.
  $effect(() => {
    const enabled = documentStore.document?.settings.show_characters_below_header ?? false;
    // Build a { sceneIndex: string[] } map from scene_cards so the plugin can
    // merge user-supplied non-speaking characters with auto-detected speakers.
    const extras: Record<number, string[]> = {};
    const cards = documentStore.document?.scene_cards ?? [];
    for (const card of cards) {
      const raw = (card.extra_characters ?? '').trim();
      if (raw.length === 0) continue;
      extras[card.scene_index] = raw
        .split(',')
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
    }
    if (!view) return;
    const current = characterListKey.getState(view.state);
    const sameEnabled = current?.enabled === enabled;
    const sameExtras = current ? JSON.stringify(current.extras) === JSON.stringify(extras) : false;
    if (sameEnabled && sameExtras) return;
    view.dispatch(view.state.tr.setMeta(characterListKey, { enabled, extras }));
    scheduleAnnotationUpdate();
  });

  let editorElement: HTMLDivElement;
  let view: EditorView | null = null;
  const inputManager = InputModeManager.getInstance();

  // Map the font setting slug to a CSS font-family name
  let fontFamily = $derived(
    documentStore.currentFont === 'manjari' ? 'Manjari' : 'Noto Sans Malayalam'
  );

  // Svelte 5 runes for reactive state
  let isMalayalam = $state(inputManager.isMalayalam);

  // Scene index being actively edited via shortcut — forces the annotation
  // fields to show even when empty, so the user can type into them.
  let editingSceneIndex = $state<number>(-1);
  let gutterEl = $state<HTMLDivElement | null>(null);

  // ─── Scene annotations with ProseMirror spacer decorations ───
  // When an annotation is taller than its scene's natural space, a spacer
  // widget decoration is injected into the ProseMirror doc BEFORE the next
  // scene heading, pushing the editor content down to make room.
  // This keeps annotations aligned with their scene headings.

  interface SceneSlot {
    sceneOrder: number;
    extent: number;
    description: string;
    shootNotes: string;
  }

  // Plugin that inserts invisible spacer divs before scene headings.
  // Spacer heights are stored in plugin state, updated via transaction metadata.
  const spacerKey = new PluginKey<Record<number, number>>('annotation-spacers');

  const annotationSpacerPlugin = new Plugin({
    key: spacerKey,
    state: {
      init(): Record<number, number> { return {}; },
      apply(tr, value): Record<number, number> {
        const meta = tr.getMeta(spacerKey);
        return meta !== undefined ? meta : value;
      }
    },
    props: {
      decorations(state) {
        const heights = spacerKey.getState(state);
        if (!heights || Object.keys(heights).length === 0) return DecorationSet.empty;

        const decos: Decoration[] = [];
        let idx = 0;
        state.doc.forEach((node, pos) => {
          if (node.type.name === 'scene_heading') {
            const extra = heights[idx];
            if (extra && extra > 0) {
              // Insert an invisible spacer div before this scene heading
              decos.push(Decoration.widget(pos, () => {
                const el = document.createElement('div');
                el.style.height = extra + 'px';
                el.setAttribute('aria-hidden', 'true');
                return el;
              }, { side: -1 }));
            }
            idx++;
          }
        });

        return DecorationSet.create(state.doc, decos);
      }
    }
  });

  let sceneSlots = $state<SceneSlot[]>([]);
  let gutterTopPad = $state(0);
  let annotationRafId = 0;
  let spacerRafId = 0;
  let gutterResizeObserver: ResizeObserver | null = null;

  // Track which scenes have had their annotation explicitly collapsed.
  // Expanded is the default; users opt into the compact two-line view.
  // Collapsed slots limit description/notes to ~2 visible lines; expanded
  // shows the full text.
  let collapsedSlots = $state(new Set<number>());

  function toggleSlotExpanded(sceneOrder: number) {
    const next = new Set(collapsedSlots);
    if (next.has(sceneOrder)) next.delete(sceneOrder);
    else next.add(sceneOrder);
    collapsedSlots = next;
    // Svelte flushes the state change in the same microtask; a single RAF
    // is enough to let layout settle before we measure.
    scheduleSpacerRecalc();
  }

  function scheduleAnnotationUpdate() {
    cancelAnimationFrame(annotationRafId);
    annotationRafId = requestAnimationFrame(updateAnnotationPositions);
  }

  /** Debounced single-frame scheduler for spacer recomputation.
   *  Called directly after any change that might grow/shrink a gutter slot
   *  (toggle, textarea typing, ResizeObserver firing). Collapses bursts of
   *  calls into one measurement pass. */
  function scheduleSpacerRecalc() {
    cancelAnimationFrame(spacerRafId);
    spacerRafId = requestAnimationFrame(() => measureAndApplySpacers());
  }

  /** Get an element's vertical position relative to a container's content top.
   *  Uses getBoundingClientRect which is reliable regardless of
   *  offsetParent chains or ProseMirror's position:relative usage.
   *  The difference between two rects is stable regardless of scroll. */
  function posRelativeTo(el: HTMLElement, container: HTMLElement): number {
    return el.getBoundingClientRect().top - container.getBoundingClientRect().top;
  }

  function updateAnnotationPositions() {
    if (!editorElement || !view) return;
    const doc = documentStore.document;
    if (!doc) return;

    const headingEls = Array.from(
      editorElement.querySelectorAll('.scene-heading')
    ) as HTMLElement[];
    if (headingEls.length === 0) { sceneSlots = []; return; }

    // Read positions (may include existing spacers)
    const positions = headingEls.map((el) => posRelativeTo(el, editorElement));
    const editorHeight = editorElement.scrollHeight || editorElement.offsetHeight;
    gutterTopPad = positions[0];

    // Build slots so the gutter renders annotation content
    const slots: SceneSlot[] = [];
    for (let i = 0; i < headingEls.length; i++) {
      const nextTop = i + 1 < positions.length ? positions[i + 1] : editorHeight;
      const extent = nextTop - positions[i];
      const card = doc.scene_cards.find(
        (c: { scene_index: number }) => c.scene_index === i
      );
      slots.push({
        sceneOrder: i,
        extent,
        description: card?.description ?? '',
        shootNotes: card?.shoot_notes ?? '',
      });
    }
    sceneSlots = slots;

    // Slot data changed; re-measure next frame once the gutter re-renders.
    scheduleSpacerRecalc();
  }

  /** Full spacer recompute: clear → measure base positions → measure
   *  annotation heights → apply new spacers. All in one synchronous
   *  block before the browser paints, so no visible flicker. */
  function measureAndApplySpacers() {
    if (!gutterEl || !editorElement || !view) return;
    const doc = documentStore.document;
    if (!doc) return;

    const GAP = 20;

    // 1. Clear all spacers to get base positions
    const currentSpacers = spacerKey.getState(view.state) ?? {};
    if (Object.keys(currentSpacers).length > 0) {
      view.dispatch(view.state.tr.setMeta(spacerKey, {}));
    }
    // Force reflow so positions reflect the cleared state
    void editorElement.offsetHeight;

    // 2. Read base positions (no spacers)
    const headingEls = Array.from(
      editorElement.querySelectorAll('.scene-heading')
    ) as HTMLElement[];
    const basePositions = headingEls.map((el) => posRelativeTo(el, editorElement));
    const baseEditorHeight = editorElement.scrollHeight || editorElement.offsetHeight;

    // Update gutter top pad and slot extents to base values
    gutterTopPad = basePositions[0];
    for (let i = 0; i < sceneSlots.length && i < headingEls.length; i++) {
      const nextTop = i + 1 < basePositions.length ? basePositions[i + 1] : baseEditorHeight;
      sceneSlots[i].extent = nextTop - basePositions[i];
    }

    // 3. Measure actual annotation content heights (not scrollHeight of the
    //    slot, which includes min-height and would give inflated values)
    const contentEls = gutterEl.querySelectorAll('.slot-content');
    const newSpacers: Record<number, number> = {};

    contentEls.forEach((contentEl, i) => {
      const contentHeight = (contentEl as HTMLElement).offsetHeight;
      const baseExtent = sceneSlots[i]?.extent ?? 0;
      if (contentHeight > baseExtent && i + 1 < contentEls.length) {
        newSpacers[i + 1] = contentHeight - baseExtent + GAP;
      }
    });

    // 4. Apply new spacers (if any)
    if (Object.keys(newSpacers).length > 0) {
      view.dispatch(view.state.tr.setMeta(spacerKey, newSpacers));
      void editorElement.offsetHeight;

      // Update positions and extents with spacers applied
      const finalPositions = headingEls.map((el) => posRelativeTo(el, editorElement));
      const finalEditorHeight = editorElement.scrollHeight || editorElement.offsetHeight;
      gutterTopPad = finalPositions[0];

      for (let i = 0; i < sceneSlots.length && i < headingEls.length; i++) {
        const nextTop = i + 1 < finalPositions.length ? finalPositions[i + 1] : finalEditorHeight;
        sceneSlots[i].extent = nextTop - finalPositions[i];
      }
    }
  }

  /** Open annotation fields for the scene at the cursor position.
   *  Creates empty card entries if needed, then focuses the description textarea. */
  export function editCurrentSceneAnnotation() {
    if (!view || !documentStore.document) return;

    // Find which scene the cursor is in
    const cursorPos = view.state.selection.$from.pos;
    let sceneIdx = -1;
    view.state.doc.forEach((node, offset, index) => {
      if (offset <= cursorPos && node.type.name === 'scene_heading') {
        sceneIdx = index;
      }
    });

    // Count scene_heading nodes to get the 0-based scene order
    let sceneOrder = -1;
    let headingCount = 0;
    view.state.doc.forEach((node, _offset, index) => {
      if (node.type.name === 'scene_heading') {
        if (index === sceneIdx) sceneOrder = headingCount;
        headingCount++;
      }
    });

    if (sceneOrder < 0) return;

    // Ensure a scene_card entry exists
    const cards = documentStore.document.scene_cards;
    if (!cards.find((c: { scene_index: number }) => c.scene_index === sceneOrder)) {
      cards.push({ scene_index: sceneOrder, description: '', shoot_notes: '', extra_characters: '' });
    }

    // Show annotation fields for this scene and trigger update
    editingSceneIndex = sceneOrder;
    scheduleAnnotationUpdate();

    // Focus the description textarea after DOM updates
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        if (gutterEl) {
          const textarea = gutterEl.querySelector(`[data-scene="${sceneOrder}"] .ann-text`) as HTMLTextAreaElement;
          textarea?.focus();
        }
      });
    });
  }

  function updateSceneCard(sceneOrder: number, field: 'description' | 'shoot_notes', value: string) {
    if (!documentStore.document) return;
    const cards = documentStore.document.scene_cards;
    const existing = cards.find((c: { scene_index: number }) => c.scene_index === sceneOrder);
    if (existing) {
      if (field === 'description') existing.description = value;
      else existing.shoot_notes = value;
    } else {
      cards.push({
        scene_index: sceneOrder,
        description: field === 'description' ? value : '',
        shoot_notes: field === 'shoot_notes' ? value : '',
        extra_characters: '',
      });
    }
    documentStore.markDirty();
    // Textarea grows via field-sizing:content — ResizeObserver picks that up
    // automatically via scheduleSpacerRecalc. This call is a belt-and-braces
    // scheduler in case the observer hasn't fired yet for a fresh slot.
    scheduleSpacerRecalc();
  }

  // Create initial document with one empty scene_heading
  function createInitialDoc() {
    return screenplaySchema.node('doc', null, [
      screenplaySchema.node('scene_heading')
    ]);
  }

  // Update the current element type display and mark state based on cursor position
  function updateCurrentElement(state: EditorState) {
    const nodeName = state.selection.$from.parent.type.name;
    // Convert node type name to display name
    const displayNames: Record<string, string> = {
      scene_heading: 'SCENE HEADING',
      action: 'ACTION',
      character: 'CHARACTER',
      parenthetical: 'PARENTHETICAL',
      dialogue: 'DIALOGUE',
      transition: 'TRANSITION',
    };
    editorStore.currentElement = displayNames[nodeName] ?? nodeName.toUpperCase();

    // Update which inline marks are active at the current cursor/selection.
    // `storedMarks` are marks that will be applied to the next typed character
    // (set when toggling a mark with an empty selection). `$from.marks()` returns
    // marks on existing text at the cursor position.
    const marks = state.storedMarks || state.selection.$from.marks();
    editorStore.markState = {
      bold: marks.some(m => m.type === screenplaySchema.marks.bold),
      italic: marks.some(m => m.type === screenplaySchema.marks.italic),
      underline: marks.some(m => m.type === screenplaySchema.marks.underline),
    };

  }

  // Watch for New/Open events only — loadTrigger is incremented exclusively by
  // newDocument() and openDocument(), never by setContent() during typing.
  // IMPORTANT: only read loadTrigger and loadedContent here — NOT document or
  // document.content, because those are mutated on every keystroke by setContent().
  $effect(() => {
    // Read loadTrigger to establish the reactive dependency
    const _trigger = documentStore.loadTrigger;
    // Read the snapshot taken at load time — not the live document
    const content = documentStore.loadedContent;

    if (!view) return;

    let newDoc;
    if (content !== null) {
      // Parse the stored ProseMirror JSON back into a document node
      newDoc = ProseMirrorNode.fromJSON(screenplaySchema, content as Record<string, unknown>);
    } else {
      // New or empty document — start with a fresh scene_heading
      newDoc = createInitialDoc();
    }

    const newState = EditorState.create({
      doc: newDoc,
      plugins: view.state.plugins,
    });
    view.updateState(newState);
    updateCurrentElement(newState);
    scheduleAnnotationUpdate();
  });

  onMount(() => {
    // Guard: never create a second EditorView if one already exists.
    // This prevents issues if onMount somehow fires twice (e.g. HMR, re-mount).
    if (view) return;

    // Restore the document from the store if it exists (e.g. returning from Scene Cards).
    // Fall back to a fresh empty doc for first launch.
    const content = documentStore.document?.content;
    const initialDoc = content
      ? ProseMirrorNode.fromJSON(screenplaySchema, content as Record<string, unknown>)
      : createInitialDoc();

    const state = EditorState.create({
      doc: initialDoc,
      plugins: [
        characterAutocompletePlugin,
        screenplayKeymap,
        keymap(baseKeymap),
        history(),
        autoUppercasePlugin,
        findReplacePlugin,
        annotationSpacerPlugin,
        characterListPlugin,
      ]
    });

    view = new EditorView(editorElement, {
      // Store the view in editorStore so other components can access it
      // (we set editorStore.view right after the constructor returns — see below)
      state,
      dispatchTransaction(tr) {
        if (!view) return;
        const newState = view.state.apply(tr);
        view.updateState(newState);
        updateCurrentElement(newState);

        // Sync document changes to the store — setContent() does not
        // increment loadTrigger, so the $effect won't re-trigger
        if (tr.docChanged) {
          documentStore.setContent(newState.doc.toJSON());
          documentStore.markDirty();
          scheduleAnnotationUpdate();
        }
      }
    });

    // Share the EditorView with other components via the store
    editorStore.view = view;

    // Attach a capture-phase keydown listener directly on the EditorView's DOM element.
    // Using capture: true ensures this fires BEFORE ProseMirror's own keydown handler
    // and before the browser's default text input, so we can intercept keys reliably.
    const editorDom = view.dom;

    function handleMalayalamKeydown(event: KeyboardEvent) {
      // When character autocomplete dropdown is open, let ProseMirror's plugin
      // system handle navigation keys (Arrow, Enter, Tab, Escape) so the
      // autocomplete plugin can process them. Without this, the capture-phase
      // listener would swallow these keys before ProseMirror sees them.
      if (view) {
        const acState = autocompleteKey.getState(view.state);
        if (acState?.active) {
          const autocompleteKeys = new Set(['ArrowDown', 'ArrowUp', 'Enter', 'Tab', 'Escape']);
          if (autocompleteKeys.has(event.key)) {
            // Don't intercept — let ProseMirror's handleKeyDown handle it
            return;
          }
        }
      }

      // Cmd+S (Mac) or Ctrl+S (Windows/Linux) — save the document
      if ((event.metaKey || event.ctrlKey) && event.key === 's') {
        event.preventDefault();
        event.stopPropagation();
        documentStore.saveWithDialog();
        return;
      }

      // Ctrl+Space toggles input mode — intercept before ProseMirror sees it
      if (event.ctrlKey && event.code === 'Space') {
        event.preventDefault();
        event.stopPropagation();
        inputManager.toggle();
        // Sync the reactive state so the status bar updates
        isMalayalam = inputManager.isMalayalam;
        return;
      }

      // Only intercept when Malayalam mode is active, the key is a printable character
      // (key.length === 1), and no modifier keys are held
      if (
        inputManager.isMalayalam &&
        event.key.length === 1 &&
        !event.ctrlKey &&
        !event.metaKey &&
        !event.altKey
      ) {
        const result = inputManager.processKey(event.key);
        if (result !== null && view) {
          // Stop the browser from inserting the English character
          event.preventDefault();
          // Stop ProseMirror from also processing this key
          event.stopPropagation();

          // Build a ProseMirror transaction that handles Mozhi's delete-back-and-replace
          let tr = view.state.tr;
          if (result.deleteBack > 0) {
            // Delete the specified number of characters before the cursor.
            // This is needed for Mozhi, which sometimes replaces previously inserted
            // Malayalam characters (e.g., ക് + h → ഖ്, deleting ക് and inserting ഖ്).
            const from = tr.selection.from - result.deleteBack;
            const to = tr.selection.from;
            tr = tr.delete(from, to);
          }
          if (result.text) {
            tr = tr.insertText(result.text);
          }
          view.dispatch(tr);
        }

        // Reset Mozhi buffer on word boundaries — space means the next keystroke
        // should not combine with the previous word's output
        if (event.key === ' ') {
          inputManager.resetMozhi();
        }
      }

      // Reset Mozhi buffer on keys that invalidate the context, even if they
      // weren't intercepted above (e.g., Backspace deletes editor content so
      // the buffer no longer matches what's in the document)
      if (inputManager.isMalayalam && inputManager.scheme === 'mozhi') {
        if (event.key === 'Backspace' || event.key === 'Enter' ||
            event.key === 'ArrowLeft' || event.key === 'ArrowRight' ||
            event.key === 'ArrowUp' || event.key === 'ArrowDown' ||
            event.key === 'Home' || event.key === 'End') {
          inputManager.resetMozhi();
        }
      }
    }

    editorDom.addEventListener('keydown', handleMalayalamKeydown, { capture: true });

    // Set initial element display
    updateCurrentElement(view.state);
    scheduleAnnotationUpdate();

    // Recompute annotation positions on window resize
    window.addEventListener('resize', scheduleAnnotationUpdate);

    // Watch the gutter for intrinsic size changes — e.g. field-sizing:content
    // textareas growing as the user types. One observer, one debounced RAF.
    if (gutterEl && typeof ResizeObserver !== 'undefined') {
      gutterResizeObserver = new ResizeObserver(() => scheduleSpacerRecalc());
      gutterResizeObserver.observe(gutterEl);
    }

    return () => {
      editorStore.view = null;
      editorDom.removeEventListener('keydown', handleMalayalamKeydown, { capture: true });
      window.removeEventListener('resize', scheduleAnnotationUpdate);
      cancelAnimationFrame(annotationRafId);
      cancelAnimationFrame(spacerRafId);
      gutterResizeObserver?.disconnect();
      gutterResizeObserver = null;
      view?.destroy();
    };
  });
</script>

<div class="editor-wrapper">
  {#if findReplaceOpen}
    <FindReplaceBar mode={findReplaceMode} onclose={() => { findReplaceOpen = false; }} />
  {/if}
  <div class="editor-scroll">
    <div class="editor-with-annotations" style="--editor-font: '{fontFamily}'">
      <div class="editor-container" bind:this={editorElement} style="--scene-counter-start: {(documentStore.document?.settings.scene_number_start ?? 1) - 1}"></div>
      {#if showAnnotations}
      <div class="annotations-gutter" style="padding-top: {gutterTopPad}px" bind:this={gutterEl}>
        {#each sceneSlots as slot (slot.sceneOrder)}
          {@const showFields = slot.description || slot.shootNotes || editingSceneIndex === slot.sceneOrder}
          {@const expanded = !collapsedSlots.has(slot.sceneOrder)}
          <div class="scene-slot" style="min-height: {slot.extent}px" data-scene={slot.sceneOrder}>
            <div class="slot-content" class:expanded>
              {#if showFields}
                <button
                  type="button"
                  class="ann-toggle"
                  onclick={() => toggleSlotExpanded(slot.sceneOrder)}
                  aria-label={expanded ? `Collapse annotation for scene ${slot.sceneOrder + 1}` : `Expand annotation for scene ${slot.sceneOrder + 1}`}
                  title={expanded ? 'Collapse' : 'Expand'}
                >
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" style="transform: rotate({expanded ? 180 : 0}deg); transition: transform 120ms ease;">
                    <polyline points="6 9 12 15 18 9"></polyline>
                  </svg>
                </button>
                <div class="ann-field">
                  <span class="ann-label">Description</span>
                  <textarea
                    class="ann-text"
                    class:collapsed={!expanded}
                    placeholder="Scene description..."
                    value={slot.description}
                    oninput={(e: Event) => updateSceneCard(slot.sceneOrder, 'description', (e.target as HTMLTextAreaElement).value)}
                    onfocusout={() => { if (editingSceneIndex === slot.sceneOrder) editingSceneIndex = -1; }}
                  ></textarea>
                </div>
                <div class="ann-field">
                  <span class="ann-label">Notes</span>
                  <textarea
                    class="ann-text"
                    class:collapsed={!expanded}
                    placeholder="Additional notes..."
                    value={slot.shootNotes}
                    oninput={(e: Event) => updateSceneCard(slot.sceneOrder, 'shoot_notes', (e.target as HTMLTextAreaElement).value)}
                    onfocusout={() => { if (editingSceneIndex === slot.sceneOrder) editingSceneIndex = -1; }}
                  ></textarea>
                </div>
              {/if}
            </div>
          </div>
        {/each}
      </div>
      {/if}
    </div>
  </div>

  <FormatBubble />
</div>

<style>
  .editor-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .editor-scroll {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--surface-base);
    padding: 40px 0;
  }

  /* Flex-centered editor keeps the page in the viewport center regardless of
     whether the annotation gutter is visible. The gutter is absolutely
     positioned so toggling it on/off doesn't shift the editor horizontally. */
  .editor-with-annotations {
    position: relative;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    min-height: 100%;
    padding: 0 20px;
  }

  .editor-container {
    flex: 0 0 680px;
    max-width: 680px;
    min-width: 0;
    position: relative;
  }

  .annotations-gutter {
    position: absolute;
    top: 0;
    /* Park the gutter just to the right of the centered 680px editor. */
    left: calc(50% + 340px + 16px);
    width: 320px;
  }

  .scene-slot {
    box-sizing: border-box;
    flex-shrink: 0;
    border-top: 1px solid transparent;
  }

  /* Add a visible separator between adjacent annotated slots */
  .scene-slot:has(.ann-field) + .scene-slot:has(.ann-field) {
    border-top-color: var(--border-subtle);
    padding-top: 8px;
  }

  .slot-content {
    position: relative;
  }

  .ann-toggle {
    position: absolute;
    top: 0;
    right: 0;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: 3px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0.7;
    transition: opacity 120ms ease, background 120ms ease, color 120ms ease;
  }

  .ann-toggle:hover {
    opacity: 1;
    background: var(--accent-muted);
    color: var(--accent);
  }

  .ann-field {
    border-left: 2px solid var(--accent);
    padding-left: 10px;
    margin-bottom: 8px;
  }

  /* Shared tokens — keep in lock-step with .field-label in SceneCardsView.svelte */
  .ann-label {
    display: block;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: var(--label-font-size);
    font-weight: var(--label-font-weight);
    color: var(--label-color);
    text-transform: uppercase;
    letter-spacing: var(--label-tracking);
    margin-bottom: 2px;
  }

  .ann-text {
    width: 100%;
    resize: none;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    /* Inherit the editor font so annotations render Malayalam text with
       the same shaping as the page — otherwise the system font renders
       Malayalam in an entirely different style from the screenplay. */
    font-family: var(--editor-font), system-ui, -apple-system, sans-serif;
    font-size: 12px;
    line-height: 1.5;
    padding: 0;
    outline: none;
    overflow: hidden;
    field-sizing: content;
    min-height: 1.4em;
  }

  /* Collapsed state: limit the textarea to two lines of content. max-height
     caps field-sizing: content from growing past two lines; overflow hidden
     clips anything longer so the gutter stays compact. */
  .ann-text.collapsed {
    max-height: calc(1.4em * 2);
    overflow: hidden;
  }

  .ann-text::placeholder {
    color: var(--text-muted);
    opacity: 0.4;
  }

  /* ─── ProseMirror editor — the screenplay page ─── */
  .editor-container :global(.ProseMirror) {
    padding: 60px 72px 60vh 72px;
    box-sizing: border-box;
    /* Increased from 800px to 2000px to simulate infinite/continuous page rendering */
    min-height: 2000px;
    outline: none;
    font-family: var(--editor-font), sans-serif;
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-on-page);
    background-color: var(--page-bg);
    background-image: var(--page-grain);
    background-repeat: repeat;
    background-size: 240px 240px;
    border-radius: 2px;
    box-shadow:
      inset 0 1px 0 var(--page-edge-highlight),
      0 1px 2px var(--page-shadow-close),
      0 12px 32px var(--page-shadow);
    direction: ltr;
    unicode-bidi: normal;
    counter-reset: scene-counter var(--scene-counter-start, 0);
  }

  /* ─── Screenplay element styles — Hollywood format ─── */
  :global(.ProseMirror p) {
    margin: 0;
    padding: 4px 0;
  }

  :global(.ProseMirror .scene-heading) {
    font-weight: bold;
    font-size: 16px;
    margin-top: 2em;
    margin-bottom: 0.5em;
    color: var(--text-on-page);
    counter-increment: scene-counter;
    position: relative;
  }

  /* Signature gutter scene number — floats into the left margin, italic
     display style, right-aligned so two- and three-digit numbers sit on
     the same rail. Sits inside the page's 72px left padding. */
  :global(.ProseMirror .scene-heading::before) {
    content: counter(scene-counter);
    position: absolute;
    left: -56px;
    width: 40px;
    top: 0.1em;
    text-align: right;
    font-family: Georgia, 'Iowan Old Style', 'Times New Roman', serif;
    font-style: italic;
    font-weight: 400;
    font-size: 13px;
    color: var(--text-muted);
    opacity: 0.75;
    letter-spacing: 0.01em;
    pointer-events: none;
    user-select: none;
  }

  :global(.ProseMirror .scene-characters-line) {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 11px;
    line-height: 1.5;
    margin: 0 0 0.9em 0;
    padding: 0;
    user-select: none;
    -webkit-user-select: none;
  }

  :global(.ProseMirror .scene-characters-label) {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
  }

  :global(.ProseMirror .scene-characters-sep) {
    color: var(--text-muted);
    opacity: 0.5;
    font-weight: 700;
  }

  :global(.ProseMirror .scene-characters-names) {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-on-page);
    opacity: 0.75;
  }

  :global(.ProseMirror .action) {
    margin: 0.5em 0;
    color: var(--text-on-page);
    opacity: 0.85;
  }

  :global(.ProseMirror .character) {
    margin-left: 200px;
    margin-top: 1em;
    margin-bottom: 0;
    color: var(--text-on-page);
    font-weight: bold;
  }

  :global(.ProseMirror .dialogue) {
    margin-left: 100px;
    margin-right: 100px;
    margin-top: 0;
    margin-bottom: 0.5em;
    color: var(--text-on-page);
    opacity: 0.85;
  }

  :global(.ProseMirror .parenthetical) {
    margin-left: 160px;
    margin-right: 160px;
    margin-top: 0;
    margin-bottom: 0;
    color: var(--text-on-page);
    opacity: 0.6;
    font-style: italic;
  }

  :global(.ProseMirror .parenthetical::before) {
    content: "(";
  }

  :global(.ProseMirror .parenthetical::after) {
    content: ")";
  }

  :global(.ProseMirror .parenthetical br.ProseMirror-trailingBreak) {
    display: none;
  }

  :global(.ProseMirror .transition) {
    text-align: right;
    margin-top: 1em;
    color: var(--accent-deep);
    letter-spacing: 0.08em;
    font-weight: 600;
  }


  /* ─── Inline formatting (bold, italic, underline) ─── */
  :global(.ProseMirror strong) {
    font-weight: bold;
  }

  :global(.ProseMirror em) {
    font-style: italic;
  }

  :global(.ProseMirror u) {
    text-decoration: underline;
  }

  /* ─── Character autocomplete dropdown ─── */
  :global(.character-autocomplete) {
    position: absolute;
    z-index: 100;
    list-style: none;
    margin: 0;
    padding: 4px 0;
    min-width: 180px;
    max-width: 320px;
    max-height: 200px;
    overflow-y: auto;
    background: var(--surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    box-shadow: 0 4px 16px var(--shadow-medium);
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 13px;
  }

  :global(.autocomplete-item) {
    padding: 6px 12px;
    cursor: pointer;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  :global(.autocomplete-item:hover),
  :global(.autocomplete-item.selected) {
    background: var(--accent-muted);
    color: var(--accent);
  }

  /* ─── Find and Replace highlights ─── */
  :global(.find-match) {
    background: var(--find-match);
    border-radius: 2px;
  }

  :global(.find-match-current) {
    background: var(--find-match-current);
    border-radius: 2px;
    outline: 2px solid var(--accent);
  }
</style>
