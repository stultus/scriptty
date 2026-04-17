<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import { writeFile } from '@tauri-apps/plugin-fs';
  import { documentStore, type ScreenplayDocument } from '$lib/stores/documentStore.svelte';
  import { focusTrap } from '$lib/actions/focusTrap';

  let {
    open = $bindable(false),
    onEditMetadata = () => {},
  } = $props<{
    open: boolean;
    /** Opens the Metadata modal from the title-page section so writers can
     *  tweak title / tagline / credits without leaving the export flow. */
    onEditMetadata?: () => void;
  }>();

  // Export options state
  let includeTitlePage = $state(true);
  let includeSynopsis = $state(false);
  let includeTreatment = $state(false);
  let includeScreenplay = $state(true);
  let includeNarrative = $state(false);
  let includeSceneCards = $state(false);
  let format = $state<'hollywood' | 'indian'>('hollywood');
  let pageBreakAfterScene = $state(false);
  // Page numbering is off by default — opt in per export.
  let includePageNumbers = $state(false);
  // Seed from the document setting so "Show characters" in settings carries
  // over as the default export choice. User can still override per-export.
  let charactersBelowHeading = $state(
    documentStore.activeSettings?.show_characters_below_header ?? false
  );

  let exporting = $state(false);
  let exportingFountain = $state(false);
  let exportingPlaintext = $state(false);
  let errorMessage = $state('');

  // Scene range — lets a writer export a slice of the script for an actor /
  // technician (issue #17). Defaults to the full document; when enabled the
  // writer picks two scene numbers and the backend only sees the slice.
  let sceneRangeEnabled = $state(false);
  let sceneRangeFrom = $state(1);
  let sceneRangeTo = $state(1);

  // Export scope — only meaningful for series projects. "episode" exports the
  // currently-active episode as a standalone film (default, matches the
  // writer's most common need). "series" concatenates every episode into a
  // single combined export.
  let exportScope = $state<'episode' | 'series'>('episode');
  let isSeriesProject = $derived(documentStore.isSeries);

  // Derived: check if synopsis/treatment have content
  let hasSynopsis = $derived((documentStore.activeStory?.synopsis ?? '').trim().length > 0);
  let hasTreatment = $derived((documentStore.activeStory?.treatment ?? '').trim().length > 0);
  let hasNarrative = $derived((documentStore.activeStory?.narrative ?? '').trim().length > 0);

  // Count scene headings so the range inputs can clamp against the real
  // document size rather than an arbitrary cap.
  let sceneCount = $derived.by(() => {
    const content = documentStore.activeContent as { content?: Array<{ type?: string }> } | null;
    if (!content?.content) return 0;
    return content.content.filter((n) => n.type === 'scene_heading').length;
  });
  let firstSceneNumber = $derived(documentStore.activeSettings?.scene_number_start ?? 1);
  let lastSceneNumber = $derived(firstSceneNumber + Math.max(0, sceneCount - 1));

  // Keep the range bounded to what actually exists whenever the modal opens
  // or the document changes — spares us from validating on every click.
  $effect(() => {
    if (!open) return;
    sceneRangeFrom = firstSceneNumber;
    sceneRangeTo = lastSceneNumber;
  });

  // Any in-flight export locks dismissal so users don't click away thinking
  // the modal is stuck, and so we never leave a half-written file behind.
  let anyExporting = $derived(exporting || exportingFountain || exportingPlaintext);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && !anyExporting) {
      open = false;
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget && !anyExporting) {
      open = false;
    }
  }

  /**
   * Build scene cards data with auto-populated fields derived from the screenplay.
   *
   * Tracks two numbers per scene:
   *   - `sceneIndex` — 0-based document position (used for scene_cards[] lookups
   *     and for writing into the cards[] array). Independent of `scene_number_start`.
   *   - `scene_number` — the display number, offset by `scene_number_start` so
   *     co-writing files that cover scenes 5–14 render as "Scene 5".
   *
   * Keeping these separate fixes the misalignment that showed up when
   * `scene_number_start > 1`: the old code used display-number math for
   * array writes, which left every card's character list (and page
   * estimate) empty (issues #46, #54).
   *
   * Merges `extra_characters` from manual scene cards into the speaker
   * list so non-speaking characters the user added (background extras,
   * silent antagonists) still appear on the breakdown card.
   */
  function buildSceneCardsData(doc: ScreenplayDocument | null = documentStore.activeExportDocument): Array<Record<string, unknown>> {
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
    let sceneIndex = -1; // -1 means "no scene heading seen yet"
    let currentSceneCharacters: string[] = [];
    let currentSceneCharCount = 0;

    const finalizePreviousScene = () => {
      if (sceneIndex < 0 || sceneIndex >= cards.length) return;

      // Merge auto-detected speakers with user-supplied `extra_characters`
      // (non-speaking roles) — matches the editor's below-heading widget
      // behavior so the PDF and the editor show the same list.
      const manual = doc.scene_cards.find((c) => c.scene_index === sceneIndex);
      const extras = (manual?.extra_characters ?? '')
        .split(',')
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
      const merged = [...currentSceneCharacters];
      for (const name of extras) {
        if (!merged.includes(name)) merged.push(name);
      }

      cards[sceneIndex].characters = merged.join(', ');
      // Rough page estimate: ~3000 chars per page
      const pages = Math.max(0.1, currentSceneCharCount / 3000);
      cards[sceneIndex].page_estimate = `~${pages.toFixed(1)} pages`;
    };

    for (const node of content.content) {
      if (node.type === 'scene_heading') {
        finalizePreviousScene();
        sceneIndex++;
        currentSceneCharacters = [];
        currentSceneCharCount = 0;

        const headingText = (node.content ?? []).map((c) => c.text ?? '').join('');
        const { location, time } = parseSceneHeading(headingText);

        // scene_index is 0-based document position → matches how Rust stores
        // it on SceneCard. The display `scene_number` includes the startNum offset.
        const manualCard = doc.scene_cards.find((c) => c.scene_index === sceneIndex);

        cards.push({
          scene_number: startNum + sceneIndex,
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

    finalizePreviousScene();

    return cards;
  }

  /**
   * Return the document as the backend should see it for this export.
   *
   * When `sceneRangeEnabled` is off this is a straight passthrough. When on
   * we slice the ProseMirror `content.content` down to the chosen scene
   * range and override `scene_number_start` so the output is numbered from
   * the user's "From" value instead of restarting at 1 — matches the "give
   * the actor scenes 5–9 of our script" use case (issue #17).
   */
  /** Build a film-shaped export document that concatenates every episode of
   *  a series project. Scene cards are remapped so their `scene_index`
   *  values align with the combined document's global scene ordering. The
   *  series title becomes the export meta title; the first episode's
   *  author / director / settings / story are inherited (writers typically
   *  set these once at the top of the series). A synthetic `episode_boundary`
   *  node is inserted before every episode after the first — the PDF pipeline
   *  renders it as a hard page break + centred episode title and resets the
   *  scene counter so each episode numbers from 1 again. */
  function buildSeriesExportDocument(): ScreenplayDocument | null {
    const doc = documentStore.document;
    if (!doc || doc.type !== 'series' || !doc.series) return null;
    const episodes = doc.series.episodes;
    if (episodes.length === 0) return null;

    type Block = { type?: string; content?: unknown[] };
    const combinedBlocks: Block[] = [];
    // Walk every episode's scene_cards, shifting each card's scene_index by
    // the running count of scenes already emitted. The PDF side treats
    // `scene_index` as a 0-based pointer into the flattened scene list, so
    // this must align with the order blocks are pushed below.
    const combinedSceneCards: Array<{
      scene_index: number;
      description: string;
      shoot_notes: string;
      extra_characters: string;
    }> = [];
    let sceneOffset = 0;

    for (let i = 0; i < episodes.length; i++) {
      const ep = episodes[i];
      const epContent = ep.content as { content?: Block[] } | null;
      const blocks: Block[] = Array.isArray(epContent?.content) ? (epContent!.content ?? []) : [];

      // Remap the episode's scene_cards by adding the running offset.
      for (const card of ep.scene_cards ?? []) {
        combinedSceneCards.push({
          ...card,
          scene_index: card.scene_index + sceneOffset,
        });
      }

      // Insert an `episode_boundary` marker before *every* episode. The PDF
      // renderer uses a weak pagebreak so the first episode's title lands on
      // the opening content page (no spurious blank leader), while later
      // episodes get a real page break. The boundary also resets the
      // scene-number counter so each episode numbers from 1.
      //
      // We use `ep.title` (the Episode's own short title like "തിരിച്ചുവരവ്")
      // rather than `ep.meta.title` — the latter is often the composed
      // "<Series> — Ep N: <Title>" used on per-episode title pages, and
      // reusing it here would print the series name twice.
      const epTitle = ep.title?.trim() ?? '';
      const label = epTitle
        ? `EPISODE ${ep.number}: ${epTitle}`
        : `EPISODE ${ep.number}`;
      combinedBlocks.push({
        type: 'episode_boundary',
        content: [{ type: 'text', text: label }],
      });
      combinedBlocks.push(...blocks);

      // Advance the offset by however many scene_headings this episode had.
      const sceneCountInEp = blocks.filter((b) => b.type === 'scene_heading').length;
      sceneOffset += sceneCountInEp;
    }

    const firstEp = episodes[0];
    const seriesTitle = doc.series.title || 'Untitled Series';

    // Font is a series-wide choice — `document.settings.font` is where the
    // Settings modal writes. We merge it over the first episode's settings
    // so a combined export always uses the user-selected font rather than
    // the episode's stale default.
    const mergedSettings = {
      ...firstEp.settings,
      font: doc.settings.font || firstEp.settings.font,
    };

    return {
      type: 'film',
      series: null,
      content: { type: 'doc', content: combinedBlocks },
      meta: { ...firstEp.meta, title: seriesTitle },
      settings: mergedSettings,
      story: firstEp.story,
      scene_cards: combinedSceneCards,
    };
  }

  function buildDocumentForExport(): ScreenplayDocument | null {
    // For a series in "series" scope, flatten every episode into one
    // combined document. Otherwise start from the "active export document" —
    // for a film this is the doc itself; for a series in "episode" scope
    // it's a film-shaped wrapper around the active episode.
    const base = isSeriesProject && exportScope === 'series'
      ? buildSeriesExportDocument()
      : documentStore.activeExportDocument;
    if (!base) return null;
    // Scene-range slicing only applies in episode / film scope — it doesn't
    // make sense to slice across an entire series export.
    if (!sceneRangeEnabled || (isSeriesProject && exportScope === 'series')) return base;

    const content = base.content as {
      type?: string;
      content?: Array<{ type?: string }>;
    };
    if (!content?.content) return base;

    const startNum = base.settings.scene_number_start ?? 1;
    // Clamp into the real range so a stale state can't produce an empty slice.
    const fromDisplay = Math.max(startNum, Math.min(lastSceneNumber, sceneRangeFrom));
    const toDisplay = Math.max(fromDisplay, Math.min(lastSceneNumber, sceneRangeTo));
    const fromIdx = fromDisplay - startNum; // 0-based doc scene index
    const toIdx = toDisplay - startNum;

    // Walk nodes, keeping only those belonging to scenes in [fromIdx, toIdx].
    // Nodes before the first selected scene_heading are dropped (title-page
    // etc. is rebuilt from meta separately).
    let currentSceneIdx = -1;
    const filtered: Array<{ type?: string }> = [];
    for (const node of content.content) {
      if (node.type === 'scene_heading') {
        currentSceneIdx++;
        if (currentSceneIdx > toIdx) break;
      }
      if (currentSceneIdx >= fromIdx && currentSceneIdx <= toIdx) {
        filtered.push(node);
      }
    }

    // Re-index scene_cards so they align with the sliced content. In the
    // full document a card's scene_index is 0-based against the whole doc;
    // after slicing, scene_index 0 should mean the first scene in the slice.
    const rebasedCards = base.scene_cards
      .filter((c) => c.scene_index >= fromIdx && c.scene_index <= toIdx)
      .map((c) => ({ ...c, scene_index: c.scene_index - fromIdx }));

    return {
      ...base,
      content: { ...content, content: filtered },
      settings: { ...base.settings, scene_number_start: fromDisplay },
      scene_cards: rebasedCards,
    };
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
    const doc = buildDocumentForExport();
    if (!doc) return;
    exportingPlaintext = true;
    errorMessage = '';

    try {
      const plaintext = await invoke<string>('export_plaintext', {
        document: doc,
      });

      const title = doc.meta.title || 'screenplay';
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
    const doc = buildDocumentForExport();
    if (!doc) return;
    exportingFountain = true;
    errorMessage = '';

    try {
      const fountainText = await invoke<string>('export_fountain', {
        document: doc,
      });

      const title = doc.meta.title || 'screenplay';
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
    const doc = buildDocumentForExport();
    if (!doc) return;
    exporting = true;
    errorMessage = '';

    try {
      const sceneCardsData = includeSceneCards ? buildSceneCardsData(doc) : [];

      const bytes = await invoke<number[]>('export_combined_pdf', {
        document: doc,
        options: {
          include_title_page: includeTitlePage,
          include_synopsis: includeSynopsis,
          include_treatment: includeTreatment,
          include_screenplay: includeScreenplay,
          include_narrative: includeNarrative,
          include_scene_cards: includeSceneCards,
          format,
          page_break_after_scene: pageBreakAfterScene,
          characters_below_heading: charactersBelowHeading,
          include_page_numbers: includePageNumbers,
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
    <div class="modal-card" use:focusTrap>
      {#if anyExporting}
        <div class="progress-bar" aria-hidden="true"><span class="progress-bar-fill"></span></div>
      {/if}
      <div class="modal-header">
        <h2>Export Document</h2>
        <button class="btn-close" onclick={() => { open = false; }} disabled={anyExporting}>&times;</button>
      </div>

      {#if isSeriesProject}
        <div class="series-context">
          <span class="series-context-label">Series</span>
          <span class="series-context-value">{documentStore.document?.series?.title || 'Untitled Series'}</span>
          <span class="series-context-ep">· Episode {documentStore.activeEpisode?.number ?? ''}{documentStore.activeEpisode?.title ? ` — ${documentStore.activeEpisode.title}` : ''}</span>
        </div>
        <div class="section-label">Scope</div>
        <div class="radio-group">
          <label class="radio-row">
            <input type="radio" name="export-scope" value="episode" bind:group={exportScope} />
            <span>Current episode only</span>
          </label>
          <label class="radio-row">
            <input type="radio" name="export-scope" value="series" bind:group={exportScope} />
            <span>Entire series (all episodes)</span>
          </label>
        </div>
      {/if}

      <div class="section-label">
        <span>What to include</span>
        <button type="button" class="inline-link" onclick={onEditMetadata} disabled={anyExporting}>
          Edit metadata
        </button>
      </div>
      <div class="checkbox-group">
        <label class="checkbox-row">
          <input type="checkbox" bind:checked={includeTitlePage} />
          <span>Title Page</span>
        </label>
        {#if hasSynopsis}
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={includeSynopsis} />
            <span>Synopsis</span>
          </label>
        {/if}
        {#if hasTreatment}
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={includeTreatment} />
            <span>Treatment</span>
          </label>
        {/if}
        <label class="checkbox-row">
          <input type="checkbox" bind:checked={includeScreenplay} />
          <span>Screenplay</span>
        </label>
        {#if hasNarrative}
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={includeNarrative} />
            <span>Narrative</span>
          </label>
        {/if}
        <label class="checkbox-row">
          <input type="checkbox" bind:checked={includeSceneCards} />
          <span>Scene Cards</span>
        </label>
      </div>
      {#if !hasSynopsis || !hasTreatment || !hasNarrative}
        <p class="unavailable-hint">
          Add text in the Story panel to include
          {#if !hasSynopsis}synopsis{/if}{#if !hasSynopsis && (!hasTreatment || !hasNarrative)}{', '}{/if}{#if !hasTreatment}treatment{/if}{#if !hasTreatment && !hasNarrative}{', '}{/if}{#if !hasNarrative}narrative{/if}.
        </p>
      {/if}

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
      {/if}

      <div class="section-label">Layout</div>
      <div class="checkbox-group">
        {#if includeScreenplay}
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={pageBreakAfterScene} />
            <span>Page break after each scene</span>
          </label>
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={charactersBelowHeading} />
            <span>Characters list below scene heading</span>
          </label>
        {/if}
        <label class="checkbox-row">
          <input type="checkbox" bind:checked={includePageNumbers} />
          <span>Page numbers</span>
        </label>
        {#if sceneCount > 0 && (includeScreenplay || includeSceneCards) && !(isSeriesProject && exportScope === 'series')}
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={sceneRangeEnabled} />
            <span>Only selected scenes</span>
          </label>
          {#if sceneRangeEnabled}
            <div class="range-row">
              <span class="range-label">Scenes</span>
              <input
                type="number"
                min={firstSceneNumber}
                max={lastSceneNumber}
                bind:value={sceneRangeFrom}
                aria-label="From scene number"
              />
              <span class="range-sep">to</span>
              <input
                type="number"
                min={firstSceneNumber}
                max={lastSceneNumber}
                bind:value={sceneRangeTo}
                aria-label="To scene number"
              />
              <span class="range-hint">(of {firstSceneNumber}–{lastSceneNumber})</span>
            </div>
          {/if}
        {/if}
      </div>

      {#if errorMessage}
        <p class="error-message">{errorMessage}</p>
      {/if}

      <div class="modal-footer">
        <button class="btn-ghost" onclick={() => { open = false; }} disabled={anyExporting}>Cancel</button>
        <button class="btn-secondary" onclick={handlePlaintextExport} disabled={anyExporting}>
          {#if exportingPlaintext}<span class="spinner" aria-hidden="true"></span>Exporting{:else}Plain Text{/if}
        </button>
        <button class="btn-secondary" onclick={handleFountainExport} disabled={anyExporting}>
          {#if exportingFountain}<span class="spinner" aria-hidden="true"></span>Exporting{:else}Fountain{/if}
        </button>
        <button class="btn-primary" onclick={handleExport} disabled={anyExporting}>
          {#if exporting}<span class="spinner spinner-on-primary" aria-hidden="true"></span>Generating PDF{:else}Export PDF{/if}
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
    position: relative;
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

  .progress-bar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    overflow: hidden;
    border-top-left-radius: 12px;
    border-top-right-radius: 12px;
    background: var(--surface-hover);
  }

  .progress-bar-fill {
    display: block;
    width: 40%;
    height: 100%;
    background: var(--accent);
    animation: progress-slide 1.1s ease-in-out infinite;
  }

  @keyframes progress-slide {
    0%   { transform: translateX(-100%); }
    100% { transform: translateX(350%); }
  }

  .spinner {
    display: inline-block;
    width: 10px;
    height: 10px;
    margin-right: 6px;
    border: 1.5px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    animation: spinner-spin 0.7s linear infinite;
    vertical-align: -1px;
    opacity: 0.8;
  }

  .spinner-on-primary {
    border-color: #fff;
    border-right-color: transparent;
  }

  @keyframes spinner-spin {
    to { transform: rotate(360deg); }
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

  /* Mirror MetadataModal's series-context chip so writers always see which
     episode the export scope refers to when editing a series project. */
  .series-context {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 8px 10px;
    margin: -8px 0 16px;
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    font-size: 11.5px;
    color: var(--text-secondary);
    flex-wrap: wrap;
  }

  .series-context-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .series-context-value {
    color: var(--text-primary);
    font-weight: 500;
  }

  .series-context-ep {
    color: var(--text-muted);
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
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }

  .inline-link {
    appearance: none;
    background: transparent;
    border: none;
    padding: 0;
    font-family: inherit;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0;
    text-transform: none;
    color: var(--accent);
    cursor: pointer;
  }

  .inline-link:hover {
    text-decoration: underline;
  }

  .inline-link:disabled {
    color: var(--text-muted);
    cursor: default;
    text-decoration: none;
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

  .unavailable-hint {
    margin: -14px 0 18px;
    font-size: 11.5px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .checkbox-row input,
  .radio-row input {
    accent-color: var(--accent);
  }

  .range-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: 22px;
    padding: 2px 0 4px;
    font-size: 12.5px;
    color: var(--text-secondary);
  }

  .range-label {
    font-size: 12px;
    color: var(--text-muted);
  }

  .range-row input[type="number"] {
    width: 56px;
    padding: 3px 6px;
    font-size: 12.5px;
    font-family: inherit;
    color: var(--text-primary);
    background: var(--surface-base);
    border: 1px solid var(--border-medium);
    border-radius: 4px;
  }

  .range-row input[type="number"]:focus {
    outline: none;
    border-color: var(--accent);
  }

  .range-sep {
    font-size: 12px;
    color: var(--text-muted);
  }

  .range-hint {
    font-size: 11px;
    color: var(--text-muted);
    margin-left: 2px;
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

</style>
