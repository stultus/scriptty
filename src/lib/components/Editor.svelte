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
  import SettingsModal from '$lib/components/SettingsModal.svelte';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';

  let {
    findReplaceOpen = $bindable(false),
    findReplaceMode = $bindable<'find' | 'replace'>('find'),
  } = $props();

  let editorElement: HTMLDivElement;
  let view: EditorView | null = null;
  const inputManager = InputModeManager.getInstance();

  // Map the font setting slug to a CSS font-family name
  let fontFamily = $derived(
    documentStore.currentFont === 'manjari' ? 'Manjari' : 'Noto Sans Malayalam'
  );

  // Svelte 5 runes for reactive state
  let currentElement = $state<string>('SCENE HEADING');
  let showSettings = $state(false);
  let isMalayalam = $state(inputManager.isMalayalam);

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
    currentElement = displayNames[nodeName] ?? nodeName.toUpperCase();

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
      <button class="settings-btn" onclick={() => { showSettings = true; }} title="Configure Settings">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="4" y1="21" x2="4" y2="14"></line><line x1="4" y1="10" x2="4" y2="3"></line>
          <line x1="12" y1="21" x2="12" y2="12"></line><line x1="12" y1="8" x2="12" y2="3"></line>
          <line x1="20" y1="21" x2="20" y2="16"></line><line x1="20" y1="12" x2="20" y2="3"></line>
          <line x1="1" y1="14" x2="7" y2="14"></line><line x1="9" y1="8" x2="15" y2="8"></line>
          <line x1="17" y1="16" x2="23" y2="16"></line>
        </svg>
      </button>
      <span class="status-lang" class:malayalam={isMalayalam}>
        {isMalayalam ? 'MAL' : 'ENG'}
      </span>
    </div>
    <span class="status-element">{currentElement}</span>
  </div>
</div>

<SettingsModal bind:open={showSettings} />

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

  .status-element {
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .status-lang {
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 3px;
    transition: background 120ms ease, color 120ms ease;
  }

  .status-lang.malayalam {
    color: var(--accent);
    background: var(--accent-muted);
  }

  .settings-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    width: 24px;
    height: 24px;
    border-radius: 4px;
    transition: background 120ms, color 120ms;
  }

  .settings-btn:hover {
    color: var(--text-primary);
    background: var(--surface-hover);
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
