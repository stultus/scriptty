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

  /** Auto-grow the textarea to its content. We don't use the new
   *  `field-sizing: content` CSS because it isn't reliably honored in
   *  every Tauri WebView build — the textarea would extend past the page
   *  card. Setting the height to scrollHeight after each input gives us
   *  the same growth behavior with universal support. */
  function autoGrow(el: HTMLTextAreaElement) {
    el.style.height = 'auto';
    el.style.height = `${el.scrollHeight}px`;
  }

  /** Action: bind to the textarea so it auto-sizes on mount, on tab
   *  switch (the textarea unmounts/remounts), and after the value updates. */
  function autoGrowAction(node: HTMLTextAreaElement) {
    // Initial size after mount + after the value prop is applied. */
    queueMicrotask(() => autoGrow(node));
    return {};
  }

  // Tab state — persisted to localStorage so a user returning to Story Mode
  // lands on the section they were last editing.
  type StoryField = 'idea' | 'synopsis' | 'treatment' | 'narrative';
  // marker is the editorial numeral shown above each tab label
  // (`№ 01 · Idea`). Stable per-section so the same numeral travels with
  // the section across renders and active-state changes.
  const tabs: { id: StoryField; label: string; marker: string; placeholder: string }[] = [
    { id: 'idea',      label: 'Idea',      marker: '№ 01', placeholder: 'The core premise. One to three lines.' },
    { id: 'synopsis',  label: 'Synopsis',  marker: '№ 02', placeholder: 'The full story in prose — beginning, middle, end. 300–800 words.' },
    { id: 'treatment', label: 'Treatment', marker: '№ 03', placeholder: 'Full narrative prose. Scene by scene, every beat written out.' },
    { id: 'narrative', label: 'Narrative', marker: '№ 04', placeholder: 'Full-length story. Write freely — no formatting constraints, no element types.' },
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
        autoGrow(ta);
      }
    }
  }
</script>

<div class="story-mode">
  <div class="story-editor">
    <div class="story-editor-inner">
    <div class="page" style="--editor-font-ml: '{fontFamily}'">
      <!-- Editorial masthead — section-page treatment for Story Mode.
           Matches the vocabulary used in the rest of the app: tracked-
           caps eyebrow + Courier title. Title is the active section
           (Idea / Synopsis / Treatment / Narrative) so the writer
           always sees which prose surface they're on. -->
      <header class="story-masthead">
        <div class="mh-eyebrow" aria-hidden="true">
          <span class="mh-rule"></span>
          <span>The Story</span>
        </div>
        <h1 class="mh-title story-title">{tabs.find((t) => t.id === activeTab)?.label ?? 'Story'}</h1>
      </header>
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
            <span class="mh-marker tab-marker" aria-hidden="true">{tab.marker}</span>
            <span class="tab-label">{tab.label}</span>
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
              use:autoGrowAction
              oninput={(e: Event) => {
                const ta = e.target as HTMLTextAreaElement;
                updateField(tab.id, ta.value);
                autoGrow(ta);
              }}
              onkeydown={(e: KeyboardEvent) => handleKeydown(e, tab.id)}
              onfocus={(e: FocusEvent) => { activeTextarea = e.target as HTMLTextAreaElement; }}
            ></textarea>
          </div>
        {/if}
      {/each}
    </div>
    </div>
  </div>
</div>

<style>
  /* Mirrors the writing view's geometry exactly — see Editor.svelte
     `.editor-wrapper / .editor-scroll / .editor-with-annotations /
     .editor-container / .ProseMirror`. Story view is a textarea instead
     of a contenteditable, but the page card and scroll behavior should
     feel identical, so the writer's eye doesn't lose its place when
     switching tabs. */
  .story-mode {
    position: relative;
    width: 100%;
    height: 100%;
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--surface-base);
    overflow: hidden;
  }

  .story-editor {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--surface-base);
    padding: 40px 0;
  }

  /* Flex-centered page wrapper — `min-height: 100%` keeps the page
     vertically anchored to the visible scroll area when content is short. */
  .story-editor-inner {
    position: relative;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    min-height: 100%;
    padding: 0 20px;
  }

  /* Fixed 680px wide column matching the writing view. */
  .page {
    flex: 0 0 680px;
    max-width: 680px;
    min-width: 0;
    /* Same generous min-height as .ProseMirror so the page feels like a
       full sheet from the start; grows naturally as the textarea grows. */
    min-height: 2000px;
    background-color: var(--page-bg);
    background-image: var(--page-grain);
    background-repeat: repeat;
    background-size: 240px 240px;
    border-radius: 2px;
    box-shadow:
      inset 0 1px 0 var(--page-edge-highlight),
      0 1px 2px var(--page-shadow-close),
      0 12px 32px var(--page-shadow);
    /* 60vh bottom padding mirrors .ProseMirror — the cursor always has
       a generous breathing margin below; typing never reaches the page edge. */
    padding: 60px 72px 60vh;
    box-sizing: border-box;
  }

  /* Editorial masthead at the top of the Story page. Centered
     frontispiece block — eyebrow above the section title — followed
     by the existing tab bar acting as the section navigator. */
  .story-masthead {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    margin-bottom: 22px;
    padding-bottom: 18px;
    border-bottom: 1px solid var(--border-subtle);
    text-align: center;
    color: var(--text-on-page);
  }

  .story-title {
    margin: 0;
    font-size: 30px;
    line-height: 1;
    color: var(--text-on-page);
  }

  .tab-bar {
    display: flex;
    gap: 18px;
    margin: 0 0 24px;
    padding-bottom: 0;
    justify-content: center;
  }

  /* Tabs are department blocks now — small Courier numeral on top
     (the .mh-marker utility), then the section label below. Borrows the
     marketing-site's `№ 0X · Topic` editorial vocabulary. */
  .tab {
    display: inline-flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    border: none;
    background: transparent;
    padding: 8px 12px;
    margin-bottom: -1px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color 120ms ease, border-color 120ms ease;
  }

  .tab-marker {
    /* Inactive tabs read as quiet — keep the marker hue muted until the
       tab activates so the row doesn't become a wall of warm dots. */
    color: var(--text-muted);
    transition: color 120ms ease;
  }

  .tab-label {
    font-family: var(--editor-font-en), var(--ui-font);
    font-size: 12px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    line-height: 1;
    transition: color 120ms ease;
  }

  /* Use --text-on-page so hover stays legible on the cream page background
     in dark mode — --text-primary is off-white and disappears there. */
  .tab:hover .tab-label {
    color: var(--text-on-page);
  }

  .tab.active {
    border-bottom-color: var(--accent);
  }

  .tab.active .tab-label {
    color: var(--accent);
  }

  /* Active tab promotes the marker into its true marker color — the
     numeral pops, signalling the live section. */
  .tab.active .tab-marker {
    color: var(--marker-color);
  }

  .story-section {
    display: block;
  }

  /* JS-driven auto-grow (autoGrow / autoGrowAction) keeps the textarea
     sized to its content. The textarea sits inside the page's padding,
     so long entries push the page taller, the outer scrolls, and the
     cursor stays inside the page card. `overflow-wrap: anywhere` breaks
     unbreakable runs (long Malayalam conjuncts, URLs) so the page never
     produces a horizontal scrollbar.
     Body font — Courier Prime first for Latin, Malayalam font falls back
     per-glyph so mixed-script prose still shapes correctly. */
  .story-textarea {
    display: block;
    width: 100%;
    /* Two lines of placeholder breathing room before any input. */
    min-height: 2.8em;
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
