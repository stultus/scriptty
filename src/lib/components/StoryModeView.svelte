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

  // Derived story data — reads from the active story (series → active episode,
  // film → top-level) so Story Mode edits stay in sync with the editor.
  let idea = $derived(documentStore.activeStory?.idea ?? '');
  let synopsis = $derived(documentStore.activeStory?.synopsis ?? '');
  let treatment = $derived(documentStore.activeStory?.treatment ?? '');
  let narrative = $derived(documentStore.activeStory?.narrative ?? '');

  function fieldValue(field: StoryField): string {
    if (field === 'idea') return idea;
    if (field === 'synopsis') return synopsis;
    if (field === 'treatment') return treatment;
    return narrative;
  }

  function updateField(field: StoryField, value: string) {
    const s = documentStore.activeStory;
    if (s) {
      s[field] = value;
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
    <div class="page" style="--editor-font-ml: '{fontFamily}'">
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

  /* The editor area scrolls vertically — the page grows with the textarea
     content (via field-sizing on the textarea), and longer entries push
     the page taller while the outer container handles the scroll. Mirrors
     the script view's "page grows down, outer scrolls" pattern. */
  .story-editor {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--surface-base);
    padding: 40px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .page {
    width: 100%;
    max-width: 680px;
    /* Fill at least the visible canvas so an empty tab still feels like
       a page, but allow growth past the viewport (the outer scrolls). */
    min-height: calc(100vh - 80px - 28px);
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
    box-sizing: border-box;
  }

  .tab-bar {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border-subtle);
    margin: 0 0 24px;
    padding-bottom: 0;
  }

  /* Tab labels act as the title-page-like heading of Story view — Courier
     Prime bold, wider tracking. Matches the display rhythm of scene headings
     in the editor so both pages read as the same typographic system. */
  .tab {
    border: none;
    background: transparent;
    padding: 8px 14px;
    margin-bottom: -1px;
    font-family: var(--editor-font-en), var(--ui-font);
    /* Tabs are Latin-only labels, so no Malayalam fallback is required. */
    font-size: 12px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.1em;
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

  /* No flex:1 — the textarea sizes itself to content via `field-sizing`,
     and the section just wraps it. The page's own min-height keeps an
     empty tab feeling page-shaped without forcing the textarea to fill it. */
  .story-section {
    display: flex;
    flex-direction: column;
  }

  /* `field-sizing: content` makes the textarea grow with its own text
     instead of carrying an internal scrollbar — combined with the outer
     scroll on .story-editor, the page grows downward as the writer types.
     `overflow-wrap: anywhere` ensures long Malayalam words / URLs break
     onto a new line instead of forcing horizontal overflow on the page.
     Body font — Courier Prime first for Latin, Malayalam font falls back
     per-glyph so mixed-script prose still shapes correctly. Monospace
     generic sits after the Malayalam font so it can't swallow Malayalam
     glyphs via a system-monospace notdef. */
  .story-textarea {
    display: block;
    width: 100%;
    field-sizing: content;
    /* Two empty lines of room when there's no content yet — gives the
       placeholder space to breathe without forcing the textarea to fill
       the whole page (which previously made the cursor land outside the
       page on long text). The page's own min-height handles "feels like
       a page when empty" instead. */
    min-height: 2lh;
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-on-page);
    background: transparent;
    border: none;
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
    resize: none;
    box-sizing: border-box;
    padding: 0;
    overflow: hidden;
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .story-textarea:focus {
    outline: none;
  }

  .story-textarea::placeholder {
    color: var(--text-muted);
    font-style: italic;
  }
</style>
