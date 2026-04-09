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
  import { findReplacePlugin } from '$lib/editor/findReplace';
  import FindReplaceBar from '$lib/components/FindReplaceBar.svelte';
  import { InputModeManager } from '$lib/editor/input/InputModeManager';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';

  let {
    findReplaceOpen = $bindable(false),
    findReplaceMode = $bindable<'find' | 'replace'>('find'),
    showAnnotations = true,
  } = $props<{
    findReplaceOpen: boolean;
    findReplaceMode: 'find' | 'replace';
    showAnnotations?: boolean;
  }>();

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

  function scheduleAnnotationUpdate() {
    cancelAnimationFrame(annotationRafId);
    annotationRafId = requestAnimationFrame(updateAnnotationPositions);
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

    // Next frame: measure actual DOM heights and recompute spacers
    requestAnimationFrame(() => measureAndApplySpacers());
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
      cards.push({ scene_index: sceneOrder, description: '', shoot_notes: '' });
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
      });
    }
    documentStore.markDirty();
    // Recalculate spacers since annotation height may have changed.
    // Schedule both a full update AND a direct measurement pass.
    // The measurement pass can run even if slot data hasn't changed
    // (because the textarea grew from field-sizing:content).
    cancelAnimationFrame(annotationRafId);
    annotationRafId = requestAnimationFrame(() => {
      measureAndApplySpacers();
    });
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

    return () => {
      editorStore.view = null;
      editorDom.removeEventListener('keydown', handleMalayalamKeydown, { capture: true });
      window.removeEventListener('resize', scheduleAnnotationUpdate);
      cancelAnimationFrame(annotationRafId);
      view?.destroy();
    };
  });
</script>

<div class="editor-wrapper">
  {#if findReplaceOpen}
    <FindReplaceBar mode={findReplaceMode} onclose={() => { findReplaceOpen = false; }} />
  {/if}
  <div class="editor-scroll">
    <div class="editor-with-annotations">
      <div class="editor-container" bind:this={editorElement} style="--editor-font: '{fontFamily}'; --scene-counter-start: {(documentStore.document?.settings.scene_number_start ?? 1) - 1}"></div>
      {#if showAnnotations}
      <div class="annotations-gutter" style="padding-top: {gutterTopPad}px" bind:this={gutterEl}>
        {#each sceneSlots as slot (slot.sceneOrder)}
          {@const showFields = slot.description || slot.shootNotes || editingSceneIndex === slot.sceneOrder}
          <div class="scene-slot" style="min-height: {slot.extent}px" data-scene={slot.sceneOrder}>
            <div class="slot-content">
              {#if showFields}
                <div class="ann-field">
                  <span class="ann-label">Description</span>
                  <textarea
                    class="ann-text"
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

  .editor-with-annotations {
    display: flex;
    justify-content: center;
    align-items: flex-start;
    gap: 16px;
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
    flex: 0 1 320px;
    min-width: 0;
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

  .ann-field {
    border-left: 2px solid var(--accent);
    padding-left: 10px;
    margin-bottom: 8px;
  }

  .ann-label {
    display: block;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 10px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin-bottom: 2px;
  }

  .ann-text {
    width: 100%;
    resize: none;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 12px;
    line-height: 1.4;
    padding: 0;
    outline: none;
    overflow: hidden;
    field-sizing: content;
    min-height: 1.4em;
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
    background: var(--page-bg);
    border-radius: 2px;
    box-shadow: 0 4px 24px var(--page-shadow), 0 1px 4px rgba(0, 0, 0, 0.2);
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
  }

  :global(.ProseMirror .scene-heading::before) {
    content: counter(scene-counter) ". ";
    color: var(--text-muted);
    font-size: 16px;
    font-weight: bold;
    margin-right: 4px;
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
    color: var(--text-on-page);
    opacity: 0.7;
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
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
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
    background: rgba(255, 213, 79, 0.35);
    border-radius: 2px;
  }

  :global(.find-match-current) {
    background: rgba(255, 152, 0, 0.55);
    border-radius: 2px;
    outline: 2px solid var(--accent);
  }
</style>
