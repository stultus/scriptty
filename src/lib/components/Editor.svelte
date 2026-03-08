<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState } from 'prosemirror-state';
  import { EditorView } from 'prosemirror-view';
  import { history } from 'prosemirror-history';
  import { baseKeymap } from 'prosemirror-commands';
  import { keymap } from 'prosemirror-keymap';
  import { Node as ProseMirrorNode } from 'prosemirror-model';
  import { screenplaySchema } from '$lib/editor/schema';
  import { screenplayKeymap } from '$lib/editor/keymap';
  import { autoUppercasePlugin } from '$lib/editor/autoUppercase';
  import { InputModeManager } from '$lib/editor/input/InputModeManager';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';

  let editorElement: HTMLDivElement;
  let view: EditorView | null = null;
  const inputManager = InputModeManager.getInstance();

  // Svelte 5 runes for reactive state
  let currentMode = $state<'ENGLISH' | 'MALAYALAM'>('ENGLISH');
  let currentElement = $state<string>('SCENE HEADING');
  let modeFlash = $state(false);
  let currentScheme = $state<'inscript1' | 'inscript2' | 'mozhi'>('mozhi');

  // Map the font setting slug to a CSS font-family name
  let fontFamily = $derived(
    documentStore.currentFont === 'manjari' ? 'Manjari' : 'Noto Sans Malayalam'
  );

  /** Switch the active Malayalam input scheme and update local state */
  function selectScheme(scheme: 'inscript1' | 'inscript2' | 'mozhi') {
    currentScheme = scheme;
    inputManager.setScheme(scheme);
  }

  // Create initial document with one empty scene_heading
  function createInitialDoc() {
    return screenplaySchema.node('doc', null, [
      screenplaySchema.node('scene_heading')
    ]);
  }

  // Update the current element type display based on cursor position
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
    currentElement = displayNames[nodeName] ?? nodeName.toUpperCase();
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
  });

  onMount(() => {
    // Guard: never create a second EditorView if one already exists.
    // This prevents issues if onMount somehow fires twice (e.g. HMR, re-mount).
    if (view) return;

    const state = EditorState.create({
      doc: createInitialDoc(),
      plugins: [
        screenplayKeymap,
        keymap(baseKeymap),
        history(),
        autoUppercasePlugin,
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
        const isNowMalayalam = inputManager.toggle();
        currentMode = isNowMalayalam ? 'MALAYALAM' : 'ENGLISH';
        // When toggling to Malayalam mode, ensure currentScheme reflects the manager's state
        currentScheme = inputManager.scheme;

        // Brief flash on the mode indicator for visual feedback
        modeFlash = true;
        setTimeout(() => { modeFlash = false; }, 500);
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

    return () => {
      // Clean up: clear the shared store reference, remove the capture listener,
      // then destroy the EditorView
      editorStore.view = null;
      editorDom.removeEventListener('keydown', handleMalayalamKeydown, { capture: true });
      view?.destroy();
    };
  });
</script>

<div class="editor-wrapper">
  <div class="editor-container" bind:this={editorElement} style="--editor-font: '{fontFamily}'"></div>
  <div class="status-bar">
    <span class="status-mode" class:malayalam={currentMode === 'MALAYALAM'} class:flash={modeFlash}>
      {currentMode}
    </span>
    {#if currentMode === 'MALAYALAM'}
      <span class="status-separator">|</span>
      <span class="scheme-selector">
        <button
          class="scheme-btn"
          class:active={currentScheme === 'mozhi'}
          onclick={() => selectScheme('mozhi')}
        >Mozhi</button>
        <button
          class="scheme-btn"
          class:active={currentScheme === 'inscript2'}
          onclick={() => selectScheme('inscript2')}
        >Inscript 2</button>
        <button
          class="scheme-btn"
          class:active={currentScheme === 'inscript1'}
          onclick={() => selectScheme('inscript1')}
        >Inscript 1</button>
      </span>
    {/if}
    <span class="status-separator">|</span>
    <span class="status-element">{currentElement}</span>
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

  .editor-container {
    flex: 1;
    overflow-y: auto;
    background: #141414;
  }

  /* ProseMirror editor styles — fixed-width content area centered in the container */
  .editor-container :global(.ProseMirror) {
    max-width: 680px;
    margin: 0 auto;
    padding: 60px 80px;
    box-sizing: border-box;
    min-height: 100%;
    outline: none;
    font-family: var(--editor-font), sans-serif;
    font-size: 14px;
    line-height: 1.6;
    color: #e0e0e0;
    background: #1c1c1c;
    border-radius: 2px;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.5);
    direction: ltr;
    unicode-bidi: normal;
    counter-reset: scene-counter;
  }

  /* Screenplay element styles — Hollywood format
   * All inner ProseMirror DOM styles must use :global() because ProseMirror
   * generates its own DOM nodes without Svelte's scoping attributes.
   * Fixed pixel margins relative to ~520px usable content area.
   * No text-transform: uppercase — preserves Malayalam script correctly. */

  :global(.ProseMirror p) {
    margin: 0;
    padding: 4px 0;
  }

  :global(.ProseMirror .scene-heading) {
    font-weight: bold;
    font-size: 16px;
    margin-top: 2em;
    margin-bottom: 0.5em;
    color: #fff;
    counter-increment: scene-counter;
  }

  :global(.ProseMirror .scene-heading::before) {
    content: counter(scene-counter) ". ";
    color: #888;
    font-size: 11px;
    font-weight: normal;
    margin-right: 4px;
  }

  :global(.ProseMirror .action) {
    margin: 0.5em 0;
    color: #ddd;
  }

  :global(.ProseMirror .character) {
    margin-left: 200px;
    margin-top: 1em;
    margin-bottom: 0;
    color: #fff;
    font-weight: bold;
  }

  :global(.ProseMirror .dialogue) {
    margin-left: 100px;
    margin-right: 100px;
    margin-top: 0;
    margin-bottom: 0;
    color: #ddd;
  }

  :global(.ProseMirror .parenthetical) {
    margin-left: 160px;
    margin-right: 160px;
    margin-top: 0;
    margin-bottom: 0;
    color: #aaa;
    font-style: italic;
  }

  :global(.ProseMirror .transition) {
    text-align: right;
    margin-top: 1em;
    color: #ccc;
  }

  /* Status bar */
  .status-bar {
    position: fixed;
    bottom: 0;
    right: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 16px;
    background: #2a2a2a;
    border-top: 1px solid #333;
    border-left: 1px solid #333;
    border-top-left-radius: 6px;
    font-size: 11px;
    font-family: system-ui, sans-serif;
    color: #888;
    z-index: 100;
    user-select: none;
  }

  .status-mode {
    font-weight: 600;
    color: #4fc3f7;
  }

  .status-mode.malayalam {
    color: #81c784;
  }

  .status-mode.flash {
    background: rgba(255, 255, 255, 0.15);
    border-radius: 3px;
    padding: 1px 6px;
    transition: background 0.5s ease-out;
  }

  .status-separator {
    color: #555;
  }

  .status-element {
    color: #aaa;
  }

  .scheme-selector {
    display: flex;
    gap: 2px;
  }

  .scheme-btn {
    background: none;
    border: none;
    padding: 1px 6px;
    font-size: 11px;
    font-family: system-ui, sans-serif;
    color: #666;
    cursor: pointer;
    border-radius: 3px;
  }

  .scheme-btn:hover {
    color: #aaa;
    background: rgba(255, 255, 255, 0.08);
  }

  .scheme-btn.active {
    color: #81c784;
    font-weight: 600;
  }
</style>
