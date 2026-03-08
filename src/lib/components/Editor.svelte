<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState } from 'prosemirror-state';
  import { EditorView } from 'prosemirror-view';
  import { history } from 'prosemirror-history';
  import { baseKeymap } from 'prosemirror-commands';
  import { keymap } from 'prosemirror-keymap';
  import { screenplaySchema } from '$lib/editor/schema';
  import { screenplayKeymap } from '$lib/editor/keymap';
  import { InputModeManager } from '$lib/editor/input/InputModeManager';

  let editorElement: HTMLDivElement;
  let view: EditorView | null = null;
  const inputManager = InputModeManager.getInstance();

  // Svelte 5 runes for reactive state
  let currentMode = $state<'ENGLISH' | 'MALAYALAM'>('ENGLISH');
  let currentElement = $state<string>('SCENE HEADING');

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
      ]
    });

    view = new EditorView(editorElement, {
      state,
      dispatchTransaction(tr) {
        if (!view) return;
        const newState = view.state.apply(tr);
        view.updateState(newState);
        updateCurrentElement(newState);
      }
    });

    // Attach a capture-phase keydown listener directly on the EditorView's DOM element.
    // Using capture: true ensures this fires BEFORE ProseMirror's own keydown handler
    // and before the browser's default text input, so we can intercept keys reliably.
    const editorDom = view.dom;

    function handleMalayalamKeydown(event: KeyboardEvent) {
      // Debug: log every keypress with scheme and processKey result
      if (event.key.length === 1) {
        console.log('[Scriptty keydown]', event.key,
          'isMalayalam:', inputManager.isMalayalam,
          'scheme:', inputManager.scheme,
          'processKey result:', inputManager.processKey(event.key));
      }

      // Ctrl+Space toggles input mode — intercept before ProseMirror sees it
      if (event.ctrlKey && event.code === 'Space') {
        event.preventDefault();
        event.stopPropagation();
        const isNowMalayalam = inputManager.toggle();
        currentMode = isNowMalayalam ? 'MALAYALAM' : 'ENGLISH';
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

          // Insert the Malayalam character via a ProseMirror transaction
          const tr = view.state.tr.insertText(result);
          view.dispatch(tr);
        }
      }
    }

    editorDom.addEventListener('keydown', handleMalayalamKeydown, { capture: true });

    // Set initial element display
    updateCurrentElement(view.state);

    return () => {
      // Clean up: remove the capture listener, then destroy the EditorView
      editorDom.removeEventListener('keydown', handleMalayalamKeydown, { capture: true });
      view?.destroy();
    };
  });
</script>

<div class="editor-wrapper">
  <div class="editor-container" bind:this={editorElement}></div>
  <div class="status-bar">
    <span class="status-mode" class:malayalam={currentMode === 'MALAYALAM'}>
      {currentMode}
    </span>
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
    padding: 2rem;
  }

  /* ProseMirror editor styles */
  .editor-container :global(.ProseMirror) {
    max-width: 816px; /* ~8.5 inches at 96dpi — standard US Letter width */
    margin: 0 auto;
    padding: 3rem 2rem;
    min-height: 100%;
    outline: none;
    font-family: var(--editor-font, 'Noto Sans Malayalam'), sans-serif;
    font-size: 14px;
    line-height: 1.6;
    color: #e0e0e0;
    background: #242424;
    border-radius: 4px;
  }

  .editor-container :global(.ProseMirror p) {
    margin: 0;
    padding: 4px 0;
  }

  /* Screenplay element styles — Hollywood format positioning */
  .editor-container :global(.scene-heading) {
    text-transform: uppercase;
    font-weight: 700;
    margin-top: 1.5em;
  }

  .editor-container :global(.action) {
    /* Full width, default formatting */
  }

  .editor-container :global(.character) {
    text-transform: uppercase;
    text-align: center;
    padding-left: 30%;
    margin-top: 1em;
  }

  .editor-container :global(.parenthetical) {
    padding-left: 25%;
    max-width: 60%;
    font-style: italic;
  }

  .editor-container :global(.dialogue) {
    padding-left: 15%;
    padding-right: 15%;
  }

  .editor-container :global(.transition) {
    text-transform: uppercase;
    text-align: right;
    margin-top: 1em;
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

  .status-separator {
    color: #555;
  }

  .status-element {
    color: #aaa;
  }
</style>
