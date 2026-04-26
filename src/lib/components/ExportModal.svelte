<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import { writeFile } from '@tauri-apps/plugin-fs';
  import { documentStore, type ScreenplayDocument, type ScreenplayMeta } from '$lib/stores/documentStore.svelte';
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
  /** Compact card view — strips description / notes / location group
   *  from each card so the breakdown reads as a one-line-per-scene
   *  shoot-day overview. Only meaningful when scene cards are
   *  included; the toggle is hidden otherwise. */
  let compactSceneCards = $state(false);
  /** Daily Shoot List PDF section (#124) — only meaningful when at least
   *  one scene card has a `scheduled_date`. */
  let includeShootList = $state(false);

  /** Bulk-select / clear for the section tile group (#142). "All" turns
   *  on every tile that's actually available — Synopsis / Treatment /
   *  Narrative are skipped when the Story panel is empty (their tiles
   *  are disabled in the UI), and Daily Shoot List is skipped when no
   *  scene has a scheduled date. */
  function selectAllSections() {
    includeTitlePage = true;
    includeScreenplay = true;
    includeSceneCards = true;
    if (hasSynopsis) includeSynopsis = true;
    if (hasTreatment) includeTreatment = true;
    if (hasNarrative) includeNarrative = true;
    if (hasScheduledScenes) includeShootList = true;
  }

  function clearAllSections() {
    includeTitlePage = false;
    includeSynopsis = false;
    includeTreatment = false;
    includeScreenplay = false;
    includeNarrative = false;
    includeSceneCards = false;
    includeShootList = false;
  }
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
  /** True when at least one scene card has a non-empty scheduled_date —
   *  the Daily Shoot List checkbox only appears when there's data to print. */
  let hasScheduledScenes = $derived(
    documentStore.activeSceneCards.some((c) => (c.scheduled_date ?? '').trim().length > 0),
  );

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
    };

    for (const node of content.content) {
      if (node.type === 'scene_heading') {
        finalizePreviousScene();
        sceneIndex++;
        currentSceneCharacters = [];

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
          description: manualCard?.description ?? '',
          shoot_notes: manualCard?.shoot_notes ?? '',
          // Production-planning fields surfaced on the printed
          // breakdown card. Empty strings are normal — the PDF
          // template only renders the line when the field is set.
          scheduled_date: manualCard?.scheduled_date ?? '',
          location_group: manualCard?.location_group ?? '',
        });
      } else if (node.type === 'character') {
        const charName = (node.content ?? []).map((c) => c.text ?? '').join('').trim();
        if (charName && !currentSceneCharacters.includes(charName)) {
          currentSceneCharacters.push(charName);
        }
      }
    }

    finalizePreviousScene();

    return cards;
  }

  /**
   * Build the Daily Shoot List rows the backend renders into a per-day PDF
   * (#124). Walks the same content as buildSceneCardsData but only emits
   * scenes that have a non-empty `scheduled_date` on their SceneCard. Each
   * row carries the auto-detected location/time/cast count plus the
   * page-eighths estimate (1 page ≈ 8 eighths ≈ 3000 chars, so ~375 chars
   * per eighth, with a 1-eighth floor so the shortest scenes still register).
   *
   * Rows are sorted by (scheduled_date, location_group, scene_number) so
   * the backend can group on date / group boundaries with a simple linear
   * walk.
   */
  function buildShootListData(
    doc: ScreenplayDocument | null = documentStore.activeExportDocument,
  ): Array<Record<string, unknown>> {
    if (!doc || !doc.content) return [];

    const content = doc.content as {
      content?: Array<{ type?: string; content?: Array<{ text?: string }> }>;
    };
    if (!content.content) return [];

    const startNum = doc.settings?.scene_number_start ?? 1;
    let sceneIndex = -1;
    let currentSceneCharacters: string[] = [];
    let currentSceneCharCount = 0;
    let currentSceneHeading = '';

    interface Row extends Record<string, unknown> {
      scheduled_date: string;
      location_group: string;
      scene_number: number;
      heading: string;
      location: string;
      time: string;
      character_count: number;
      eighths: number;
    }
    const rows: Row[] = [];

    const finalize = () => {
      if (sceneIndex < 0) return;
      const card = doc.scene_cards.find((c) => c.scene_index === sceneIndex);
      const date = (card?.scheduled_date ?? '').trim();
      if (!date) return; // Unscheduled scenes don't appear in the shoot list.

      const { location, time } = parseSceneHeading(currentSceneHeading);
      // Merge auto-detected speakers with extras for the cast count, same as
      // the per-card characters line — non-speaking roles count as cast too.
      const extras = (card?.extra_characters ?? '')
        .split(',')
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
      const allCast = new Set([...currentSceneCharacters, ...extras]);

      const eighths = Math.max(1, Math.round(currentSceneCharCount / 375));

      rows.push({
        scheduled_date: date,
        location_group: (card?.location_group ?? '').trim(),
        scene_number: startNum + sceneIndex,
        heading: currentSceneHeading,
        location,
        time,
        character_count: allCast.size,
        eighths,
      });
    };

    for (const node of content.content) {
      if (node.type === 'scene_heading') {
        finalize();
        sceneIndex++;
        currentSceneCharacters = [];
        currentSceneCharCount = 0;
        currentSceneHeading = (node.content ?? []).map((c) => c.text ?? '').join('');
      } else if (node.type === 'character') {
        const name = (node.content ?? []).map((c) => c.text ?? '').join('').trim();
        if (name && !currentSceneCharacters.includes(name)) currentSceneCharacters.push(name);
      }
      if (node.content) {
        currentSceneCharCount += node.content.reduce((sum, c) => sum + (c.text ?? '').length, 0);
      }
    }
    finalize();

    rows.sort((a, b) => {
      if (a.scheduled_date !== b.scheduled_date) {
        return a.scheduled_date.localeCompare(b.scheduled_date);
      }
      if (a.location_group !== b.location_group) {
        // Empty groups go after named ones within a day so writer-noted
        // location clusters head the list.
        if (!a.location_group) return 1;
        if (!b.location_group) return -1;
        return a.location_group.localeCompare(b.location_group);
      }
      return a.scene_number - b.scene_number;
    });

    return rows;
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
      scheduled_date: string;
      location_group: string;
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

    // Pick a series-level credit by collapsing the per-episode fields. If
    // every episode shares the same author (or director, contact, …) we
    // show that value on the series cover; otherwise we blank it so the
    // cover doesn't misattribute the whole series to whoever wrote Ep 1.
    // A dedicated per-series metadata UI would be nicer eventually, but
    // this is a safe default that respects what writers already typed.
    const uniqueOrBlank = (pick: (m: ScreenplayMeta) => string): string => {
      const vals = new Set(episodes.map((ep) => pick(ep.meta).trim()).filter(Boolean));
      return vals.size === 1 ? [...vals][0] : '';
    };
    const seriesMeta: ScreenplayMeta = {
      ...firstEp.meta,
      title: seriesTitle,
      tagline: uniqueOrBlank((m) => m.tagline),
      author: uniqueOrBlank((m) => m.author),
      director: uniqueOrBlank((m) => m.director),
      contact: uniqueOrBlank((m) => m.contact),
      registration_number: uniqueOrBlank((m) => m.registration_number),
      footnote: uniqueOrBlank((m) => m.footnote),
    };

    return {
      type: 'film',
      series: null,
      content: { type: 'doc', content: combinedBlocks },
      meta: seriesMeta,
      settings: mergedSettings,
      story: firstEp.story,
      scene_cards: combinedSceneCards,
    };
  }

  /** Strip characters that cause friction in cross-platform filenames
   *  (slashes, colons, control chars), collapse internal whitespace,
   *  and trim. Doesn't touch Unicode letters — Malayalam titles
   *  survive intact and get saved as ".സ്ക്രിപ്റ്റ്.pdf" if the writer
   *  named the doc that way. */
  function sanitizeForFilename(s: string): string {
    return s
      .replace(/[\\/:*?"<>|\x00-\x1f]/g, '')
      .replace(/\s+/g, ' ')
      .trim();
  }

  /** Derive a contextual default filename (without extension) for a
   *  PDF / Fountain / plain-text export. Reads the document title,
   *  series state, scope, and which sections are included so the
   *  Save dialog opens with a name the writer would have typed
   *  themselves. Falls back gracefully if no title is set. */
  function defaultExportName(): string {
    const meta = documentStore.activeMeta;
    const seriesTitle = documentStore.document?.series?.title?.trim();
    const ep = documentStore.activeEpisode;

    // Base — the project's editorial name.
    let base: string;
    if (isSeriesProject) {
      const series = sanitizeForFilename(seriesTitle || '') || 'Untitled Series';
      if (exportScope === 'series') {
        base = `${series} — All episodes`;
      } else {
        const epNum = ep ? String(ep.number).padStart(2, '0') : '01';
        const epTitle = sanitizeForFilename(ep?.title?.trim() || '');
        base = epTitle ? `${series} — Ep ${epNum} · ${epTitle}` : `${series} — Ep ${epNum}`;
      }
    } else {
      const docTitle = sanitizeForFilename(meta?.title?.trim() || '');
      if (docTitle) {
        base = docTitle;
      } else {
        // No title — fall back to the .screenplay filename stem so the
        // PDF lands next to its source with the same name.
        const path = documentStore.currentPath;
        if (path) {
          const file = path.split('/').pop() ?? path.split('\\').pop() ?? path;
          base = sanitizeForFilename(file.replace(/\.screenplay$/, '')) || 'Screenplay';
        } else {
          base = 'Screenplay';
        }
      }
    }

    // Section suffix — only when the export is *just* one supplemental
    // section (not the full screenplay). Reading "Title — Cards.pdf"
    // tells you what's in the file at a glance. When the screenplay
    // is included we leave the base bare since it's the canonical doc.
    if (!includeScreenplay) {
      const onlyOne = (a: boolean, b: boolean, c: boolean, d: boolean, e: boolean) =>
        [a, b, c, d, e].filter(Boolean).length === 1;
      if (onlyOne(includeSceneCards, includeShootList, includeSynopsis, includeTreatment, includeNarrative)) {
        if (includeSceneCards) base += ' — Cards';
        else if (includeShootList) base += ' — Shoot List';
        else if (includeSynopsis) base += ' — Synopsis';
        else if (includeTreatment) base += ' — Treatment';
        else if (includeNarrative) base += ' — Narrative';
      }
    }

    return base;
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

      const path = await save({
        defaultPath: `${defaultExportName()}.txt`,
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

      const path = await save({
        defaultPath: `${defaultExportName()}.fountain`,
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
      const shootListData = includeShootList ? buildShootListData(doc) : [];

      const bytes = await invoke<number[]>('export_combined_pdf', {
        document: doc,
        options: {
          include_title_page: includeTitlePage,
          include_synopsis: includeSynopsis,
          include_treatment: includeTreatment,
          include_screenplay: includeScreenplay,
          include_narrative: includeNarrative,
          include_scene_cards: includeSceneCards,
          compact_scene_cards: compactSceneCards,
          include_shoot_list: includeShootList,
          format,
          page_break_after_scene: pageBreakAfterScene,
          characters_below_heading: charactersBelowHeading,
          include_page_numbers: includePageNumbers,
          scene_cards_data: sceneCardsData,
          shoot_list_data: shootListData,
        },
      });

      const path = await save({
        defaultPath: `${defaultExportName()}.pdf`,
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
    <div class="modal-card export-card" use:focusTrap>
      {#if anyExporting}
        <div class="progress-bar" aria-hidden="true"><span class="progress-bar-fill"></span></div>
      {/if}

      <header class="export-header">
        <div class="header-text">
          <div class="mh-eyebrow" aria-hidden="true">
            <span class="mh-rule"></span>
            <span>Build &amp; export</span>
          </div>
          <h2 class="mh-title export-title">Print to <em>paper</em></h2>
        </div>
        <button class="btn-close" onclick={() => { open = false; }} disabled={anyExporting} aria-label="Close export">&times;</button>
      </header>

      {#if isSeriesProject}
        <div class="series-strip">
          <div class="series-strip-meta">
            <span class="series-strip-label">Series</span>
            <span class="series-strip-value">{documentStore.document?.series?.title || 'Untitled Series'}</span>
            <span class="series-strip-ep">Episode {documentStore.activeEpisode?.number ?? ''}{documentStore.activeEpisode?.title ? ` — ${documentStore.activeEpisode.title}` : ''}</span>
          </div>
          <div class="scope-segmented" role="group" aria-label="Export scope">
            <button
              type="button"
              class="scope-seg"
              class:active={exportScope === 'episode'}
              onclick={() => { exportScope = 'episode'; }}
            >This episode</button>
            <button
              type="button"
              class="scope-seg"
              class:active={exportScope === 'series'}
              onclick={() => { exportScope = 'series'; }}
            >Full series</button>
          </div>
        </div>
      {/if}

      <div class="export-body">
        <!-- ── Sections column ─────────────────────────────────── -->
        <section class="col-sections" aria-labelledby="sections-heading">
          <div class="col-heading">
            <h3 id="sections-heading">Sections</h3>
            <div class="col-heading-actions">
              <button type="button" class="inline-link" onclick={selectAllSections} disabled={anyExporting} title="Turn on every available section">
                All
              </button>
              <span class="inline-link-sep" aria-hidden="true">·</span>
              <button type="button" class="inline-link" onclick={clearAllSections} disabled={anyExporting} title="Turn off every section">
                None
              </button>
              <span class="inline-link-sep" aria-hidden="true">·</span>
              <button type="button" class="inline-link" onclick={onEditMetadata} disabled={anyExporting}>
                Edit metadata
              </button>
            </div>
          </div>

          <div class="section-grid">
            <!-- Each card is a self-contained tile with icon, title, hint,
                 toggle. Disabled tiles dim down (e.g. Synopsis when empty). -->
            <button
              type="button"
              class="section-tile"
              class:on={includeTitlePage}
              onclick={() => { includeTitlePage = !includeTitlePage; }}
              aria-pressed={includeTitlePage}
            >
              <span class="tile-icon" aria-hidden="true">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="5" y="3" width="14" height="18" rx="1.5"/>
                  <line x1="9" y1="8" x2="15" y2="8"/>
                  <line x1="8" y1="12" x2="16" y2="12"/>
                  <line x1="9" y1="15" x2="15" y2="15"/>
                </svg>
              </span>
              <span class="tile-body">
                <span class="tile-title">Title Page</span>
                <span class="tile-hint">Auto-generated from metadata</span>
              </span>
              <span class="tile-check" aria-hidden="true"></span>
            </button>

            <button
              type="button"
              class="section-tile"
              class:on={includeScreenplay}
              onclick={() => { includeScreenplay = !includeScreenplay; }}
              aria-pressed={includeScreenplay}
            >
              <span class="tile-icon tile-icon-accent" aria-hidden="true">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="4" y="3" width="16" height="18" rx="1.5"/>
                  <line x1="7" y1="7" x2="13" y2="7"/>
                  <line x1="7" y1="11" x2="17" y2="11"/>
                  <line x1="7" y1="15" x2="17" y2="15"/>
                  <line x1="7" y1="19" x2="11" y2="19"/>
                </svg>
              </span>
              <span class="tile-body">
                <span class="tile-title">Screenplay</span>
                <span class="tile-hint">{sceneCount} {sceneCount === 1 ? 'scene' : 'scenes'} · main content</span>
              </span>
              <span class="tile-check"></span>
            </button>

            <button
              type="button"
              class="section-tile"
              class:on={includeSynopsis}
              class:disabled={!hasSynopsis}
              disabled={!hasSynopsis}
              onclick={() => { includeSynopsis = !includeSynopsis; }}
              aria-pressed={includeSynopsis}
            >
              <span class="tile-icon" aria-hidden="true">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="5" y1="7" x2="19" y2="7"/>
                  <line x1="5" y1="11" x2="19" y2="11"/>
                  <line x1="5" y1="15" x2="14" y2="15"/>
                </svg>
              </span>
              <span class="tile-body">
                <span class="tile-title">Synopsis</span>
                <span class="tile-hint">{hasSynopsis ? 'Story panel — short prose' : 'Add text in the Story panel'}</span>
              </span>
              <span class="tile-check"></span>
            </button>

            <button
              type="button"
              class="section-tile"
              class:on={includeTreatment}
              class:disabled={!hasTreatment}
              disabled={!hasTreatment}
              onclick={() => { includeTreatment = !includeTreatment; }}
              aria-pressed={includeTreatment}
            >
              <span class="tile-icon" aria-hidden="true">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="5" y1="6" x2="19" y2="6"/>
                  <line x1="5" y1="10" x2="19" y2="10"/>
                  <line x1="5" y1="14" x2="19" y2="14"/>
                  <line x1="5" y1="18" x2="14" y2="18"/>
                </svg>
              </span>
              <span class="tile-body">
                <span class="tile-title">Treatment</span>
                <span class="tile-hint">{hasTreatment ? 'Scene-by-scene prose' : 'Add text in the Story panel'}</span>
              </span>
              <span class="tile-check"></span>
            </button>

            <button
              type="button"
              class="section-tile"
              class:on={includeNarrative}
              class:disabled={!hasNarrative}
              disabled={!hasNarrative}
              onclick={() => { includeNarrative = !includeNarrative; }}
              aria-pressed={includeNarrative}
            >
              <span class="tile-icon" aria-hidden="true">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M5 4 H19 V20 H5 Z"/>
                  <line x1="8" y1="8" x2="16" y2="8"/>
                  <line x1="8" y1="11" x2="16" y2="11"/>
                  <line x1="8" y1="14" x2="16" y2="14"/>
                  <line x1="8" y1="17" x2="13" y2="17"/>
                </svg>
              </span>
              <span class="tile-body">
                <span class="tile-title">Narrative</span>
                <span class="tile-hint">{hasNarrative ? 'Long-form story' : 'Add text in the Story panel'}</span>
              </span>
              <span class="tile-check"></span>
            </button>

            <button
              type="button"
              class="section-tile"
              class:on={includeSceneCards}
              onclick={() => { includeSceneCards = !includeSceneCards; }}
              aria-pressed={includeSceneCards}
            >
              <span class="tile-icon" aria-hidden="true">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="4" y="5" width="7" height="14" rx="1.5"/>
                  <rect x="13" y="5" width="7" height="14" rx="1.5"/>
                </svg>
              </span>
              <span class="tile-body">
                <span class="tile-title">Scene Cards</span>
                <span class="tile-hint">Per-scene breakdown</span>
              </span>
              <span class="tile-check"></span>
            </button>

            {#if hasScheduledScenes}
              <button
                type="button"
                class="section-tile"
                class:on={includeShootList}
                onclick={() => { includeShootList = !includeShootList; }}
                aria-pressed={includeShootList}
              >
                <span class="tile-icon" aria-hidden="true">
                  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="3" y="5" width="18" height="16" rx="2"/>
                    <line x1="3" y1="10" x2="21" y2="10"/>
                    <line x1="8" y1="3" x2="8" y2="7"/>
                    <line x1="16" y1="3" x2="16" y2="7"/>
                  </svg>
                </span>
                <span class="tile-body">
                  <span class="tile-title">Daily Shoot List</span>
                  <span class="tile-hint">One page per shoot day</span>
                </span>
                <span class="tile-check"></span>
              </button>
            {/if}
          </div>
        </section>

        <!-- ── Format & options column ─────────────────────────── -->
        <section class="col-options" aria-labelledby="format-heading">
          <div class="col-heading">
            <h3 id="format-heading">Format &amp; options</h3>
          </div>

          {#if includeScreenplay}
            <div class="format-picker" role="radiogroup" aria-label="Screenplay format">
              <button
                type="button"
                class="format-card"
                class:active={format === 'hollywood'}
                role="radio"
                aria-checked={format === 'hollywood'}
                onclick={() => { format = 'hollywood'; }}
              >
                <span class="format-art" aria-hidden="true">
                  <svg width="44" height="56" viewBox="0 0 44 56" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="3" y="3" width="38" height="50" rx="1.5"/>
                    <line x1="9"  y1="11" x2="20" y2="11"/>
                    <line x1="9"  y1="16" x2="35" y2="16"/>
                    <line x1="9"  y1="20" x2="35" y2="20"/>
                    <line x1="20" y1="26" x2="32" y2="26"/>
                    <line x1="16" y1="30" x2="34" y2="30"/>
                    <line x1="16" y1="34" x2="34" y2="34"/>
                    <line x1="9"  y1="40" x2="35" y2="40"/>
                    <line x1="9"  y1="44" x2="35" y2="44"/>
                  </svg>
                </span>
                <span class="format-meta">
                  <span class="format-name">Hollywood</span>
                  <span class="format-desc">Single column, US standard</span>
                </span>
              </button>

              <button
                type="button"
                class="format-card"
                class:active={format === 'indian'}
                role="radio"
                aria-checked={format === 'indian'}
                onclick={() => { format = 'indian'; }}
              >
                <span class="format-art" aria-hidden="true">
                  <svg width="44" height="56" viewBox="0 0 44 56" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="3" y="3" width="38" height="50" rx="1.5"/>
                    <line x1="22" y1="9" x2="22" y2="49"/>
                    <line x1="6"  y1="13" x2="19" y2="13"/>
                    <line x1="6"  y1="17" x2="19" y2="17"/>
                    <line x1="6"  y1="21" x2="17" y2="21"/>
                    <line x1="6"  y1="27" x2="19" y2="27"/>
                    <line x1="6"  y1="31" x2="19" y2="31"/>
                    <line x1="25" y1="13" x2="38" y2="13"/>
                    <line x1="25" y1="17" x2="38" y2="17"/>
                    <line x1="25" y1="21" x2="38" y2="21"/>
                    <line x1="25" y1="27" x2="36" y2="27"/>
                    <line x1="25" y1="31" x2="38" y2="31"/>
                  </svg>
                </span>
                <span class="format-meta">
                  <span class="format-name">Indian</span>
                  <span class="format-desc">Two-column dialogue layout</span>
                </span>
              </button>
            </div>
          {:else}
            <div class="format-empty">
              <p>Format applies to the screenplay section. Turn that on to choose Hollywood or Indian.</p>
            </div>
          {/if}

          <div class="options-block">
            <span class="options-title">Options</span>

            {#if includeScreenplay}
              <label class="opt-row">
                <span class="opt-label">
                  <span class="opt-name">Page break per scene</span>
                  <span class="opt-desc">Each scene starts on a new PDF page</span>
                </span>
                <button
                  type="button"
                  class="opt-toggle"
                  role="switch"
                  aria-label="Page break after each scene"
                  aria-checked={pageBreakAfterScene}
                  onclick={() => { pageBreakAfterScene = !pageBreakAfterScene; }}
                ><span class="opt-thumb"></span></button>
              </label>

              <label class="opt-row">
                <span class="opt-label">
                  <span class="opt-name">Characters under heading</span>
                  <span class="opt-desc">Auto-list speakers below each scene heading</span>
                </span>
                <button
                  type="button"
                  class="opt-toggle"
                  role="switch"
                  aria-label="Show characters under scene heading"
                  aria-checked={charactersBelowHeading}
                  onclick={() => { charactersBelowHeading = !charactersBelowHeading; }}
                ><span class="opt-thumb"></span></button>
              </label>
            {/if}

            <label class="opt-row">
              <span class="opt-label">
                <span class="opt-name">Page numbers</span>
                <span class="opt-desc">Top-right corner of every body page</span>
              </span>
              <button
                type="button"
                class="opt-toggle"
                role="switch"
                aria-label="Include page numbers"
                aria-checked={includePageNumbers}
                onclick={() => { includePageNumbers = !includePageNumbers; }}
              ><span class="opt-thumb"></span></button>
            </label>

            {#if includeSceneCards}
              <label class="opt-row">
                <span class="opt-label">
                  <span class="opt-name">Compact card view</span>
                  <span class="opt-desc">Slug + cast only — packs more cards per page</span>
                </span>
                <button
                  type="button"
                  class="opt-toggle"
                  role="switch"
                  aria-label="Compact scene card view"
                  aria-checked={compactSceneCards}
                  onclick={() => { compactSceneCards = !compactSceneCards; }}
                ><span class="opt-thumb"></span></button>
              </label>
            {/if}

            {#if sceneCount > 0 && (includeScreenplay || includeSceneCards) && !(isSeriesProject && exportScope === 'series')}
              <label class="opt-row">
                <span class="opt-label">
                  <span class="opt-name">Scene range</span>
                  <span class="opt-desc">Export only a slice of the script</span>
                </span>
                <button
                  type="button"
                  class="opt-toggle"
                  role="switch"
                  aria-label="Limit export to a scene range"
                  aria-checked={sceneRangeEnabled}
                  onclick={() => { sceneRangeEnabled = !sceneRangeEnabled; }}
                ><span class="opt-thumb"></span></button>
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
                  <span class="range-sep">→</span>
                  <input
                    type="number"
                    min={firstSceneNumber}
                    max={lastSceneNumber}
                    bind:value={sceneRangeTo}
                    aria-label="To scene number"
                  />
                  <span class="range-hint">of {firstSceneNumber}–{lastSceneNumber}</span>
                </div>
              {/if}
            {/if}
          </div>
        </section>
      </div>

      {#if errorMessage}
        <p class="error-message">{errorMessage}</p>
      {/if}

      <footer class="export-footer">
        <button class="btn-ghost" onclick={() => { open = false; }} disabled={anyExporting}>Cancel</button>
        <div class="footer-spacer"></div>
        <button class="btn-secondary" onclick={handlePlaintextExport} disabled={anyExporting}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <line x1="6" y1="7" x2="18" y2="7"/>
            <line x1="6" y1="11" x2="18" y2="11"/>
            <line x1="6" y1="15" x2="14" y2="15"/>
          </svg>
          {#if exportingPlaintext}<span class="spinner" aria-hidden="true"></span>Exporting{:else}Plain text{/if}
        </button>
        <button class="btn-secondary" onclick={handleFountainExport} disabled={anyExporting}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 5 H19 L17 11 L19 17 H5 L7 11 Z"/>
          </svg>
          {#if exportingFountain}<span class="spinner" aria-hidden="true"></span>Exporting{:else}Fountain{/if}
        </button>
        <button class="btn-primary" onclick={handleExport} disabled={anyExporting}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 4 V16"/>
            <path d="M7 11 L12 16 L17 11"/>
            <line x1="5" y1="20" x2="19" y2="20"/>
          </svg>
          {#if exporting}<span class="spinner spinner-on-primary" aria-hidden="true"></span>Generating PDF{:else}Export PDF{/if}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--backdrop);
    backdrop-filter: var(--backdrop-blur);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--modal-z);
  }

  /* Wide squared layout that matches the "build something" feel of the
     Export operation — sections on the left, format/options on the right,
     export buttons in a sticky footer. Stable size regardless of which
     sections are toggled on (#108 + sizing complaint). */
  .export-card {
    position: relative;
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: var(--modal-radius);
    width: 880px;
    max-width: 94vw;
    height: 76vh;
    max-height: 720px;
    box-shadow: var(--modal-shadow);
    animation: modal-in var(--modal-anim-duration) ease-out;
    font-family: var(--ui-font);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .progress-bar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    overflow: hidden;
    border-top-left-radius: var(--modal-radius);
    border-top-right-radius: var(--modal-radius);
    background: var(--surface-hover);
    z-index: 2;
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
    opacity: 0.85;
  }

  .spinner-on-primary {
    border-color: var(--text-on-accent);
    border-right-color: transparent;
  }

  @keyframes spinner-spin {
    to { transform: rotate(360deg); }
  }

  @keyframes modal-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }

  /* ─── Header strip ─── */
  .export-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 22px 28px 18px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .header-text {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .export-title {
    margin: 0;
    font-size: 22px;
    line-height: 1;
  }

  .export-header h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.2;
  }

  .btn-close {
    flex-shrink: 0;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    font-size: 20px;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-close:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  /* ─── Series scope strip ─── */
  .series-strip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    padding: 12px 28px;
    background: var(--surface-base);
    border-bottom: 1px solid var(--border-subtle);
  }

  .series-strip-meta {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 11.5px;
    color: var(--text-secondary);
    min-width: 0;
  }

  .series-strip-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .series-strip-value {
    color: var(--text-primary);
    font-weight: 600;
  }

  .series-strip-ep {
    color: var(--text-muted);
  }

  /* Pill-style segmented control. Cleaner than radio rows for a binary
     choice the writer flips a few times per export. */
  .scope-segmented {
    display: inline-flex;
    background: var(--surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 7px;
    padding: 2px;
    flex-shrink: 0;
  }

  .scope-seg {
    background: transparent;
    border: none;
    padding: 5px 12px;
    border-radius: 5px;
    color: var(--text-muted);
    font-family: var(--ui-font);
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .scope-seg:hover { color: var(--text-secondary); }

  .scope-seg.active {
    background: var(--surface-float);
    color: var(--text-primary);
    box-shadow: 0 1px 2px var(--shadow-soft);
  }

  /* ─── Two-column body ─── */
  .export-body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr);
    gap: 0;
  }

  .col-sections,
  .col-options {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
    padding: 22px 28px 24px;
  }

  .col-options {
    background: var(--surface-base);
    border-left: 1px solid var(--border-subtle);
  }

  .col-heading {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
  }

  .col-heading h3 {
    margin: 0;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .inline-link {
    appearance: none;
    background: transparent;
    border: none;
    padding: 0;
    font-family: inherit;
    font-size: 11px;
    font-weight: 500;
    color: var(--accent);
    cursor: pointer;
    text-transform: none;
    letter-spacing: 0;
  }

  .col-heading-actions {
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
  }

  .inline-link-sep {
    color: var(--text-muted);
    opacity: 0.5;
    font-size: 11px;
  }

  .inline-link:hover { text-decoration: underline; }

  .inline-link:disabled {
    color: var(--text-muted);
    cursor: default;
    text-decoration: none;
  }

  /* ─── Section tiles (replaces checkbox stack) ─── */
  .section-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .section-tile {
    display: grid;
    grid-template-columns: 36px 1fr 16px;
    align-items: center;
    gap: 12px;
    padding: 11px 12px;
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    color: var(--text-secondary);
    font-family: var(--ui-font);
    text-align: left;
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                border-color var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .section-tile:hover:not(:disabled) {
    background: var(--surface-hover);
    border-color: var(--border-medium);
  }

  .section-tile.on {
    background: var(--accent-muted);
    border-color: var(--accent);
    color: var(--text-primary);
  }

  .section-tile.disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .tile-icon {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    background: var(--surface-elevated);
    color: var(--text-muted);
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .section-tile.on .tile-icon {
    background: var(--accent);
    color: var(--text-on-accent);
  }

  /* Screenplay tile gets a slightly warmer treatment so the eye lands on
     the main content section first. */
  .tile-icon-accent {
    background: var(--accent-muted);
    color: var(--accent);
  }

  .tile-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .tile-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.2;
  }

  .tile-hint {
    font-size: 10.5px;
    color: var(--text-muted);
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Inline check that fills with accent when the tile is on. */
  .tile-check {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid var(--border-medium);
    background: transparent;
    flex-shrink: 0;
    position: relative;
    transition: background var(--motion-fast, 100ms) ease,
                border-color var(--motion-fast, 100ms) ease;
  }

  .section-tile.on .tile-check {
    background: var(--accent);
    border-color: var(--accent);
  }

  .section-tile.on .tile-check::after {
    content: '';
    position: absolute;
    top: 3px;
    left: 5px;
    width: 4px;
    height: 8px;
    border-right: 2px solid var(--text-on-accent);
    border-bottom: 2px solid var(--text-on-accent);
    transform: rotate(45deg);
  }

  /* ─── Format picker (right column top half) ─── */
  .format-picker {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin-bottom: 20px;
  }

  .format-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 14px 12px 12px;
    background: var(--surface-float);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    cursor: pointer;
    color: var(--text-secondary);
    transition: background var(--motion-fast, 100ms) ease,
                border-color var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .format-card:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .format-card.active {
    background: var(--surface-float);
    border-color: var(--accent);
    color: var(--text-primary);
    box-shadow: 0 0 0 2px var(--accent-muted) inset;
  }

  .format-art {
    color: var(--text-muted);
    transition: color var(--motion-fast, 100ms) ease;
  }

  .format-card.active .format-art {
    color: var(--accent);
  }

  .format-meta {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .format-name {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .format-desc {
    font-size: 10.5px;
    color: var(--text-muted);
  }

  .format-empty {
    margin-bottom: 20px;
    padding: 14px 14px;
    background: var(--surface-float);
    border: 1px dashed var(--border-medium);
    border-radius: 8px;
    font-size: 11.5px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .format-empty p { margin: 0; }

  /* ─── Options block ─── */
  .options-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .options-title {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--text-muted);
    padding: 0 0 8px;
  }

  .opt-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 4px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .opt-row:last-of-type { border-bottom: none; }

  .opt-label {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .opt-name {
    font-size: 12px;
    color: var(--text-primary);
    font-weight: 500;
  }

  .opt-desc {
    font-size: 10.5px;
    color: var(--text-muted);
    line-height: 1.3;
  }

  /* iOS-style toggle. Compact (28×16) so the row reads as a setting rather
     than a button cluster. */
  .opt-toggle {
    flex-shrink: 0;
    width: 28px;
    height: 16px;
    border: none;
    border-radius: 999px;
    background: var(--border-medium);
    padding: 0;
    cursor: pointer;
    position: relative;
    transition: background var(--motion-fast, 100ms) ease;
  }

  .opt-toggle[aria-checked='true'] {
    background: var(--accent);
  }

  .opt-thumb {
    display: block;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--surface-float);
    position: absolute;
    top: 2px;
    left: 2px;
    transition: transform var(--motion-fast, 100ms) ease;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.18);
  }

  .opt-toggle[aria-checked='true'] .opt-thumb {
    transform: translateX(12px);
  }

  /* ─── Scene range row ─── */
  .range-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 4px 8px;
    font-size: 11.5px;
    color: var(--text-secondary);
  }

  .range-label {
    font-size: 10.5px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .range-row input[type='number'] {
    width: 56px;
    padding: 4px 6px;
    font-size: 12px;
    font-family: inherit;
    color: var(--text-primary);
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 5px;
    font-variant-numeric: tabular-nums;
  }

  .range-row input[type='number']:focus {
    outline: none;
    border-color: var(--accent);
  }

  .range-sep {
    color: var(--text-muted);
  }

  .range-hint {
    font-size: 10.5px;
    color: var(--text-muted);
    margin-left: 2px;
  }

  /* ─── Footer ─── */
  .error-message {
    margin: 0 28px;
    padding: 8px 12px;
    font-size: 11.5px;
    color: var(--error);
    background: rgba(192, 87, 74, 0.1);
    border-radius: 6px;
  }

  .export-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 28px;
    border-top: 1px solid var(--border-subtle);
    background: var(--surface-float);
  }

  .footer-spacer {
    flex: 1;
  }

  .btn-ghost {
    height: 32px;
    padding: 0 14px;
    border-radius: 6px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--ui-font);
    font-size: 12px;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }

  .btn-ghost:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .btn-secondary {
    height: 32px;
    padding: 0 12px;
    border-radius: 6px;
    border: 1px solid var(--border-medium);
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--ui-font);
    font-size: 12px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text-primary);
    border-color: var(--text-muted);
  }

  .btn-primary {
    height: 32px;
    padding: 0 16px;
    border-radius: 6px;
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--text-on-accent);
    font-family: var(--ui-font);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: background 120ms ease, border-color 120ms ease;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .btn-primary:disabled,
  .btn-secondary:disabled,
  .btn-ghost:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
