<script lang="ts">
  import { editorStore } from '$lib/stores/editorStore.svelte';
  import { toggleMark } from 'prosemirror-commands';
  import { screenplaySchema } from '$lib/editor/schema';
  import type { EditorView } from 'prosemirror-view';

  let visible = $state(false);
  let x = $state(0);
  let y = $state(0);

  let isBoldActive = $derived(editorStore.markState.bold);
  let isItalicActive = $derived(editorStore.markState.italic);
  let isUnderlineActive = $derived(editorStore.markState.underline);

  function applyMark(markName: 'bold' | 'italic' | 'underline') {
    const view = editorStore.view;
    if (!view) return;
    toggleMark(screenplaySchema.marks[markName])(view.state, view.dispatch);
    view.focus();
  }

  /**
   * Position the bubble above the current non-empty selection. Uses
   * ProseMirror's coordsAtPos for the selection endpoints and reads the
   * scroll container's rect so the bubble stays anchored to the viewport
   * coordinate space of the editor scroller.
   */
  function reposition(view: EditorView) {
    const sel = view.state.selection;
    if (sel.empty) {
      visible = false;
      return;
    }

    const fromCoords = view.coordsAtPos(sel.from);
    const toCoords = view.coordsAtPos(sel.to);
    const left = (fromCoords.left + toCoords.left) / 2;
    const top = Math.min(fromCoords.top, toCoords.top);

    x = left;
    y = top - 8;
    visible = true;
  }

  // Poll for selection changes via the editor store's markState + selection.
  // ProseMirror doesn't expose a subscriber API directly — we hook into
  // document-level selectionchange (fires on every cursor movement) which
  // is cheap and catches both mouse and keyboard selection changes.
  $effect(() => {
    function onSelectionChange() {
      const view = editorStore.view;
      if (!view || !view.hasFocus()) {
        // Keep the bubble visible briefly while clicking a bubble button —
        // otherwise the focus leaves the editor and we hide prematurely.
        return;
      }
      reposition(view);
    }

    document.addEventListener('selectionchange', onSelectionChange);
    return () => document.removeEventListener('selectionchange', onSelectionChange);
  });

  // Hide the bubble when the editor loses focus entirely (e.g. user clicks
  // the sidebar). Uses a focusout listener filtered to the document level.
  $effect(() => {
    function onFocusOut() {
      // Defer — a click inside the bubble fires focusout on the editor
      // even though we want the bubble to stay up.
      setTimeout(() => {
        const view = editorStore.view;
        if (!view) { visible = false; return; }
        if (!view.hasFocus() && !isPointerOverBubble) visible = false;
      }, 0);
    }
    document.addEventListener('focusout', onFocusOut);
    return () => document.removeEventListener('focusout', onFocusOut);
  });

  // Track pointer over the bubble so we don't dismiss it while the user
  // is moving toward a button.
  let isPointerOverBubble = false;
  function handlePointerEnter() { isPointerOverBubble = true; }
  function handlePointerLeave() { isPointerOverBubble = false; }

  // Using mousedown + preventDefault keeps focus inside the editor so
  // toggleMark operates on the current selection.
  function handleMouseDown(event: MouseEvent, mark: 'bold' | 'italic' | 'underline') {
    event.preventDefault();
    applyMark(mark);
  }
</script>

{#if visible}
  <div
    class="format-bubble"
    style="left: {x}px; top: {y}px;"
    onpointerenter={handlePointerEnter}
    onpointerleave={handlePointerLeave}
    role="toolbar"
    tabindex="-1"
    aria-label="Text formatting"
  >
    <button
      class="fmt-btn"
      class:active={isBoldActive}
      onmousedown={(e) => handleMouseDown(e, 'bold')}
      title="Bold (⌘B)"
      aria-pressed={isBoldActive}
    ><span class="fmt-bold">B</span></button>
    <button
      class="fmt-btn"
      class:active={isItalicActive}
      onmousedown={(e) => handleMouseDown(e, 'italic')}
      title="Italic (⌘I)"
      aria-pressed={isItalicActive}
    ><span class="fmt-italic">I</span></button>
    <button
      class="fmt-btn"
      class:active={isUnderlineActive}
      onmousedown={(e) => handleMouseDown(e, 'underline')}
      title="Underline (⌘U)"
      aria-pressed={isUnderlineActive}
    ><span class="fmt-underline">U</span></button>
  </div>
{/if}

<style>
  .format-bubble {
    position: fixed;
    transform: translate(-50%, -100%);
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 3px;
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 7px;
    box-shadow:
      0 4px 14px var(--shadow-medium),
      0 1px 2px var(--shadow-soft);
    z-index: 400;
    animation: bubble-in 140ms ease-out;
  }

  @keyframes bubble-in {
    from { opacity: 0; transform: translate(-50%, calc(-100% + 4px)); }
    to { opacity: 1; transform: translate(-50%, -100%); }
  }

  .fmt-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-secondary);
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 13px;
    cursor: pointer;
    transition: background 100ms ease, color 100ms ease;
  }

  .fmt-btn:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .fmt-btn.active {
    background: var(--accent-muted);
    color: var(--accent);
  }

  .fmt-bold { font-weight: 700; }
  .fmt-italic { font-style: italic; font-family: Georgia, 'Iowan Old Style', serif; }
  .fmt-underline { text-decoration: underline; text-underline-offset: 2px; }
</style>
