<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { InputModeManager } from '$lib/editor/input/InputModeManager';

  // Map the font setting slug to a CSS font-family name
  let fontFamily = $derived(
    documentStore.currentFont === 'manjari' ? 'Manjari' : 'Noto Sans Malayalam'
  );

  // Get the shared input manager singleton
  const inputManager = InputModeManager.getInstance();

  // Input mode UI state — synced with the singleton
  let currentMode = $state<'ENGLISH' | 'MALAYALAM'>(inputManager.isMalayalam ? 'MALAYALAM' : 'ENGLISH');

  // Track which textarea is currently focused for Malayalam input
  let activeTextarea = $state<HTMLTextAreaElement | null>(null);

  // Tab state — persisted to localStorage so a user returning to Story Mode
  // lands on the section they were last editing.
  type StoryField = 'idea' | 'synopsis' | 'treatment' | 'narrative';
  const tabs: { id: StoryField; label: string; placeholder: string }[] = [
    { id: 'idea', label: 'Idea', placeholder: 'The core premise. One to three lines.' },
    { id: 'synopsis', label: 'Synopsis', placeholder: 'The full story in prose — beginning, middle, end. 300–800 words.' },
    { id: 'treatment', label: 'Treatment', placeholder: 'Full narrative prose. Scene by scene, every beat written out.' },
    { id: 'narrative', label: 'Narrative', placeholder: 'Full-length story. Write freely — no formatting constraints, no element types.' },
  ];

  function initialTab(): StoryField {
    if (typeof localStorage === 'undefined') return 'idea';
    const stored = localStorage.getItem('scriptty-story-tab');
    if (stored === 'idea' || stored === 'synopsis' || stored === 'treatment' || stored === 'narrative') {
      return stored;
    }
    return 'idea';
  }

  let activeTab = $state<StoryField>(initialTab());

  $effect(() => {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('scriptty-story-tab', activeTab);
    }
  });

  // Derived story data from document store
  let idea = $derived(documentStore.document?.story.idea ?? '');
  let synopsis = $derived(documentStore.document?.story.synopsis ?? '');
  let treatment = $derived(documentStore.document?.story.treatment ?? '');
  let narrative = $derived(documentStore.document?.story.narrative ?? '');

  function fieldValue(field: StoryField): string {
    if (field === 'idea') return idea;
    if (field === 'synopsis') return synopsis;
    if (field === 'treatment') return treatment;
    return narrative;
  }

  function updateField(field: StoryField, value: string) {
    if (documentStore.document) {
      documentStore.document.story[field] = value;
      documentStore.markDirty();
    }
  }

  /**
   * Handle keydown on any story textarea — intercepts keys for Malayalam input
   * and Ctrl+Space mode toggle.
   */
  function handleKeydown(event: KeyboardEvent, field: StoryField) {
    // Ctrl+Space — toggle English/Malayalam
    if (event.ctrlKey && event.code === 'Space') {
      event.preventDefault();
      const isNowMalayalam = inputManager.toggle();
      currentMode = isNowMalayalam ? 'MALAYALAM' : 'ENGLISH';
      return;
    }

    // Don't intercept keys with Cmd/Ctrl modifiers (save, undo, select all, etc.)
    if (event.metaKey || event.ctrlKey) return;

    // Reset Mozhi buffer on word boundaries and navigation keys
    if (['Space', 'Enter', 'Backspace', 'Delete', 'ArrowLeft', 'ArrowRight',
         'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.code)) {
      inputManager.resetMozhi();
      return;
    }

    // Malayalam input processing — only for single printable characters
    if (inputManager.isMalayalam && event.key.length === 1 && !event.altKey) {
      const result = inputManager.processKey(event.key);
      if (result !== null && activeTextarea) {
        event.preventDefault();

        const ta = activeTextarea;
        const start = ta.selectionStart;
        const end = ta.selectionEnd;
        const value = ta.value;

        const deleteFrom = start - result.deleteBack;
        const newValue = value.substring(0, deleteFrom) + result.text + value.substring(end);

        ta.value = newValue;
        const newCursorPos = deleteFrom + result.text.length;
        ta.selectionStart = newCursorPos;
        ta.selectionEnd = newCursorPos;

        updateField(field, newValue);
      }
    }
  }
</script>

<div class="story-mode">
  <div class="story-editor">
    <div class="page" style="--editor-font: '{fontFamily}'">
      <div class="tab-bar" role="tablist" aria-label="Story sections">
        {#each tabs as tab}
          <button
            type="button"
            role="tab"
            class="tab"
            class:active={activeTab === tab.id}
            aria-selected={activeTab === tab.id}
            onclick={() => { activeTab = tab.id; }}
          >
            {tab.label}
          </button>
        {/each}
      </div>
      {#each tabs as tab (tab.id)}
        {#if activeTab === tab.id}
          <div class="story-section" role="tabpanel">
            <textarea
              class="story-textarea"
              placeholder={tab.placeholder}
              value={fieldValue(tab.id)}
              oninput={(e: Event) => updateField(tab.id, (e.target as HTMLTextAreaElement).value)}
              onkeydown={(e: KeyboardEvent) => handleKeydown(e, tab.id)}
              onfocus={(e: FocusEvent) => { activeTextarea = e.target as HTMLTextAreaElement; }}
            ></textarea>
          </div>
        {/if}
      {/each}
    </div>
  </div>
</div>

<style>
  .story-mode {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--surface-base);
    overflow: hidden;
  }

  /* The editor area doesn't scroll — the page is sized to fit the viewport
     and the active textarea scrolls internally. This keeps every tab
     rendering on an identically sized page regardless of how much text
     is stored in any individual section. */
  .story-editor {
    flex: 1;
    overflow: hidden;
    background: var(--surface-base);
    padding: 40px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .page {
    width: 100%;
    max-width: 680px;
    background-color: var(--page-bg);
    background-image: var(--page-grain);
    background-repeat: repeat;
    background-size: 240px 240px;
    border-radius: 2px;
    box-shadow:
      inset 0 1px 0 var(--page-edge-highlight),
      0 1px 2px var(--page-shadow-close),
      0 12px 32px var(--page-shadow);
    padding: 32px 72px 60px;
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    box-sizing: border-box;
  }

  .tab-bar {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border-subtle);
    margin: 0 0 24px;
    padding-bottom: 0;
  }

  .tab {
    border: none;
    background: transparent;
    padding: 8px 14px;
    margin-bottom: -1px;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color 120ms ease, border-color 120ms ease;
  }

  /* Use --text-on-page so hover stays legible on the cream page background
     in dark mode — --text-primary is off-white and disappears there. */
  .tab:hover {
    color: var(--text-on-page);
  }

  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .story-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  /* No field-sizing: the textarea occupies the fixed page body and scrolls
     its own content. All four tabs therefore render on an identical page. */
  .story-textarea {
    width: 100%;
    flex: 1;
    min-height: 0;
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-on-page);
    background: transparent;
    border: none;
    font-family: var(--editor-font), system-ui, -apple-system, sans-serif;
    resize: none;
    box-sizing: border-box;
    padding: 0;
    overflow-y: auto;
  }

  .story-textarea:focus {
    outline: none;
  }

  .story-textarea::placeholder {
    color: var(--text-muted);
    font-style: italic;
  }
</style>
