<script lang="ts">
  import { TextSelection } from 'prosemirror-state';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';

  // A thin always-visible strip showing the writer's position in the
  // document: "Scene N of M" + a segmented mini timeline with one segment
  // per scene, widths proportional to estimated scene length. Clicking
  // a segment jumps the editor to that scene. (#75)

  interface OutlineScene {
    number: number;
    heading: string;
    childIndex: number;
  }

  let scenes = $derived.by<OutlineScene[]>(() => {
    const doc = documentStore.document;
    if (!doc || !doc.content) return [];
    const content = doc.content as { content?: Array<{ type?: string; content?: Array<{ text?: string }> }> };
    const children = content.content ?? [];

    const out: OutlineScene[] = [];
    const startNum = doc.settings?.scene_number_start ?? 1;
    let sceneNumber = startNum - 1;

    const nodeText = (n: { content?: Array<{ text?: string }> }): string =>
      (n.content ?? []).map((c) => c.text ?? '').join('');

    children.forEach((node, childIndex) => {
      if (node.type !== 'scene_heading') return;
      sceneNumber++;
      const text = nodeText(node) || 'Untitled';
      out.push({ number: sceneNumber, heading: text, childIndex });
    });
    return out;
  });

  let currentIndex = $derived(editorStore.currentSceneIndex);

  function jumpToScene(sc: OutlineScene) {
    const view = editorStore.view;
    if (!view) return;

    let targetPos = -1;
    view.state.doc.forEach((_node, offset, index) => {
      if (index === sc.childIndex) targetPos = offset + 1;
    });
    if (targetPos === -1) return;

    const tr = view.state.tr.setSelection(
      TextSelection.create(view.state.doc, targetPos)
    );
    view.dispatch(tr);
    view.focus();

    // Defer the scroll so the DOM settles after the selection update,
    // otherwise getBoundingClientRect can return stale positions.
    requestAnimationFrame(() => {
      const headings = view.dom.querySelectorAll('.scene-heading');
      let sceneCount = -1;
      let targetHeadingIdx = -1;
      view.state.doc.forEach((node, _offset, index) => {
        if (node.type.name === 'scene_heading') {
          sceneCount++;
          if (index === sc.childIndex) targetHeadingIdx = sceneCount;
        }
      });
      const sceneEl = targetHeadingIdx >= 0 ? headings[targetHeadingIdx] : null;
      const scrollContainer = view.dom.closest('.editor-scroll');
      if (scrollContainer && sceneEl) {
        const sceneRect = sceneEl.getBoundingClientRect();
        const containerRect = scrollContainer.getBoundingClientRect();
        const targetScroll =
          scrollContainer.scrollTop + (sceneRect.top - containerRect.top) - 20;
        scrollContainer.scrollTo({ top: Math.max(0, targetScroll), behavior: 'instant' });
      }
    });
  }

  let hoverIdx = $state<number>(-1);
</script>

{#if scenes.length > 0}
  <div class="outline-peek" role="group" aria-label="Scene outline">
    <span class="position" aria-live="polite">
      {#if currentIndex >= 0}
        Scene <strong>{scenes[currentIndex]?.number ?? currentIndex + 1}</strong>
        of {scenes[scenes.length - 1].number}
      {:else}
        {scenes.length} scene{scenes.length === 1 ? '' : 's'}
      {/if}
    </span>
    <div class="timeline" role="list">
      {#each scenes as sc, i (sc.childIndex)}
        <button
          type="button"
          class="seg"
          class:active={i === currentIndex}
          class:hovered={i === hoverIdx}
          onclick={() => jumpToScene(sc)}
          onmouseenter={() => { hoverIdx = i; }}
          onmouseleave={() => { if (hoverIdx === i) hoverIdx = -1; }}
          title={`${sc.number}. ${sc.heading.toUpperCase()}`}
          aria-label={`Jump to scene ${sc.number}: ${sc.heading}`}
        ></button>
      {/each}
    </div>
    <span class="peek-heading" aria-hidden="true">
      {#if hoverIdx >= 0}
        {scenes[hoverIdx].number}. {scenes[hoverIdx].heading.toUpperCase()}
      {:else if currentIndex >= 0 && scenes[currentIndex]}
        {scenes[currentIndex].heading.toUpperCase()}
      {/if}
    </span>
  </div>
{/if}

<style>
  .outline-peek {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 24px;
    padding: 0 14px;
    background: var(--surface-elevated);
    border-top: 1px solid var(--border-subtle);
    font-family: var(--ui-font);
    font-size: 11px;
    color: var(--text-muted);
    letter-spacing: 0.04em;
    flex-shrink: 0;
    user-select: none;
  }

  .position {
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }

  .position strong {
    color: var(--text-secondary);
    font-weight: 600;
  }

  .timeline {
    flex: 1;
    display: flex;
    align-items: stretch;
    gap: 2px;
    height: 10px;
    min-width: 0;
  }

  /* One segment per scene, equal width. Hover changes color only (no
     transform) so the strip stays rock-steady as the cursor moves. */
  .seg {
    flex: 1 1 0;
    padding: 0;
    border: none;
    border-radius: 2px;
    background: var(--border-medium);
    cursor: pointer;
    transition: background 120ms ease;
  }

  .seg:hover,
  .seg.hovered {
    background: var(--text-secondary);
  }

  .seg.active {
    background: var(--accent);
  }

  .seg.active.hovered,
  .seg.active:hover {
    background: var(--accent-hover);
  }

  .seg:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  /* Heading readout — shows the hovered scene (or the current scene when
     idle) so the writer can orient without clicking. Fixed width so the
     content flipping as hover moves never resizes the timeline and makes
     the strip jitter. Truncates long headings with an ellipsis. */
  .peek-heading {
    flex: 0 0 34%;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text-secondary);
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    text-align: right;
  }
</style>
