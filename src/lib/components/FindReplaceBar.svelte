<script lang="ts">
  import { onMount } from 'svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';
  import {
    findReplaceKey,
    dispatchSearch,
    dispatchNext,
    dispatchPrev,
    dispatchClear,
    replaceCurrentMatch,
    replaceAllMatches,
    scrollToCurrentMatch,
  } from '$lib/editor/findReplace';

  let { mode = 'find', onclose }: { mode: 'find' | 'replace'; onclose: () => void } = $props();

  let query = $state('');
  let replacement = $state('');
  let caseSensitive = $state(false);
  let matchCount = $state(0);
  let currentIndex = $state(-1);

  let findInput: HTMLInputElement;

  // Read the plugin state and update local reactive variables
  function syncState(): void {
    const view = editorStore.view;
    if (!view) return;
    const ps = findReplaceKey.getState(view.state);
    if (ps) {
      matchCount = ps.matches.length;
      currentIndex = ps.currentIndex;
    }
  }

  // Re-run search whenever query or caseSensitive changes
  $effect(() => {
    const view = editorStore.view;
    if (!view) return;

    // Read reactive values to establish dependency
    const q = query;
    const cs = caseSensitive;

    if (q.length > 0) {
      dispatchSearch(view, q, cs);
      syncState();
      scrollToCurrentMatch(view);
    } else {
      dispatchClear(view);
      matchCount = 0;
      currentIndex = -1;
    }
  });

  function next(): void {
    const view = editorStore.view;
    if (!view) return;
    dispatchNext(view);
    syncState();
    scrollToCurrentMatch(view);
  }

  function prev(): void {
    const view = editorStore.view;
    if (!view) return;
    dispatchPrev(view);
    syncState();
    scrollToCurrentMatch(view);
  }

  function toggleCase(): void {
    caseSensitive = !caseSensitive;
  }

  function close(): void {
    const view = editorStore.view;
    if (view) {
      dispatchClear(view);
      view.focus();
    }
    onclose();
  }

  function handleReplace(): void {
    const view = editorStore.view;
    if (!view) return;
    replaceCurrentMatch(view, replacement);
    syncState();
    scrollToCurrentMatch(view);
  }

  function handleReplaceAll(): void {
    const view = editorStore.view;
    if (!view) return;
    replaceAllMatches(view, replacement);
    syncState();
  }

  function handleFindKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' && event.shiftKey) {
      event.preventDefault();
      prev();
    } else if (event.key === 'Enter') {
      event.preventDefault();
      next();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      close();
    }
  }

  function handleReplaceKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      handleReplace();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      close();
    }
  }

  onMount(() => {
    // Auto-focus the find input when the bar appears
    findInput?.focus();

    // If the editor has selected text, pre-fill the search query
    const view = editorStore.view;
    if (view) {
      const { from, to } = view.state.selection;
      if (from !== to) {
        const selectedText = view.state.doc.textBetween(from, to);
        if (selectedText.length > 0 && selectedText.length < 200) {
          query = selectedText;
        }
      }
    }

    return () => {
      // Clear highlights when the component is destroyed
      const v = editorStore.view;
      if (v) {
        dispatchClear(v);
      }
    };
  });
</script>

<div class="find-replace-bar">
  <div class="find-row">
    <input
      bind:this={findInput}
      bind:value={query}
      class="find-input"
      placeholder="Find..."
      onkeydown={handleFindKeydown}
      spellcheck={false}
    />
    <span class="match-info" class:hint={query.length === 0} class:empty={query.length > 0 && matchCount === 0}>
      {#if matchCount > 0}
        {currentIndex + 1} of {matchCount}
      {:else if query.length > 0}
        No matches
      {:else}
        Type to search
      {/if}
    </span>
    <button class="bar-btn" onclick={prev} title="Previous match (Shift+Enter)" disabled={matchCount === 0}>
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none"><path d="M2 8L6 4L10 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
    </button>
    <button class="bar-btn" onclick={next} title="Next match (Enter)" disabled={matchCount === 0}>
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none"><path d="M2 4L6 8L10 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
    </button>
    <button class="bar-btn" class:active={caseSensitive} onclick={toggleCase} title="Match case">Aa</button>
    <button class="bar-btn btn-close" onclick={close} title="Close (Esc)">&times;</button>
  </div>
  {#if mode === 'replace'}
    <div class="replace-row">
      <input
        bind:value={replacement}
        class="find-input"
        placeholder="Replace..."
        onkeydown={handleReplaceKeydown}
        spellcheck={false}
      />
      <button class="bar-btn replace-action" onclick={handleReplace} disabled={matchCount === 0} title="Replace current match (Enter)">Replace</button>
      <button class="bar-btn replace-action" onclick={handleReplaceAll} disabled={matchCount === 0} title="Replace every match">Replace All</button>
    </div>
  {/if}
</div>

<style>
  .find-replace-bar {
    position: absolute;
    top: 0;
    right: 16px;
    z-index: 50;
    background: var(--surface-elevated);
    border: 1px solid var(--border-subtle);
    border-top: none;
    border-radius: 0 0 8px 8px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    box-shadow: 0 4px 16px var(--shadow-medium);
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 12px;
  }

  .find-row,
  .replace-row {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .find-input {
    width: 200px;
    height: 26px;
    padding: 0 8px;
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    background: var(--surface-base);
    color: var(--text-primary);
    font-size: 12px;
    font-family: system-ui, -apple-system, sans-serif;
    outline: none;
    transition: border-color 120ms ease;
  }

  .find-input:focus {
    border-color: var(--accent);
  }

  .find-input::placeholder {
    color: var(--text-muted);
  }

  .match-info {
    min-width: 76px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 11px;
    white-space: nowrap;
    user-select: none;
    font-variant-numeric: tabular-nums;
  }

  /* Soft placeholder when there's no query yet — shouldn't read as a
     result, just as a hint. */
  .match-info.hint {
    color: var(--text-muted);
    font-style: italic;
  }

  .match-info.empty {
    color: var(--text-muted);
  }

  .bar-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 26px;
    min-width: 26px;
    padding: 0 6px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 100ms, color 100ms;
    white-space: nowrap;
  }

  .bar-btn:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .bar-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .bar-btn.active {
    color: var(--accent);
    background: var(--accent-muted);
  }

  .btn-close {
    font-size: 16px;
    font-weight: 300;
  }

  .replace-action {
    font-size: 11px;
    padding: 0 8px;
  }
</style>
