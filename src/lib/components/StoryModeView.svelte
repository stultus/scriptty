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

  // Derived story data from document store
  let idea = $derived(documentStore.document?.story.idea ?? '');
  let synopsis = $derived(documentStore.document?.story.synopsis ?? '');
  let treatment = $derived(documentStore.document?.story.treatment ?? '');
  let narrative = $derived(documentStore.document?.story.narrative ?? '');

  function updateField(field: 'idea' | 'synopsis' | 'treatment' | 'narrative', value: string) {
    if (documentStore.document) {
      documentStore.document.story[field] = value;
      documentStore.markDirty();
    }
  }

  // Word count across all story sections
  let wordCount = $derived(() => {
    const text = [idea, synopsis, treatment, narrative].join(' ').trim();
    if (!text) return 0;
    return text.split(/\s+/).length;
  });


  /**
   * Handle keydown on any story textarea — intercepts keys for Malayalam input
   * and Ctrl+Space mode toggle.
   */
  function handleKeydown(event: KeyboardEvent, field: 'idea' | 'synopsis' | 'treatment' | 'narrative') {
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
      <div class="story-section">
        <h2 class="section-title">Idea</h2>
        <textarea
          class="story-textarea"
          placeholder="The core premise. One to three lines."
          value={idea}
          oninput={(e: Event) => updateField('idea', (e.target as HTMLTextAreaElement).value)}
          onkeydown={(e: KeyboardEvent) => handleKeydown(e, 'idea')}
          onfocus={(e: FocusEvent) => { activeTextarea = e.target as HTMLTextAreaElement; }}
        ></textarea>
      </div>

      <div class="story-section">
        <h2 class="section-title">Synopsis</h2>
        <textarea
          class="story-textarea"
          placeholder="The full story in prose — beginning, middle, end. 300–800 words."
          value={synopsis}
          oninput={(e: Event) => updateField('synopsis', (e.target as HTMLTextAreaElement).value)}
          onkeydown={(e: KeyboardEvent) => handleKeydown(e, 'synopsis')}
          onfocus={(e: FocusEvent) => { activeTextarea = e.target as HTMLTextAreaElement; }}
        ></textarea>
      </div>

      <div class="story-section">
        <h2 class="section-title">Treatment</h2>
        <textarea
          class="story-textarea"
          placeholder="Full narrative prose. Scene by scene, every beat written out."
          value={treatment}
          oninput={(e: Event) => updateField('treatment', (e.target as HTMLTextAreaElement).value)}
          onkeydown={(e: KeyboardEvent) => handleKeydown(e, 'treatment')}
          onfocus={(e: FocusEvent) => { activeTextarea = e.target as HTMLTextAreaElement; }}
        ></textarea>
      </div>

      <div class="story-section">
        <h2 class="section-title">Narrative</h2>
        <textarea
          class="story-textarea narrative-textarea"
          placeholder="Full-length story. Write freely — no formatting constraints, no element types."
          value={narrative}
          oninput={(e: Event) => updateField('narrative', (e.target as HTMLTextAreaElement).value)}
          onkeydown={(e: KeyboardEvent) => handleKeydown(e, 'narrative')}
          onfocus={(e: FocusEvent) => { activeTextarea = e.target as HTMLTextAreaElement; }}
        ></textarea>
      </div>
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

  .word-count {
    font-size: 11px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .story-editor {
    flex: 1;
    overflow-y: auto;
    background: var(--surface-base);
    padding: 40px 0;
  }

  .page {
    max-width: 680px;
    margin: 0 auto;
    background: var(--page-bg);
    border-radius: 2px;
    box-shadow: 0 4px 24px var(--page-shadow), 0 1px 4px var(--shadow-soft);
    padding: 60px 72px;
  }

  .story-section {
    margin-bottom: 32px;
  }

  .story-section:last-child {
    margin-bottom: 0;
  }

  .section-title {
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 11px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin: 0 0 8px;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .story-textarea {
    width: 100%;
    min-height: 80px;
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-on-page);
    background: transparent;
    border: none;
    font-family: var(--editor-font), system-ui, -apple-system, sans-serif;
    resize: none;
    box-sizing: border-box;
    padding: 0;
    field-sizing: content;
    overflow: hidden;
  }

  .story-textarea:focus {
    outline: none;
  }

  .story-textarea::placeholder {
    color: var(--text-muted);
    font-style: italic;
  }

  .narrative-textarea {
    min-height: 400px;
  }

</style>
