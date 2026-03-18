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
  import { characterAutocompletePlugin, autocompleteKey } from '$lib/editor/characterAutocomplete';
  import { findReplacePlugin } from '$lib/editor/findReplace';
  import FindReplaceBar from '$lib/components/FindReplaceBar.svelte';
  import { InputModeManager } from '$lib/editor/input/InputModeManager';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';

  let {
    findReplaceOpen = $bindable(false),
    findReplaceMode = $bindable<'find' | 'replace'>('find'),
  } = $props();

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
  {#if findReplaceOpen}
    <FindReplaceBar mode={findReplaceMode} onclose={() => { findReplaceOpen = false; }} />
  {/if}
  <div class="editor-scroll">
    <div class="editor-container" bind:this={editorElement} style="--editor-font: '{fontFamily}'"></div>
  </div>
  <div class="status-bar">
    <div class="status-left">
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
    </div>
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

  .editor-scroll {
    flex: 1;
    overflow-y: auto;
    background: var(--surface-base);
    padding: 40px 0;
  }

  .editor-container {
    max-width: 680px;
    margin: 0 auto;
    position: relative;
  }

  /* ─── ProseMirror editor — the screenplay page ─── */
  .editor-container :global(.ProseMirror) {
    padding: 60px 72px;
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
    counter-reset: scene-counter;
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
    font-size: 11px;
    font-weight: normal;
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
    margin-bottom: 0;
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

  /* ─── Status bar — full-width bottom bar ─── */
  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 28px;
    padding: 0 16px;
    background: var(--surface-elevated);
    border-top: 1px solid var(--border-subtle);
    font-size: 11px;
    font-family: system-ui, -apple-system, sans-serif;
    color: var(--text-muted);
    user-select: none;
    flex-shrink: 0;
    letter-spacing: 0.04em;
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-mode {
    font-weight: 600;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .status-mode.malayalam {
    color: var(--accent);
  }

  .status-mode.flash {
    background: var(--accent-muted);
    border-radius: 3px;
    padding: 1px 6px;
    transition: background 0.5s ease-out;
  }

  .status-separator {
    color: var(--border-medium);
  }

  .status-element {
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .scheme-selector {
    display: flex;
    gap: 1px;
  }

  .scheme-btn {
    background: none;
    border: none;
    padding: 2px 6px;
    font-size: 11px;
    font-family: system-ui, -apple-system, sans-serif;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: 3px;
    transition: background 100ms, color 100ms;
    letter-spacing: 0.04em;
  }

  .scheme-btn:hover {
    color: var(--text-secondary);
    background: var(--surface-hover);
  }

  .scheme-btn.active {
    color: var(--accent);
    font-weight: 600;
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
