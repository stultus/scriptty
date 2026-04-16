<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';

  // Track which sections are collapsed
  let ideaOpen = $state(true);
  let synopsisOpen = $state(true);
  let treatmentOpen = $state(true);
  let narrativeOpen = $state(false);

  // Derived story data from document store
  let idea = $derived(documentStore.document?.story.idea ?? '');
  let synopsis = $derived(documentStore.document?.story.synopsis ?? '');
  let treatment = $derived(documentStore.document?.story.treatment ?? '');
  let narrative = $derived(documentStore.document?.story.narrative ?? '');

  function updateIdea(event: Event) {
    const value = (event.target as HTMLTextAreaElement).value;
    if (documentStore.document) {
      documentStore.document.story.idea = value;
      documentStore.markDirty();
    }
  }

  function updateSynopsis(event: Event) {
    const value = (event.target as HTMLTextAreaElement).value;
    if (documentStore.document) {
      documentStore.document.story.synopsis = value;
      documentStore.markDirty();
    }
  }

  function updateTreatment(event: Event) {
    const value = (event.target as HTMLTextAreaElement).value;
    if (documentStore.document) {
      documentStore.document.story.treatment = value;
      documentStore.markDirty();
    }
  }

  function updateNarrative(event: Event) {
    const value = (event.target as HTMLTextAreaElement).value;
    if (documentStore.document) {
      documentStore.document.story.narrative = value;
      documentStore.markDirty();
    }
  }
</script>

<div class="story-panel">
  <!-- Idea / Logline -->
  <div class="section">
    <button class="section-header" onclick={() => { ideaOpen = !ideaOpen; }}>
      <span class="chevron" class:open={ideaOpen}>&#9654;</span>
      <span class="section-title">Idea</span>
    </button>
    {#if ideaOpen}
      <div class="section-body">
        <textarea
          class="story-textarea idea-textarea"
          placeholder="The core premise. One to three lines."
          value={idea}
          oninput={updateIdea}
          rows="5"
        ></textarea>
      </div>
    {/if}
  </div>

  <!-- Synopsis -->
  <div class="section">
    <button class="section-header" onclick={() => { synopsisOpen = !synopsisOpen; }}>
      <span class="chevron" class:open={synopsisOpen}>&#9654;</span>
      <span class="section-title">Synopsis</span>
    </button>
    {#if synopsisOpen}
      <div class="section-body">
        <textarea
          class="story-textarea synopsis-textarea"
          placeholder="The full story in prose — beginning, middle, end. 300–800 words."
          value={synopsis}
          oninput={updateSynopsis}
          rows="12"
        ></textarea>
      </div>
    {/if}
  </div>

  <!-- Treatment -->
  <div class="section">
    <button class="section-header" onclick={() => { treatmentOpen = !treatmentOpen; }}>
      <span class="chevron" class:open={treatmentOpen}>&#9654;</span>
      <span class="section-title">Treatment</span>
    </button>
    {#if treatmentOpen}
      <div class="section-body treatment-body">
        <textarea
          class="story-textarea treatment-textarea"
          placeholder="Full narrative prose. Scene by scene, every beat written out."
          value={treatment}
          oninput={updateTreatment}
        ></textarea>
      </div>
    {/if}
  </div>

  <!-- Narrative -->
  <div class="section">
    <button class="section-header" onclick={() => { narrativeOpen = !narrativeOpen; }}>
      <span class="chevron" class:open={narrativeOpen}>&#9654;</span>
      <span class="section-title">Narrative</span>
      <span class="section-hint">
        <kbd>⌘⇧L</kbd>
        <span class="hint-label">full screen</span>
      </span>
    </button>
    {#if narrativeOpen}
      <div class="section-body treatment-body">
        <textarea
          class="story-textarea treatment-textarea"
          placeholder="Full-length story. Use Story Mode (Cmd+Shift+L) for a distraction-free writing experience."
          value={narrative}
          oninput={updateNarrative}
        ></textarea>
      </div>
    {/if}
  </div>
</div>

<style>
  .story-panel {
    padding: 12px;
    height: 100%;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .section {
    display: flex;
    flex-direction: column;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 4px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: background 120ms ease;
  }

  .section-header:hover {
    background: var(--surface-hover);
  }

  .chevron {
    font-size: 8px;
    color: var(--text-muted);
    transition: transform 150ms ease;
    display: inline-block;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .section-title {
    color: var(--text-muted);
  }

  .section-hint {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    text-transform: none;
    letter-spacing: normal;
  }

  .section-hint kbd {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    font-weight: 500;
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 3px;
    padding: 1px 4px;
    color: var(--text-secondary);
  }

  .hint-label {
    font-size: 10px;
    font-weight: 400;
    color: var(--text-secondary);
  }

  .section-body {
    padding: 4px 0 8px;
  }

  .treatment-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .story-textarea {
    width: 100%;
    padding: 8px 10px;
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-primary);
    background: var(--surface-hover);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    font-family: system-ui, -apple-system, sans-serif;
    resize: vertical;
    box-sizing: border-box;
    transition: border-color 120ms ease;
  }

  .story-textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  .story-textarea::placeholder {
    color: var(--text-muted);
    font-style: italic;
  }

  .idea-textarea {
    resize: none;
  }

  .treatment-textarea {
    flex: 1;
    min-height: 200px;
    resize: none;
  }
</style>
