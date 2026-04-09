<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import { writeFile } from '@tauri-apps/plugin-fs';
  import { documentStore } from '$lib/stores/documentStore.svelte';

  let { open = $bindable(false) } = $props<{ open: boolean }>();

  // Export options state
  let includeTitlePage = $state(true);
  let includeSynopsis = $state(false);
  let includeTreatment = $state(false);
  let includeScreenplay = $state(true);
  let includeNarrative = $state(false);
  let includeSceneCards = $state(false);
  let format = $state<'hollywood' | 'indian'>('hollywood');
  let pageBreakAfterScene = $state(false);

  let exporting = $state(false);
  let exportingFountain = $state(false);
  let exportingPlaintext = $state(false);
  let errorMessage = $state('');

  // Derived: check if synopsis/treatment have content
  let hasSynopsis = $derived((documentStore.document?.story.synopsis ?? '').trim().length > 0);
  let hasTreatment = $derived((documentStore.document?.story.treatment ?? '').trim().length > 0);
  let hasNarrative = $derived((documentStore.document?.story.narrative ?? '').trim().length > 0);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      open = false;
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      open = false;
    }
  }

  /** Build scene cards data with auto-populated fields derived from the screenplay */
  function buildSceneCardsData(): Array<Record<string, unknown>> {
    const doc = documentStore.document;
    if (!doc || !doc.content) return [];

    const content = doc.content as {
      type?: string;
      content?: Array<{
        type?: string;
        content?: Array<{ text?: string }>;
      }>;
    };
    if (!content.content) return [];

    const cards: Array<Record<string, unknown>> = [];
    const startNum = doc.settings?.scene_number_start ?? 1;
    let sceneNumber = startNum - 1;
    let currentSceneCharacters: string[] = [];
    let currentSceneCharCount = 0;

    for (let i = 0; i < content.content.length; i++) {
      const node = content.content[i];

      if (node.type === 'scene_heading') {
        // If there was a previous scene, finalize it
        if (sceneNumber >= startNum) {
          finalizeCard(cards, sceneNumber, currentSceneCharacters, currentSceneCharCount);
        }

        sceneNumber++;
        currentSceneCharacters = [];
        currentSceneCharCount = 0;

        const headingText = (node.content ?? []).map((c) => c.text ?? '').join('');
        const { location, time } = parseSceneHeading(headingText);

        // Find matching manually-written scene card data
        const manualCard = doc.scene_cards.find((c) => c.scene_index === sceneNumber - 1);

        cards.push({
          scene_number: sceneNumber,
          heading: headingText,
          location,
          time,
          characters: '',
          page_estimate: '',
          description: manualCard?.description ?? '',
          shoot_notes: manualCard?.shoot_notes ?? '',
        });
      } else if (node.type === 'character') {
        const charName = (node.content ?? []).map((c) => c.text ?? '').join('').trim();
        if (charName && !currentSceneCharacters.includes(charName)) {
          currentSceneCharacters.push(charName);
        }
      }

      // Count characters for page estimate
      if (node.content) {
        currentSceneCharCount += node.content.reduce((sum, c) => sum + (c.text ?? '').length, 0);
      }
    }

    // Finalize last scene
    if (sceneNumber >= startNum) {
      finalizeCard(cards, sceneNumber, currentSceneCharacters, currentSceneCharCount);
    }

    return cards;
  }

  function finalizeCard(
    cards: Array<Record<string, unknown>>,
    sceneNumber: number,
    characters: string[],
    charCount: number
  ) {
    const cardIndex = sceneNumber - 1;
    if (cardIndex < cards.length) {
      cards[cardIndex].characters = characters.join(', ');
      // Rough page estimate: ~3000 chars per page
      const pages = Math.max(0.1, charCount / 3000);
      cards[cardIndex].page_estimate = `~${pages.toFixed(1)} pages`;
    }
  }

  /** Parse INT./EXT., location, and time from a scene heading */
  function parseSceneHeading(heading: string): { location: string; time: string } {
    // Typical format: "INT. COFFEE SHOP - MORNING"
    const match = heading.match(/^(?:INT\.|EXT\.|INT\.\/EXT\.)\s*(.+?)\s*-\s*(.+)$/i);
    if (match) {
      return { location: match[1].trim(), time: match[2].trim() };
    }
    return { location: heading, time: '' };
  }

  async function handlePlaintextExport() {
    if (!documentStore.document) return;
    exportingPlaintext = true;
    errorMessage = '';

    try {
      const plaintext = await invoke<string>('export_plaintext', {
        document: documentStore.document,
      });

      const title = documentStore.document.meta.title || 'screenplay';
      const path = await save({
        defaultPath: `${title}.txt`,
        filters: [{ name: 'Plain Text', extensions: ['txt'] }],
      });

      if (!path) {
        exportingPlaintext = false;
        return;
      }

      const encoder = new TextEncoder();
      await writeFile(path, encoder.encode(plaintext));
      open = false;
    } catch (e) {
      console.error('[ExportModal] Plain text export failed:', e);
      errorMessage = String(e);
    } finally {
      exportingPlaintext = false;
    }
  }

  async function handleFountainExport() {
    if (!documentStore.document) return;
    exportingFountain = true;
    errorMessage = '';

    try {
      const fountainText = await invoke<string>('export_fountain', {
        document: documentStore.document,
      });

      const title = documentStore.document.meta.title || 'screenplay';
      const path = await save({
        defaultPath: `${title}.fountain`,
        filters: [{ name: 'Fountain', extensions: ['fountain'] }],
      });

      if (!path) {
        exportingFountain = false;
        return;
      }

      // Write the Fountain text as UTF-8 bytes
      const encoder = new TextEncoder();
      await writeFile(path, encoder.encode(fountainText));
      open = false;
    } catch (e) {
      console.error('[ExportModal] Fountain export failed:', e);
      errorMessage = String(e);
    } finally {
      exportingFountain = false;
    }
  }

  async function handleExport() {
    if (!documentStore.document) return;
    exporting = true;
    errorMessage = '';

    try {
      const sceneCardsData = includeSceneCards ? buildSceneCardsData() : [];

      const bytes = await invoke<number[]>('export_combined_pdf', {
        document: documentStore.document,
        options: {
          include_title_page: includeTitlePage,
          include_synopsis: includeSynopsis,
          include_treatment: includeTreatment,
          include_screenplay: includeScreenplay,
          include_narrative: includeNarrative,
          include_scene_cards: includeSceneCards,
          format,
          page_break_after_scene: pageBreakAfterScene,
          scene_cards_data: sceneCardsData,
        },
      });

      const path = await save({
        defaultPath: 'screenplay.pdf',
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });

      if (!path) {
        exporting = false;
        return;
      }

      await writeFile(path, new Uint8Array(bytes));
      open = false;
    } catch (e) {
      console.error('[ExportModal] Export failed:', e);
      errorMessage = String(e);
    } finally {
      exporting = false;
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="modal-backdrop" onclick={handleBackdropClick} onkeydown={handleKeydown} role="dialog" aria-modal="true" tabindex="-1">
    <div class="modal-card">
      <div class="modal-header">
        <h2>Export Document</h2>
        <button class="btn-close" onclick={() => { open = false; }}>&times;</button>
      </div>

      <div class="section-label">What to include</div>
      <div class="checkbox-group">
        <label class="checkbox-row">
          <input type="checkbox" bind:checked={includeTitlePage} />
          <span>Title Page</span>
        </label>
        <label class="checkbox-row" class:disabled={!hasSynopsis}>
          <input type="checkbox" bind:checked={includeSynopsis} disabled={!hasSynopsis} />
          <span>Synopsis{!hasSynopsis ? ' (empty)' : ''}</span>
        </label>
        <label class="checkbox-row" class:disabled={!hasTreatment}>
          <input type="checkbox" bind:checked={includeTreatment} disabled={!hasTreatment} />
          <span>Treatment{!hasTreatment ? ' (empty)' : ''}</span>
        </label>
        <label class="checkbox-row">
          <input type="checkbox" bind:checked={includeScreenplay} />
          <span>Screenplay</span>
        </label>
        <label class="checkbox-row" class:disabled={!hasNarrative}>
          <input type="checkbox" bind:checked={includeNarrative} disabled={!hasNarrative} />
          <span>Narrative{!hasNarrative ? ' (empty)' : ''}</span>
        </label>
        <label class="checkbox-row">
          <input type="checkbox" bind:checked={includeSceneCards} />
          <span>Scene Cards</span>
        </label>
      </div>

      {#if includeScreenplay}
        <div class="section-label">Screenplay Format</div>
        <div class="radio-group">
          <label class="radio-row">
            <input type="radio" name="format" value="hollywood" bind:group={format} />
            <span>Hollywood (single column)</span>
          </label>
          <label class="radio-row">
            <input type="radio" name="format" value="indian" bind:group={format} />
            <span>Indian (two column)</span>
          </label>
        </div>

        <div class="section-label">Layout</div>
        <div class="checkbox-group">
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={pageBreakAfterScene} />
            <span>Page break after each scene</span>
          </label>
        </div>
      {/if}

      {#if errorMessage}
        <p class="error-message">{errorMessage}</p>
      {/if}

      <div class="modal-footer">
        <button class="btn-ghost" onclick={() => { open = false; }}>Cancel</button>
        <button class="btn-secondary" onclick={handlePlaintextExport} disabled={exportingPlaintext}>
          {exportingPlaintext ? 'Exporting...' : 'Plain Text'}
        </button>
        <button class="btn-secondary" onclick={handleFountainExport} disabled={exportingFountain}>
          {exportingFountain ? 'Exporting...' : 'Fountain'}
        </button>
        <button class="btn-primary" onclick={handleExport} disabled={exporting}>
          {exporting ? 'Exporting...' : 'Export PDF'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--backdrop);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-card {
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 12px;
    padding: 24px;
    width: 480px;
    max-width: 90vw;
    box-shadow: 0 8px 32px var(--shadow-heavy);
    animation: modal-in 150ms ease-out;
    font-family: system-ui, -apple-system, sans-serif;
  }

  @keyframes modal-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 15px;
    color: var(--text-primary);
    font-weight: 600;
  }

  .btn-close {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    font-size: 18px;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-close:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .section-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 8px;
  }

  .checkbox-group,
  .radio-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 20px;
  }

  .checkbox-row,
  .radio-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-primary);
    cursor: pointer;
    padding: 4px 0;
  }

  .checkbox-row.disabled {
    color: var(--text-muted);
    cursor: default;
  }

  .checkbox-row input,
  .radio-row input {
    accent-color: var(--accent);
  }

  .error-message {
    font-size: 12px;
    color: var(--error);
    margin: 0 0 12px;
    padding: 8px;
    background: rgba(192, 87, 74, 0.1);
    border-radius: 6px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn-ghost {
    height: 28px;
    padding: 0 12px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-ghost:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .btn-secondary {
    height: 28px;
    padding: 0 12px;
    border-radius: 6px;
    border: 1px solid var(--border-medium);
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-secondary:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .btn-secondary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-primary {
    height: 28px;
    padding: 0 12px;
    border-radius: 6px;
    border: none;
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 120ms ease;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
