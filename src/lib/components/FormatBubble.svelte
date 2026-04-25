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
   * ProseMirror's coordsAtPos for the selection endpoints. If the
   * selection is empty, hides the bubble.
   *
   * IMPORTANT: this function does NOT gate on view.hasFocus(). The
   * previous implementation did, which caused the bubble to silently
   * skip showing during drag-selection / double-click / triple-click
   * windows where focus was in transit (#169).
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

  /** Reposition on the next animation frame — gives the OS a chance to
   *  settle layout (especially after dblclick word-selection or scroll). */
  function repositionSoon() {
    const view = editorStore.view;
    if (!view) return;
    requestAnimationFrame(() => {
      // Re-check view existence in case it was destroyed before the rAF
      // fires (e.g. unmounting during a quick view switch).
      const v = editorStore.view;
      if (v) reposition(v);
    });
  }

  /** Selection-change listener — the primary path for keyboard and most
   *  mouse selections. Show whenever there's a non-empty selection in
   *  the editor's DOM, regardless of focus race state. */
  $effect(() => {
    function onSelectionChange() {
      const view = editorStore.view;
      if (!view) return;
      // Verify the selection lives inside the editor's DOM — selection
      // changes from other inputs (modal text fields, etc.) shouldn't
      // surface the bubble.
      const native = window.getSelection();
      if (!native || native.rangeCount === 0) return;
      const anchor = native.anchorNode;
      if (!anchor || !view.dom.contains(anchor)) {
        // Selection lives outside the editor — hide the bubble.
        if (visible) visible = false;
        return;
      }
      reposition(view);
    }

    document.addEventListener('selectionchange', onSelectionChange);
    return () => document.removeEventListener('selectionchange', onSelectionChange);
  });

  /** Belt-and-braces show path — fires after the OS has settled the
   *  selection regardless of focus. Catches the drag-completed case and
   *  double/triple-click word/line selection that the bare
   *  selectionchange listener can race past. */
  $effect(() => {
    const view = editorStore.view;
    if (!view) return;
    function onMouseUp() { repositionSoon(); }
    function onDblClick() { repositionSoon(); }
    function onClickMulti(e: MouseEvent) {
      if (e.detail >= 2) repositionSoon();
    }
    view.dom.addEventListener('mouseup', onMouseUp);
    view.dom.addEventListener('dblclick', onDblClick);
    view.dom.addEventListener('click', onClickMulti);
    return () => {
      view.dom.removeEventListener('mouseup', onMouseUp);
      view.dom.removeEventListener('dblclick', onDblClick);
      view.dom.removeEventListener('click', onClickMulti);
    };
  });

  /** Outside-click dismiss — the previous implementation relied on
   *  focusout, which never fired when the click target didn't take
   *  focus (title-bar drag region, decorative overlays). A direct
   *  pointerdown listener on document is the reliable signal. */
  $effect(() => {
    function onPointerDown(e: PointerEvent) {
      if (!visible) return;
      const view = editorStore.view;
      if (!view) return;
      const target = e.target as Node | null;
      if (!target) return;
      // Click is inside the bubble — keep it; a button click flips the
      // selection's marks.
      if (bubbleEl && bubbleEl.contains(target)) return;
      // Click is inside the editor — selectionchange handles show/hide.
      if (view.dom.contains(target)) return;
      visible = false;
    }
    document.addEventListener('pointerdown', onPointerDown);
    return () => document.removeEventListener('pointerdown', onPointerDown);
  });

  /** Escape dismisses. */
  $effect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape' && visible) visible = false;
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  });

  /** Reposition on editor scroll so the bubble follows the selection
   *  rather than floating over wrong text. Hides if the selection
   *  scrolls outside the visible viewport. */
  $effect(() => {
    const view = editorStore.view;
    if (!view) return;
    const scrollEl = view.dom.closest('.editor-scroll') as HTMLElement | null;
    if (!scrollEl) return;
    function onScroll() {
      if (!visible) return;
      const v = editorStore.view;
      if (!v) return;
      const sel = v.state.selection;
      if (sel.empty) { visible = false; return; }
      const coords = v.coordsAtPos(sel.from);
      const sr = scrollEl!.getBoundingClientRect();
      // Hide if the selection has scrolled out of the editor viewport.
      if (coords.top < sr.top || coords.top > sr.bottom) {
        visible = false;
        return;
      }
      reposition(v);
    }
    scrollEl.addEventListener('scroll', onScroll, { passive: true });
    return () => scrollEl.removeEventListener('scroll', onScroll);
  });

  // Bubble element ref, used by the outside-click handler.
  let bubbleEl = $state<HTMLDivElement | null>(null);

  // Track pointer over the bubble so we don't dismiss it while the user
  // is moving toward a button. Kept for hover hysteresis even though
  // dismiss is now driven by pointerdown / outside-click.
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
    bind:this={bubbleEl}
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
