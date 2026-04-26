<script lang="ts">
  import { untrack } from 'svelte';
  import { save } from '@tauri-apps/plugin-dialog';
  import { writeFile } from '@tauri-apps/plugin-fs';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { focusTrap } from '$lib/actions/focusTrap';

  let { open = $bindable(false) } = $props<{ open: boolean }>();

  interface CharacterStat {
    name: string;
    scenes: number;
    dialogueBlocks: number;
    percentage: number;
  }

  /** One row in the Locations report — aggregated across the script. */
  interface LocationStat {
    name: string;
    scenes: number;
    /** "Interior", "Exterior", or "Both" — derived from the headings that
     *  used this location string. */
    setting: 'Interior' | 'Exterior' | 'Both';
  }

  /** One row in the Schedule report — listed in document order so the
   *  writer can plan a sequence (or sort by location to group shoots).
   *  `locationGroup` and `scheduledDate` come from the SceneCard (#124).
   *  `eighths` mirrors the PDF shoot list math: 1 page ≈ 8 eighths ≈
   *  3000 chars, so ~375 chars per eighth. */
  interface ScheduleEntry {
    sceneNumber: number;
    setting: string;
    location: string;
    time: string;
    characterCount: number;
    locationGroup: string;
    scheduledDate: string;
    eighths: number;
  }

  interface Stats {
    pageCount: number;
    sceneCount: number;
    wordCount: number;
    dialogueLineCount: number;
    intCount: number;
    extCount: number;
    dayCount: number;
    nightCount: number;
    screenTimeMinutes: number;
    characters: CharacterStat[];
    locations: LocationStat[];
    schedule: ScheduleEntry[];
  }

  /** One row in the Episodes report (series only) — per-episode summary
   *  numbers, used by the Episodes tab to give a season-at-a-glance view.
   *  Mirrors the most useful fields from the Overview hero stats so a
   *  showrunner can spot a too-short episode or an arc imbalance fast. */
  interface EpisodeStat {
    number: number;
    title: string;
    status: string;
    pageCount: number;
    sceneCount: number;
    wordCount: number;
    dialogueLineCount: number;
    screenTimeMinutes: number;
    intCount: number;
    extCount: number;
    characterCount: number;
  }

  type Tab = 'overview' | 'characters' | 'locations' | 'schedule' | 'episodes';
  let activeTab = $state<Tab>('overview');

  // ─── Scope (series only) ────────────────────────────────────────────
  // Default 'episode' preserves the legacy behaviour — stats reflect the
  // episode the writer is editing. 'series' aggregates every episode into
  // one merged document and runs the same stats math on it. The toggle
  // is hidden for film projects (no scope concept applies).
  let statsScope = $state<'episode' | 'series'>('episode');
  let isSeriesProject = $derived(documentStore.isSeries);

  // Reset scope to 'episode' whenever the modal opens — fresh session,
  // and a series writer who switched episodes between visits expects
  // the active-episode default each time.
  $effect(() => {
    if (open) statsScope = 'episode';
  });

  // ─── Per-table sort state (#135) ────────────────────────────────────
  // Each table starts in its computed default order (sortKey === null).
  // Clicking a header cycles: default → asc → desc → default. State is
  // session-only — reset every time the modal opens so the writer always
  // sees the recommended ordering first.
  type CharSortKey = 'name' | 'scenes' | 'dialogueBlocks' | 'percentage';
  type LocSortKey = 'name' | 'scenes' | 'setting';
  type SchedSortKey =
    | 'sceneNumber'
    | 'setting'
    | 'location'
    | 'time'
    | 'characterCount'
    | 'locationGroup'
    | 'scheduledDate';
  type SortDir = 'asc' | 'desc';

  type EpSortKey =
    | 'number'
    | 'title'
    | 'status'
    | 'pageCount'
    | 'sceneCount'
    | 'wordCount'
    | 'dialogueLineCount'
    | 'screenTimeMinutes'
    | 'characterCount';

  let charSortKey = $state<CharSortKey | null>(null);
  let charSortDir = $state<SortDir>('asc');
  let locSortKey = $state<LocSortKey | null>(null);
  let locSortDir = $state<SortDir>('asc');
  let schedSortKey = $state<SchedSortKey | null>(null);
  let schedSortDir = $state<SortDir>('asc');
  let epSortKey = $state<EpSortKey | null>(null);
  let epSortDir = $state<SortDir>('asc');

  function cycleCharSort(key: CharSortKey) {
    if (charSortKey !== key) { charSortKey = key; charSortDir = 'asc'; }
    else if (charSortDir === 'asc') { charSortDir = 'desc'; }
    else { charSortKey = null; charSortDir = 'asc'; }
  }
  function cycleLocSort(key: LocSortKey) {
    if (locSortKey !== key) { locSortKey = key; locSortDir = 'asc'; }
    else if (locSortDir === 'asc') { locSortDir = 'desc'; }
    else { locSortKey = null; locSortDir = 'asc'; }
  }
  function cycleSchedSort(key: SchedSortKey) {
    if (schedSortKey !== key) { schedSortKey = key; schedSortDir = 'asc'; }
    else if (schedSortDir === 'asc') { schedSortDir = 'desc'; }
    else { schedSortKey = null; schedSortDir = 'asc'; }
  }
  function cycleEpSort(key: EpSortKey) {
    if (epSortKey !== key) { epSortKey = key; epSortDir = 'asc'; }
    else if (epSortDir === 'asc') { epSortDir = 'desc'; }
    else { epSortKey = null; epSortDir = 'asc'; }
  }

  /** Stable comparator: numbers compared numerically, strings via
   *  localeCompare. Returns -1/0/1 in the asc direction. */
  function cmp(a: string | number, b: string | number): number {
    if (typeof a === 'number' && typeof b === 'number') {
      return a < b ? -1 : a > b ? 1 : 0;
    }
    return String(a).localeCompare(String(b), undefined, { numeric: true, sensitivity: 'base' });
  }

  /** Label pair surfaced in the right pane's header — eyebrow gives the
   *  category, title gives the human-readable name. Keeps the right-pane
   *  geometry stable while signalling which view is active. */
  // Pane titles get an HTML <em> slot so the mh-title rule colorises one
   // word per heading — same editorial accent as the marketing site.
  let activeTabLabel = $derived.by<{ eyebrow: string; title: string }>(() => {
    switch (activeTab) {
      case 'overview':   return { eyebrow: 'At a glance', title: 'The <em>script</em> at a glance' };
      case 'characters': return { eyebrow: 'Cast report', title: 'Characters by <em>dialogue</em>' };
      case 'locations':  return { eyebrow: 'Production prep', title: 'Where it <em>happens</em>' };
      case 'schedule':   return { eyebrow: 'Production prep', title: 'The <em>shoot</em> schedule' };
      case 'episodes':   return { eyebrow: 'Series report',   title: 'Every <em>episode</em>, side by side' };
    }
  });

  let stats = $state<Stats>(untrack(() => computeStats()));
  let episodeStats = $state<EpisodeStat[]>(untrack(() => computeEpisodeStats()));

  // Recompute stats only when the modal opens — not on every keystroke while
  // it's open. The $effect previously read `documentStore.activeContent`
  // transitively through computeStats(), which tracked it as a dependency
  // and re-ran the full ~2000-node walk on every typed character. Wrapping
  // the call in untrack() ensures the only reactive dep is `open` (#100).
  // The Refresh button gives the writer an explicit way to re-snapshot.
  $effect(() => {
    if (open) {
      untrack(() => {
        stats = computeStats();
        episodeStats = computeEpisodeStats();
      });
      // Reset sorts on open so the writer always sees the default
      // ordering first — sort choices are session-only (#135).
      charSortKey = null; charSortDir = 'asc';
      locSortKey = null; locSortDir = 'asc';
      schedSortKey = null; schedSortDir = 'asc';
      epSortKey = null; epSortDir = 'asc';
    }
  });

  // Flipping the scope toggle is an explicit user action, so we *do*
  // want to recompute on change — but still wrap in untrack() so we
  // don't pull the document content into the reactive graph. We track
  // statsScope explicitly by referencing it before the untrack() call.
  $effect(() => {
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    statsScope;
    if (open) {
      untrack(() => { stats = computeStats(); });
    }
  });

  // Derived sorted views — the original arrays come back when sortKey
  // is null (default order from computeStats: characters by dialogue
  // desc, locations by scenes desc, schedule by document order).
  let sortedCharacters = $derived.by(() => {
    if (!charSortKey) return stats.characters;
    const arr = [...stats.characters];
    const key = charSortKey;
    const dir = charSortDir === 'asc' ? 1 : -1;
    arr.sort((a, b) => cmp(a[key], b[key]) * dir);
    return arr;
  });

  let sortedLocations = $derived.by(() => {
    if (!locSortKey) return stats.locations;
    const arr = [...stats.locations];
    const key = locSortKey;
    const dir = locSortDir === 'asc' ? 1 : -1;
    arr.sort((a, b) => cmp(a[key], b[key]) * dir);
    return arr;
  });

  let sortedSchedule = $derived.by(() => {
    if (!schedSortKey) return stats.schedule;
    const arr = [...stats.schedule];
    const key = schedSortKey;
    const dir = schedSortDir === 'asc' ? 1 : -1;
    arr.sort((a, b) => cmp(a[key], b[key]) * dir);
    return arr;
  });

  let sortedEpisodes = $derived.by(() => {
    if (!epSortKey) return episodeStats;
    const arr = [...episodeStats];
    const key = epSortKey;
    const dir = epSortDir === 'asc' ? 1 : -1;
    arr.sort((a, b) => cmp(a[key], b[key]) * dir);
    return arr;
  });

  /** Maps a sortKey + dir to the aria-sort value the table cell needs. */
  function ariaSortFor(active: boolean, dir: SortDir): 'none' | 'ascending' | 'descending' {
    if (!active) return 'none';
    return dir === 'asc' ? 'ascending' : 'descending';
  }

  // ─── Grouped schedule view (#136) ───────────────────────────────────
  // The Schedule tab's default render is grouped by shoot date — mirrors
  // the printed Daily Shoot List PDF (#124) so the writer can preview
  // the same structure. Once the writer activates a column sort the
  // table flips to a flat sorted view (grouping by date wouldn't
  // preserve the chosen ordering).
  interface ScheduleGroup {
    date: string;
    label: string;
    rows: ScheduleEntry[];
    totalEighths: number;
  }

  function formatDateLabel(iso: string): string {
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso);
    if (!m) return iso;
    const d = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
    if (Number.isNaN(d.getTime())) return iso;
    return d.toLocaleDateString(undefined, {
      weekday: 'short',
      day: 'numeric',
      month: 'long',
      year: 'numeric',
    });
  }

  function formatDayTotal(eighths: number): string {
    if (eighths === 0) return '';
    const pages = (eighths / 8).toFixed(1);
    return `${eighths} eighth${eighths === 1 ? '' : 's'} · ~${pages} pages`;
  }

  // ─── CSV export (#137) ──────────────────────────────────────────────
  // RFC 4180-compliant: comma-separated, fields with commas/quotes/
  // newlines double-quoted, embedded quotes doubled. UTF-8 BOM prepended
  // so Excel decodes Malayalam columns correctly.
  function csvEscape(value: string | number): string {
    const s = String(value);
    if (/[",\n\r]/.test(s)) return '"' + s.replace(/"/g, '""') + '"';
    return s;
  }

  function buildCsv(rows: Array<Array<string | number>>): string {
    return '﻿' + rows.map((r) => r.map(csvEscape).join(',')).join('\r\n');
  }

  /** Sanitize the project title into a filename-safe stem. Falls back
   *  to "screenplay" if the title is empty. */
  function fileStem(): string {
    const t = (documentStore.activeMeta?.title ?? '').trim();
    if (!t) return 'screenplay';
    return t.replace(/[\\/:*?"<>|]/g, '').replace(/\s+/g, ' ').trim() || 'screenplay';
  }

  async function saveCsv(suffix: string, csv: string) {
    const path = await save({
      defaultPath: `${fileStem()}-${suffix}.csv`,
      filters: [{ name: 'CSV', extensions: ['csv'] }],
    });
    if (!path) return;
    const encoder = new TextEncoder();
    await writeFile(path, encoder.encode(csv));
  }

  async function exportCharactersCsv() {
    const rows: Array<Array<string | number>> = [
      ['Rank', 'Character', 'Scenes', 'Dialogue', 'Share %'],
    ];
    sortedCharacters.forEach((c, i) => {
      rows.push([i + 1, c.name, c.scenes, c.dialogueBlocks, c.percentage]);
    });
    await saveCsv('characters', buildCsv(rows));
  }

  async function exportLocationsCsv() {
    const rows: Array<Array<string | number>> = [
      ['Rank', 'Location', 'Scenes', 'Setting'],
    ];
    sortedLocations.forEach((l, i) => {
      rows.push([i + 1, l.name, l.scenes, l.setting]);
    });
    await saveCsv('locations', buildCsv(rows));
  }

  async function exportScheduleCsv() {
    const rows: Array<Array<string | number>> = [
      ['#', 'Setting', 'Location', 'Time', 'Cast', 'Group', 'Day', 'Eighths'],
    ];
    // Mirror what's on screen: when sorting is active, export the
    // sorted view; otherwise export the grouped view in its rendered
    // order (date-grouped, with unscheduled last).
    const source = schedSortKey === null
      ? groupedSchedule.flatMap((g) => g.rows)
      : sortedSchedule;
    for (const r of source) {
      rows.push([
        r.sceneNumber,
        r.setting,
        r.location,
        r.time,
        r.characterCount,
        r.locationGroup,
        r.scheduledDate,
        r.eighths,
      ]);
    }
    await saveCsv('schedule', buildCsv(rows));
  }

  async function exportEpisodesCsv() {
    const rows: Array<Array<string | number>> = [
      ['#', 'Title', 'Status', 'Pages', 'Scenes', 'Words', 'Dialogue', 'Screen time', 'INT', 'EXT', 'Cast'],
    ];
    sortedEpisodes.forEach((e) => {
      rows.push([
        e.number,
        e.title,
        e.status,
        e.pageCount,
        e.sceneCount,
        e.wordCount,
        e.dialogueLineCount,
        e.screenTimeMinutes,
        e.intCount,
        e.extCount,
        e.characterCount,
      ]);
    });
    await saveCsv('episodes', buildCsv(rows));
  }

  let canExportCsv = $derived(
    activeTab === 'characters' ||
    activeTab === 'locations' ||
    activeTab === 'schedule' ||
    activeTab === 'episodes',
  );

  async function exportActiveCsv() {
    if (activeTab === 'characters') await exportCharactersCsv();
    else if (activeTab === 'locations') await exportLocationsCsv();
    else if (activeTab === 'schedule') await exportScheduleCsv();
    else if (activeTab === 'episodes') await exportEpisodesCsv();
  }

  let groupedSchedule = $derived.by<ScheduleGroup[]>(() => {
    const groups = new Map<string, ScheduleEntry[]>();
    for (const row of stats.schedule) {
      const key = row.scheduledDate || '';
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push(row);
    }
    // Dated groups by ISO ascending, unscheduled last.
    const keys = Array.from(groups.keys()).sort((a, b) => {
      if (a === '' && b === '') return 0;
      if (a === '') return 1;
      if (b === '') return -1;
      return a.localeCompare(b);
    });
    return keys.map((k) => {
      const rows = groups.get(k)!;
      return {
        date: k,
        label: k ? formatDateLabel(k) : 'Unscheduled',
        rows,
        totalEighths: rows.reduce((s, r) => s + r.eighths, 0),
      };
    });
  });

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

  function refresh() {
    stats = computeStats();
    episodeStats = computeEpisodeStats();
  }

  /** Pull location and time-of-day out of a scene heading. Mirrors the
   *  parser used by SceneCardsView so the two views agree on what counts
   *  as "the location" versus "the time" for each scene. */
  function parseHeading(heading: string): { setting: string; location: string; time: string } {
    const trimmed = heading.trim();
    let setting = '';
    let rest = trimmed;
    const upper = trimmed.toUpperCase();
    if (upper.startsWith('INT./EXT.') || upper.startsWith('INT/EXT')) {
      setting = 'INT./EXT.';
      rest = trimmed.replace(/^(INT\.?\/EXT\.?|I\/E)\s*/i, '');
    } else if (upper.startsWith('INT.') || upper.startsWith('INT ')) {
      setting = 'INT.';
      rest = trimmed.replace(/^INT\.?\s*/i, '');
    } else if (upper.startsWith('EXT.') || upper.startsWith('EXT ')) {
      setting = 'EXT.';
      rest = trimmed.replace(/^EXT\.?\s*/i, '');
    }
    // Split on the rightmost dash (locations themselves can contain dashes).
    const dashIdx = rest.lastIndexOf(' - ');
    if (dashIdx > 0) {
      return {
        setting,
        location: rest.slice(0, dashIdx).trim(),
        time: rest.slice(dashIdx + 3).trim(),
      };
    }
    return { setting, location: rest, time: '' };
  }

  /** Build the inputs (content + scene-cards) for the current scope.
   *  - episode scope (or film): the active episode's content + cards.
   *  - series scope: every episode's content nodes concatenated, with
   *    each episode's scene_cards rebased so scene_index points to the
   *    correct position in the merged document. */
  function buildScopeInputs(): {
    content: { content?: Array<{ type?: string; content?: Array<{ text?: string }> }> } | null;
    cards: Array<{ scene_index: number; scheduled_date?: string; location_group?: string }>;
  } {
    if (isSeriesProject && statsScope === 'series') {
      const eps = documentStore.document?.series?.episodes ?? [];
      const merged: Array<{ type?: string; content?: Array<{ text?: string }> }> = [];
      const cards: Array<{ scene_index: number; scheduled_date?: string; location_group?: string }> = [];
      let sceneOffset = 0;
      for (const ep of eps) {
        const c = ep.content as { content?: Array<{ type?: string; content?: Array<{ text?: string }> }> } | null;
        const nodes = c?.content ?? [];
        merged.push(...nodes);
        // Rebase each card's scene_index against the merged doc.
        for (const card of ep.scene_cards) {
          cards.push({
            scene_index: card.scene_index + sceneOffset,
            scheduled_date: card.scheduled_date,
            location_group: card.location_group,
          });
        }
        // Count scene_headings in this episode so the next one's cards
        // get the right offset.
        for (const n of nodes) if (n.type === 'scene_heading') sceneOffset++;
      }
      return { content: { content: merged }, cards };
    }

    // Default — episode (series) or film. Use the active content.
    const activeContent = documentStore.activeContent;
    return {
      content: activeContent as { content?: Array<{ type?: string; content?: Array<{ text?: string }> }> } | null,
      cards: documentStore.activeSceneCards.map((c) => ({
        scene_index: c.scene_index,
        scheduled_date: c.scheduled_date,
        location_group: c.location_group,
      })),
    };
  }

  function computeStats(): Stats {
    const { content, cards } = buildScopeInputs();
    return computeStatsFor(content, cards);
  }

  /** Per-episode summary table (series only). For films, returns an
   *  empty array — the Episodes tab is hidden in that case anyway. */
  function computeEpisodeStats(): EpisodeStat[] {
    if (!isSeriesProject) return [];
    const eps = documentStore.document?.series?.episodes ?? [];
    return eps.map((ep): EpisodeStat => {
      const epContent = ep.content as { content?: Array<{ type?: string; content?: Array<{ text?: string }> }> } | null;
      const epCards = ep.scene_cards.map((c) => ({
        scene_index: c.scene_index,
        scheduled_date: c.scheduled_date,
        location_group: c.location_group,
      }));
      const s = computeStatsFor(epContent, epCards);
      return {
        number: ep.number,
        title: ep.title?.trim() || '',
        status: ep.status ?? 'draft',
        pageCount: s.pageCount,
        sceneCount: s.sceneCount,
        wordCount: s.wordCount,
        dialogueLineCount: s.dialogueLineCount,
        screenTimeMinutes: s.screenTimeMinutes,
        intCount: s.intCount,
        extCount: s.extCount,
        characterCount: s.characters.length,
      };
    });
  }

  /** Pure stats walker — runs against any (content, cards) pair. The
   *  scope-aware wrapper computeStats() chooses what to feed in based on
   *  the current statsScope; the Episodes tab calls this in a loop with
   *  one episode's content at a time. */
  function computeStatsFor(
    content: { content?: Array<{ type?: string; content?: Array<{ text?: string }> }> } | null,
    sceneCards: Array<{ scene_index: number; scheduled_date?: string; location_group?: string }>,
  ): Stats {
    const empty: Stats = {
      pageCount: 0, sceneCount: 0, wordCount: 0, dialogueLineCount: 0,
      intCount: 0, extCount: 0, dayCount: 0, nightCount: 0,
      screenTimeMinutes: 0, characters: [], locations: [], schedule: [],
    };

    if (!content?.content) return empty;

    const nodes = content.content;

    let totalChars = 0;
    let sceneCount = 0;
    let wordCount = 0;
    let dialogueLineCount = 0;
    let intCount = 0;
    let extCount = 0;
    let dayCount = 0;
    let nightCount = 0;

    // Per-character tracking
    const charDialogueCount = new Map<string, number>();
    const charScenes = new Map<string, Set<number>>();
    let currentScene = 0;

    // Per-location tracking for the Locations report (#119).
    // Key: location name (case-preserved as written, but compared
    // case-insensitively so "ROOM" and "Room" merge). Value: scene
    // count + a Set of settings ("INT.", "EXT.") that location appeared in.
    interface LocationAcc {
      displayName: string;
      scenes: number;
      settings: Set<string>;
    }
    const locationAccs = new Map<string, LocationAcc>();

    // Per-scene character tracking — needed for the Schedule report's
    // characterCount column. We can't reuse charScenes because that one
    // is keyed by character.
    const sceneCharacters = new Map<number, Set<string>>();
    // Per-scene char-count, used to derive page-eighths for each scene
    // (mirrors buildShootListData in ExportModal).
    const sceneCharCount = new Map<number, number>();
    const schedule: ScheduleEntry[] = [];

    // Pull SceneCard scheduling fields up-front so the per-scene loop
    // can attach them by 0-based index. sceneCards is small (≤ scene
    // count); a Map is cheaper to look up per scene than a linear find. (#124)
    const cardsByIndex = new Map<number, { scheduled_date: string; location_group: string }>();
    for (const c of sceneCards) {
      cardsByIndex.set(c.scene_index, {
        scheduled_date: c.scheduled_date ?? '',
        location_group: c.location_group ?? '',
      });
    }

    for (const node of nodes) {
      const type = node.type;
      const text = (node.content ?? []).map((c) => c.text ?? '').join('');

      // Count total characters for page estimate
      totalChars += text.length;

      // Count words — split on whitespace, filter empty
      if (text.trim().length > 0) {
        wordCount += text.trim().split(/\s+/).length;
      }

      if (type === 'scene_heading') {
        sceneCount++;
        currentScene = sceneCount;

        const upper = text.toUpperCase();

        // INT/EXT counts
        if (upper.startsWith('INT.') || upper.startsWith('INT ')) {
          intCount++;
        }
        if (upper.startsWith('EXT.') || upper.startsWith('EXT ')) {
          extCount++;
        }
        // INT./EXT. counts as both
        if (upper.startsWith('INT./EXT.') || upper.startsWith('INT/EXT')) {
          intCount++;
          extCount++;
        }

        // Day/Night counts — look for time keywords anywhere in the heading
        const dayPattern = /\b(DAY|DAWN|MORNING)\b/;
        const nightPattern = /\b(NIGHT|DUSK|EVENING)\b/;
        if (dayPattern.test(upper)) dayCount++;
        if (nightPattern.test(upper)) nightCount++;

        // Locations + Schedule — parse the heading once and feed both reports.
        const parsed = parseHeading(text);
        if (parsed.location.length > 0) {
          const key = parsed.location.toLowerCase();
          let acc = locationAccs.get(key);
          if (!acc) {
            acc = { displayName: parsed.location, scenes: 0, settings: new Set() };
            locationAccs.set(key, acc);
          }
          acc.scenes++;
          if (parsed.setting) acc.settings.add(parsed.setting);
        }
        const card = cardsByIndex.get(sceneCount - 1);
        schedule.push({
          sceneNumber: sceneCount,
          setting: parsed.setting,
          location: parsed.location,
          time: parsed.time,
          characterCount: 0, // filled below from sceneCharacters
          locationGroup: card?.location_group ?? '',
          scheduledDate: card?.scheduled_date ?? '',
          eighths: 0, // filled below from sceneCharCount
        });
        // Seed the count for this scene with the heading's chars so a
        // heading-only scene still registers a non-trivial size.
        sceneCharCount.set(currentScene, text.length);
      } else if (currentScene > 0) {
        sceneCharCount.set(
          currentScene,
          (sceneCharCount.get(currentScene) ?? 0) + text.length,
        );
      }

      if (type === 'character') {
        const name = text.trim();
        if (name.length > 0) {
          // Track dialogue blocks per character
          charDialogueCount.set(name, (charDialogueCount.get(name) ?? 0) + 1);

          // Track scenes per character
          if (!charScenes.has(name)) {
            charScenes.set(name, new Set());
          }
          charScenes.get(name)!.add(currentScene);

          // Track per-scene speakers (for the Schedule report's count).
          if (!sceneCharacters.has(currentScene)) {
            sceneCharacters.set(currentScene, new Set());
          }
          sceneCharacters.get(currentScene)!.add(name);
        }
      }

      if (type === 'dialogue') {
        dialogueLineCount++;
      }
    }

    // Backfill characterCount + eighths per schedule row now that the
    // walk is done. Eighths math mirrors buildShootListData: 1 page ≈
    // 8 eighths ≈ 3000 chars, so 1 eighth ≈ 375 chars (min 1 eighth so
    // a near-empty scene still occupies a row).
    for (const row of schedule) {
      row.characterCount = sceneCharacters.get(row.sceneNumber)?.size ?? 0;
      const chars = sceneCharCount.get(row.sceneNumber) ?? 0;
      row.eighths = Math.max(1, Math.round(chars / 375));
    }

    // Build per-character stats sorted by dialogue count descending
    const totalDialogue = dialogueLineCount || 1; // avoid division by zero
    const characters: CharacterStat[] = Array.from(charDialogueCount.entries())
      .map(([name, dialogueBlocks]) => ({
        name,
        scenes: charScenes.get(name)?.size ?? 0,
        dialogueBlocks,
        percentage: Math.round((dialogueBlocks / totalDialogue) * 100),
      }))
      .sort((a, b) => b.dialogueBlocks - a.dialogueBlocks);

    const pageCount = Math.max(1, Math.ceil(totalChars / 3000));

    // Sort locations by scene count desc — most-shot locations first,
    // which is what a producer scanning for scheduling priorities wants.
    const locations: LocationStat[] = Array.from(locationAccs.values())
      .map((a): LocationStat => {
        const setting: 'Interior' | 'Exterior' | 'Both' =
          a.settings.size === 0
            ? 'Interior'
            : a.settings.has('INT.') && a.settings.has('EXT.')
              ? 'Both'
              : a.settings.has('EXT.')
                ? 'Exterior'
                : 'Interior';
        return { name: a.displayName, scenes: a.scenes, setting };
      })
      .sort((a, b) => b.scenes - a.scenes);

    return {
      pageCount,
      sceneCount,
      wordCount,
      dialogueLineCount,
      intCount,
      extCount,
      dayCount,
      nightCount,
      screenTimeMinutes: pageCount,
      characters,
      locations,
      schedule,
    };
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="modal-backdrop" onclick={handleBackdropClick} onkeydown={handleKeydown} role="dialog" aria-modal="true" tabindex="-1">
    <div class="modal-card stats-card" use:focusTrap>
      <!-- Left rail: vertical tab nav. Fixed width keeps the right pane's
           geometry stable across tab switches (no jumpy resize when going
           from 5 stats to a 100-row table). Each item shows an inline
           icon + label + count badge so the eye scans the rail like a
           table-of-contents. -->
      <aside class="stats-rail">
        <div class="rail-header">
          <h2>Statistics</h2>
        </div>

        {#if isSeriesProject}
          <!-- Scope toggle (series only). Mirrors the Export modal's
               "This episode / Full series" segmented control so the
               two surfaces speak the same vocabulary about scope. The
               labels swap at this size — "Episode" / "Series" — to fit
               the narrower rail without losing meaning. -->
          <div class="scope-segmented" role="group" aria-label="Statistics scope">
            <button
              type="button"
              class="scope-seg"
              class:active={statsScope === 'episode'}
              onclick={() => { statsScope = 'episode'; }}
              title="Stats for the active episode"
            >Episode</button>
            <button
              type="button"
              class="scope-seg"
              class:active={statsScope === 'series'}
              onclick={() => { statsScope = 'series'; }}
              title="Stats aggregated across every episode"
            >Series</button>
          </div>
        {/if}

        <div class="rail-nav" role="tablist" aria-label="Statistics views">
          <button
            class="rail-item"
            class:active={activeTab === 'overview'}
            role="tab"
            aria-selected={activeTab === 'overview'}
            onclick={() => { activeTab = 'overview'; }}
          >
            <svg class="rail-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 18 L8 13 L12 16 L21 6"/>
              <path d="M21 12 V6 H15"/>
            </svg>
            <span class="rail-label">Overview</span>
          </button>
          <button
            class="rail-item"
            class:active={activeTab === 'characters'}
            role="tab"
            aria-selected={activeTab === 'characters'}
            onclick={() => { activeTab = 'characters'; }}
          >
            <svg class="rail-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="9" cy="8" r="3.2"/>
              <path d="M2 20 c1.5 -3.5 5 -5 7 -5 s5.5 1.5 7 5"/>
              <circle cx="17" cy="9" r="2.4"/>
            </svg>
            <span class="rail-label">Characters</span>
            <span class="rail-count">{stats.characters.length}</span>
          </button>
          <button
            class="rail-item"
            class:active={activeTab === 'locations'}
            role="tab"
            aria-selected={activeTab === 'locations'}
            onclick={() => { activeTab = 'locations'; }}
          >
            <svg class="rail-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 22 c-5 -7 -7 -10 -7 -13 a7 7 0 0 1 14 0 c0 3 -2 6 -7 13 z"/>
              <circle cx="12" cy="9" r="2.4"/>
            </svg>
            <span class="rail-label">Locations</span>
            <span class="rail-count">{stats.locations.length}</span>
          </button>
          <button
            class="rail-item"
            class:active={activeTab === 'schedule'}
            role="tab"
            aria-selected={activeTab === 'schedule'}
            onclick={() => { activeTab = 'schedule'; }}
          >
            <svg class="rail-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="5" width="18" height="16" rx="2"/>
              <path d="M3 10 H21"/>
              <path d="M8 3 V7 M16 3 V7"/>
            </svg>
            <span class="rail-label">Schedule</span>
            <span class="rail-count">{stats.schedule.length}</span>
          </button>
          {#if isSeriesProject}
            <!-- Episodes tab — series only. Per-episode summary so a
                 showrunner can spot a too-short episode at a glance.
                 Always renders against the full series regardless of
                 the scope toggle (the table itself is the season view). -->
            <button
              class="rail-item"
              class:active={activeTab === 'episodes'}
              role="tab"
              aria-selected={activeTab === 'episodes'}
              onclick={() => { activeTab = 'episodes'; }}
            >
              <svg class="rail-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                <rect x="3"  y="5"  width="6" height="14" rx="1"/>
                <rect x="11" y="5"  width="6" height="14" rx="1"/>
                <rect x="19" y="5"  width="2" height="14" rx="1"/>
              </svg>
              <span class="rail-label">Episodes</span>
              <span class="rail-count">{episodeStats.length}</span>
            </button>
          {/if}
        </div>
        <div class="rail-footer">
          <button class="rail-action" onclick={refresh} title="Re-snapshot the document">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 12 a9 9 0 1 1 -2.6 -6.4"/>
              <path d="M21 4 V10 H15"/>
            </svg>
            <span>Refresh</span>
          </button>
        </div>
      </aside>

      <!-- Right pane: stable shell, scrollable content. The pane takes the
           full remaining width and never shrinks/grows when switching tabs
           — fixes the jumpy resize the original design had (#108 sizing). -->
      <section class="stats-pane">
        <header class="pane-header">
          <div class="pane-title-block">
            <div class="mh-eyebrow" aria-hidden="true">
              <span class="mh-rule"></span>
              <span>{activeTabLabel.eyebrow}</span>
            </div>
            <h3 class="mh-title pane-title">{@html activeTabLabel.title}</h3>
          </div>
          <div class="pane-actions">
            {#if canExportCsv}
              <button
                class="btn-csv"
                type="button"
                onclick={exportActiveCsv}
                title="Export this report as CSV"
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <path d="M12 3 V15"/>
                  <path d="M7 10 L12 15 L17 10"/>
                  <path d="M5 21 H19"/>
                </svg>
                <span>Export CSV</span>
              </button>
            {/if}
            <button class="btn-close" onclick={() => { open = false; }} aria-label="Close statistics">&times;</button>
          </div>
        </header>

        <div class="pane-content">
          {#if activeTab === 'overview'}
            {@const totalSetting = stats.intCount + stats.extCount || 1}
            {@const totalTime = stats.dayCount + stats.nightCount || 1}
            <!-- Five hero stats up top — bigger than the original 480px modal
                 could afford. Three columns let the larger numbers breathe. -->
            <div class="hero-grid">
              <div class="hero-stat">
                <span class="hero-value">{stats.pageCount}</span>
                <span class="hero-label">Pages</span>
              </div>
              <div class="hero-stat">
                <span class="hero-value">{stats.sceneCount}</span>
                <span class="hero-label">Scenes</span>
              </div>
              <div class="hero-stat">
                <span class="hero-value">{stats.wordCount.toLocaleString()}</span>
                <span class="hero-label">Words</span>
              </div>
              <div class="hero-stat">
                <span class="hero-value">{stats.dialogueLineCount}</span>
                <span class="hero-label">Dialogue blocks</span>
              </div>
              <div class="hero-stat">
                <span class="hero-value">~{stats.screenTimeMinutes}<span class="hero-unit">min</span></span>
                <span class="hero-label">Estimated screen time</span>
              </div>
            </div>

            <div class="breakdown-block">
              <div class="block-heading">
                <span class="block-rule"></span>
                <span class="block-title">Scene breakdown</span>
                <span class="block-rule"></span>
              </div>
              <div class="breakdown-bars">
                <div class="bar-pair">
                  <div class="bar-row">
                    <span class="bar-label">Interior</span>
                    <span class="bar-track"><span class="bar-fill" style="width: {(stats.intCount / totalSetting) * 100}%"></span></span>
                    <span class="bar-value">{stats.intCount}</span>
                  </div>
                  <div class="bar-row">
                    <span class="bar-label">Exterior</span>
                    <span class="bar-track"><span class="bar-fill alt" style="width: {(stats.extCount / totalSetting) * 100}%"></span></span>
                    <span class="bar-value">{stats.extCount}</span>
                  </div>
                </div>
                <div class="bar-pair">
                  <div class="bar-row">
                    <span class="bar-label">Day</span>
                    <span class="bar-track"><span class="bar-fill warm" style="width: {(stats.dayCount / totalTime) * 100}%"></span></span>
                    <span class="bar-value">{stats.dayCount}</span>
                  </div>
                  <div class="bar-row">
                    <span class="bar-label">Night</span>
                    <span class="bar-track"><span class="bar-fill cool" style="width: {(stats.nightCount / totalTime) * 100}%"></span></span>
                    <span class="bar-value">{stats.nightCount}</span>
                  </div>
                </div>
              </div>
            </div>
          {:else if activeTab === 'characters'}
            {#if stats.characters.length > 0}
              <div class="data-table-wrap">
                <table class="data-table">
                  <thead>
                    <tr>
                      <th class="col-name" aria-sort={ariaSortFor(charSortKey === 'name', charSortDir)}>
                        <button class="sort-btn" onclick={() => cycleCharSort('name')}>
                          Character
                          <span class="sort-ind" class:active={charSortKey === 'name'} data-dir={charSortKey === 'name' ? charSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-num" aria-sort={ariaSortFor(charSortKey === 'scenes', charSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleCharSort('scenes')}>
                          Scenes
                          <span class="sort-ind" class:active={charSortKey === 'scenes'} data-dir={charSortKey === 'scenes' ? charSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-num" aria-sort={ariaSortFor(charSortKey === 'dialogueBlocks', charSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleCharSort('dialogueBlocks')}>
                          Dialogue
                          <span class="sort-ind" class:active={charSortKey === 'dialogueBlocks'} data-dir={charSortKey === 'dialogueBlocks' ? charSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-bar" aria-sort={ariaSortFor(charSortKey === 'percentage', charSortDir)}>
                        <button class="sort-btn" onclick={() => cycleCharSort('percentage')}>
                          Share
                          <span class="sort-ind" class:active={charSortKey === 'percentage'} data-dir={charSortKey === 'percentage' ? charSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each sortedCharacters as char, i}
                      <tr>
                        <td class="col-name"><span class="rank">{i + 1}</span>{char.name}</td>
                        <td class="col-num">{char.scenes}</td>
                        <td class="col-num">{char.dialogueBlocks}</td>
                        <td class="col-bar">
                          <span class="inline-bar"><span class="inline-bar-fill" style="width: {char.percentage}%"></span></span>
                          <span class="inline-bar-value">{char.percentage}%</span>
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {:else}
              <div class="empty-pane">
                <span class="empty-glyph">∅</span>
                <p>No characters yet — add a Character element to populate this report.</p>
              </div>
            {/if}
          {:else if activeTab === 'locations'}
            {#if stats.locations.length > 0}
              <div class="data-table-wrap">
                <table class="data-table">
                  <thead>
                    <tr>
                      <th class="col-name" aria-sort={ariaSortFor(locSortKey === 'name', locSortDir)}>
                        <button class="sort-btn" onclick={() => cycleLocSort('name')}>
                          Location
                          <span class="sort-ind" class:active={locSortKey === 'name'} data-dir={locSortKey === 'name' ? locSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-num" aria-sort={ariaSortFor(locSortKey === 'scenes', locSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleLocSort('scenes')}>
                          Scenes
                          <span class="sort-ind" class:active={locSortKey === 'scenes'} data-dir={locSortKey === 'scenes' ? locSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-meta" aria-sort={ariaSortFor(locSortKey === 'setting', locSortDir)}>
                        <button class="sort-btn" onclick={() => cycleLocSort('setting')}>
                          Setting
                          <span class="sort-ind" class:active={locSortKey === 'setting'} data-dir={locSortKey === 'setting' ? locSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each sortedLocations as loc, i}
                      <tr>
                        <td class="col-name"><span class="rank">{i + 1}</span>{loc.name}</td>
                        <td class="col-num">{loc.scenes}</td>
                        <td class="col-meta"><span class="setting-pill setting-{loc.setting.toLowerCase()}">{loc.setting}</span></td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {:else}
              <div class="empty-pane">
                <span class="empty-glyph">∅</span>
                <p>No locations parsed — write scene headings like <em>INT. KITCHEN — DAY</em> to populate this report.</p>
              </div>
            {/if}
          {:else if activeTab === 'schedule'}
            {#if stats.schedule.length > 0}
              <div class="data-table-wrap">
                <table class="data-table schedule-table">
                  <thead>
                    <tr>
                      <th class="col-snum" aria-sort={ariaSortFor(schedSortKey === 'sceneNumber', schedSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleSchedSort('sceneNumber')}>
                          #
                          <span class="sort-ind" class:active={schedSortKey === 'sceneNumber'} data-dir={schedSortKey === 'sceneNumber' ? schedSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-meta" aria-sort={ariaSortFor(schedSortKey === 'setting', schedSortDir)}>
                        <button class="sort-btn" onclick={() => cycleSchedSort('setting')}>
                          Setting
                          <span class="sort-ind" class:active={schedSortKey === 'setting'} data-dir={schedSortKey === 'setting' ? schedSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-name" aria-sort={ariaSortFor(schedSortKey === 'location', schedSortDir)}>
                        <button class="sort-btn" onclick={() => cycleSchedSort('location')}>
                          Location
                          <span class="sort-ind" class:active={schedSortKey === 'location'} data-dir={schedSortKey === 'location' ? schedSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-meta" aria-sort={ariaSortFor(schedSortKey === 'time', schedSortDir)}>
                        <button class="sort-btn" onclick={() => cycleSchedSort('time')}>
                          Time
                          <span class="sort-ind" class:active={schedSortKey === 'time'} data-dir={schedSortKey === 'time' ? schedSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-num" aria-sort={ariaSortFor(schedSortKey === 'characterCount', schedSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleSchedSort('characterCount')}>
                          Cast
                          <span class="sort-ind" class:active={schedSortKey === 'characterCount'} data-dir={schedSortKey === 'characterCount' ? schedSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-meta" aria-sort={ariaSortFor(schedSortKey === 'locationGroup', schedSortDir)}>
                        <button class="sort-btn" onclick={() => cycleSchedSort('locationGroup')}>
                          Group
                          <span class="sort-ind" class:active={schedSortKey === 'locationGroup'} data-dir={schedSortKey === 'locationGroup' ? schedSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-meta" aria-sort={ariaSortFor(schedSortKey === 'scheduledDate', schedSortDir)}>
                        <button class="sort-btn" onclick={() => cycleSchedSort('scheduledDate')}>
                          Day
                          <span class="sort-ind" class:active={schedSortKey === 'scheduledDate'} data-dir={schedSortKey === 'scheduledDate' ? schedSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                    </tr>
                  </thead>
                  {#if schedSortKey === null}
                    <!-- Default view: grouped by shoot date with per-day
                         header + total. Mirrors the printed Daily Shoot
                         List PDF so the on-screen and printed schedules
                         read the same way. (#136) -->
                    {#each groupedSchedule as group (group.date || 'unscheduled')}
                      <tbody class="schedule-group">
                        <tr class="schedule-group-header" class:unscheduled={group.date === ''}>
                          <td colspan="7">
                            <span class="sg-label">{group.label}</span>
                            <span class="sg-meta">
                              <span class="sg-count">{group.rows.length} {group.rows.length === 1 ? 'scene' : 'scenes'}</span>
                              {#if group.totalEighths > 0 && group.date !== ''}
                                <span class="sg-sep">·</span>
                                <span class="sg-total">{formatDayTotal(group.totalEighths)}</span>
                              {/if}
                            </span>
                          </td>
                        </tr>
                        {#each group.rows as row}
                          <tr>
                            <td class="col-snum">{row.sceneNumber}</td>
                            <td class="col-meta">{row.setting || '—'}</td>
                            <td class="col-name">{row.location || '(empty)'}</td>
                            <td class="col-meta">{row.time || '—'}</td>
                            <td class="col-num">{row.characterCount}</td>
                            <td class="col-meta">{row.locationGroup || '—'}</td>
                            <td class="col-meta">{row.scheduledDate || '—'}</td>
                          </tr>
                        {/each}
                      </tbody>
                    {/each}
                  {:else}
                    <tbody>
                      {#each sortedSchedule as row}
                        <tr>
                          <td class="col-snum">{row.sceneNumber}</td>
                          <td class="col-meta">{row.setting || '—'}</td>
                          <td class="col-name">{row.location || '(empty)'}</td>
                          <td class="col-meta">{row.time || '—'}</td>
                          <td class="col-num">{row.characterCount}</td>
                          <td class="col-meta">{row.locationGroup || '—'}</td>
                          <td class="col-meta">{row.scheduledDate || '—'}</td>
                        </tr>
                      {/each}
                    </tbody>
                  {/if}
                </table>
              </div>
            {:else}
              <div class="empty-pane">
                <span class="empty-glyph">∅</span>
                <p>No scenes yet.</p>
              </div>
            {/if}
          {:else if activeTab === 'episodes'}
            <!-- Episodes — per-episode summary table (series only).
                 Same column-sort vocabulary as the other tables; default
                 order is the production order (episode #), which is the
                 reading order for a season planner. -->
            {#if episodeStats.length > 0}
              <div class="data-table-wrap">
                <table class="data-table episodes-table">
                  <thead>
                    <tr>
                      <th class="col-snum" aria-sort={ariaSortFor(epSortKey === 'number', epSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleEpSort('number')}>
                          #
                          <span class="sort-ind" class:active={epSortKey === 'number'} data-dir={epSortKey === 'number' ? epSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-name" aria-sort={ariaSortFor(epSortKey === 'title', epSortDir)}>
                        <button class="sort-btn" onclick={() => cycleEpSort('title')}>
                          Title
                          <span class="sort-ind" class:active={epSortKey === 'title'} data-dir={epSortKey === 'title' ? epSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-meta" aria-sort={ariaSortFor(epSortKey === 'status', epSortDir)}>
                        <button class="sort-btn" onclick={() => cycleEpSort('status')}>
                          Status
                          <span class="sort-ind" class:active={epSortKey === 'status'} data-dir={epSortKey === 'status' ? epSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-num" aria-sort={ariaSortFor(epSortKey === 'pageCount', epSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleEpSort('pageCount')}>
                          Pages
                          <span class="sort-ind" class:active={epSortKey === 'pageCount'} data-dir={epSortKey === 'pageCount' ? epSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-num" aria-sort={ariaSortFor(epSortKey === 'sceneCount', epSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleEpSort('sceneCount')}>
                          Scenes
                          <span class="sort-ind" class:active={epSortKey === 'sceneCount'} data-dir={epSortKey === 'sceneCount' ? epSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-num" aria-sort={ariaSortFor(epSortKey === 'wordCount', epSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleEpSort('wordCount')}>
                          Words
                          <span class="sort-ind" class:active={epSortKey === 'wordCount'} data-dir={epSortKey === 'wordCount' ? epSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-num" aria-sort={ariaSortFor(epSortKey === 'dialogueLineCount', epSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleEpSort('dialogueLineCount')}>
                          Dialogue
                          <span class="sort-ind" class:active={epSortKey === 'dialogueLineCount'} data-dir={epSortKey === 'dialogueLineCount' ? epSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-num" aria-sort={ariaSortFor(epSortKey === 'screenTimeMinutes', epSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleEpSort('screenTimeMinutes')}>
                          ~min
                          <span class="sort-ind" class:active={epSortKey === 'screenTimeMinutes'} data-dir={epSortKey === 'screenTimeMinutes' ? epSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                      <th class="col-num" aria-sort={ariaSortFor(epSortKey === 'characterCount', epSortDir)}>
                        <button class="sort-btn align-end" onclick={() => cycleEpSort('characterCount')}>
                          Cast
                          <span class="sort-ind" class:active={epSortKey === 'characterCount'} data-dir={epSortKey === 'characterCount' ? epSortDir : ''} aria-hidden="true"></span>
                        </button>
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each sortedEpisodes as ep (ep.number)}
                      <tr>
                        <td class="col-snum"><span class="ep-pip">{String(ep.number).padStart(2, '0')}</span></td>
                        <td class="col-name">
                          {#if ep.title}{ep.title}{:else}<span class="ep-untitled">Untitled</span>{/if}
                        </td>
                        <td class="col-meta"><span class="status-pill status-{ep.status}">{ep.status}</span></td>
                        <td class="col-num">{ep.pageCount}</td>
                        <td class="col-num">{ep.sceneCount}</td>
                        <td class="col-num">{ep.wordCount.toLocaleString()}</td>
                        <td class="col-num">{ep.dialogueLineCount}</td>
                        <td class="col-num">~{ep.screenTimeMinutes}</td>
                        <td class="col-num">{ep.characterCount}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {:else}
              <div class="empty-pane">
                <span class="empty-glyph">∅</span>
                <p>No episodes yet.</p>
              </div>
            {/if}
          {/if}
        </div>
      </section>
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

  /* Stable wide geometry — same size regardless of which tab is active.
     Modeled on HelpModal so the two reference modals feel of-a-piece. */
  .stats-card {
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: var(--modal-radius);
    width: var(--modal-w-lg);
    max-width: 92vw;
    height: 78vh;
    max-height: 760px;
    box-shadow: var(--modal-shadow);
    animation: modal-in var(--modal-anim-duration) ease-out;
    font-family: var(--ui-font);
    display: grid;
    grid-template-columns: 220px 1fr;
    overflow: hidden;
  }

  @keyframes modal-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }

  /* ─── Left rail ─── */
  /* `min-height: 0` + `overflow-y: auto` so a series with many episodes
     doesn't push the rail past the modal edge. Same grid-child gotcha
     as .stats-pane below. */
  .stats-rail {
    background: var(--surface-base);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    padding: 22px 14px 14px;
    min-height: 0;
    overflow-y: auto;
  }

  .rail-header {
    padding: 0 6px 18px;
  }

  /* Rail header — sits above the tab list as a department marker. */
  .rail-header h2 {
    margin: 0;
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--marker-color);
  }

  .rail-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .rail-item {
    display: grid;
    grid-template-columns: 18px 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 9px 10px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-secondary);
    font-family: var(--ui-font);
    font-size: 12.5px;
    cursor: pointer;
    text-align: left;
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .rail-item:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .rail-item.active {
    background: var(--accent-muted);
    color: var(--accent);
    /* Soft accent left bar — nearly the only color in the rail, draws
       the eye to the active section without shouting. */
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .rail-icon {
    color: currentColor;
    flex-shrink: 0;
  }

  .rail-label {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rail-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 18px;
    padding: 0 6px;
    border-radius: 9px;
    background: var(--surface-elevated);
    color: var(--text-muted);
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
  }

  .rail-item.active .rail-count {
    background: var(--accent);
    color: var(--text-on-accent);
  }

  /* ─── Scope segmented (series only) ───
     Sits between the rail header and the tab list. Mirrors the Export
     modal's scope-segmented vocabulary so writers learn the pattern
     once and recognise it everywhere. */
  .scope-segmented {
    display: flex;
    margin: 0 0 14px;
    padding: 2px;
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
  }

  .scope-seg {
    flex: 1;
    padding: 5px 8px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .scope-seg:hover:not(.active) {
    color: var(--text-primary);
  }

  .scope-seg.active {
    background: var(--surface-float);
    color: var(--marker-color);
    box-shadow: 0 1px 2px var(--shadow-soft);
  }

  .rail-footer {
    padding-top: 12px;
    border-top: 1px solid var(--border-subtle);
    margin-top: 12px;
  }

  .rail-action {
    width: 100%;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    font-family: var(--ui-font);
    font-size: 11.5px;
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .rail-action:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  /* ─── Right pane ─── */
  /* `min-height: 0` is required because grid items default to `min-height: auto`,
     which lets the pane grow taller than its grid track when the table inside
     is long. Without it, .pane-content's flex+overflow never has a bound to
     scroll within and the table just gets clipped at the modal edge. */
  .stats-pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--surface-float);
  }

  .pane-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 22px 28px 18px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .pane-title-block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .pane-title {
    margin: 0;
    font-size: 22px;
    line-height: 1;
  }

  .pane-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  /* Export CSV — secondary affordance, sits next to close. Subtle by
     default (matches the rail-action footer style) so the eye still
     lands on the title block first. (#137) */
  .btn-csv {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 11px;
    border: 1px solid var(--border-medium);
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--ui-font);
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }

  .btn-csv:hover {
    background: var(--accent-muted);
    border-color: var(--accent);
    color: var(--accent);
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

  .pane-content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 24px 28px 28px;
  }

  /* ─── Overview hero stats ─── */
  .hero-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
    margin-bottom: 32px;
  }

  .hero-stat {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 18px 18px 16px;
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    position: relative;
    overflow: hidden;
  }

  /* Quiet diagonal stripe in the hero stat — adds a touch of texture
     without competing with the numbers. Uses page-grain tokens so the
     theme drives the visual weight. */
  .hero-stat::after {
    content: '';
    position: absolute;
    top: 0;
    right: 0;
    width: 60px;
    height: 60px;
    background: linear-gradient(135deg, transparent 50%, var(--accent-muted) 50%);
    opacity: 0.5;
    pointer-events: none;
  }

  /* Big stat numerals stay in Courier Prime so the figures align as
     tabular-nums across rows. Promotion: lift the size and tighten so
     the digits read as the loud center of each tile. */
  .hero-value {
    font-family: var(--editor-font-en), var(--ui-font);
    font-size: 34px;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1;
    letter-spacing: -0.01em;
    font-variant-numeric: tabular-nums;
  }

  .hero-unit {
    margin-left: 6px;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-muted);
    letter-spacing: 0;
  }

  /* Hero label — Courier marker eyebrow. Reads as a department label
     above each big stat. */
  .hero-label {
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--marker-color);
  }

  /* ─── Block heading (centered with rule) ─── */
  .block-heading {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 14px;
    margin-bottom: 18px;
  }

  .block-rule {
    height: 1px;
    background: var(--border-subtle);
  }

  .block-title {
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  /* ─── Breakdown bars ─── */
  .breakdown-bars {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 28px;
  }

  .bar-pair {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .bar-row {
    display: grid;
    grid-template-columns: 80px 1fr 36px;
    align-items: center;
    gap: 12px;
  }

  .bar-label {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .bar-track {
    position: relative;
    height: 8px;
    background: var(--surface-base);
    border-radius: 4px;
    overflow: hidden;
  }

  .bar-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: 4px;
    transition: width 200ms ease;
  }

  .bar-fill.alt    { background: var(--accent-hover, var(--accent)); opacity: 0.75; }
  .bar-fill.warm   { background: var(--accent-warm, #d6a14a); }
  .bar-fill.cool   { background: var(--text-secondary); opacity: 0.75; }

  .bar-value {
    font-size: 12px;
    text-align: right;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }

  /* ─── Data tables ─── */
  .data-table-wrap {
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    overflow: hidden;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12.5px;
  }

  .data-table thead {
    background: var(--surface-base);
  }

  .data-table th {
    font-size: 10px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: 0;
    border-bottom: 1px solid var(--border-subtle);
    text-align: left;
    position: sticky;
    top: 0;
    background: var(--surface-base);
    z-index: 1;
  }

  /* Sort button — fills the th so the entire header cell is the click
     target. Inherits th typography. The arrow indicator on the right
     fades in only on the active sort column; inactive columns get a
     subtle hover hint so the writer learns they're clickable. (#135) */
  .sort-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 10px 12px;
    background: transparent;
    border: none;
    color: inherit;
    font: inherit;
    text-transform: inherit;
    letter-spacing: inherit;
    text-align: inherit;
    cursor: pointer;
    transition: color var(--motion-fast, 100ms) ease,
                background var(--motion-fast, 100ms) ease;
  }

  .sort-btn:hover {
    color: var(--text-primary);
    background: var(--surface-hover);
  }

  .sort-btn.align-end {
    justify-content: flex-end;
    text-align: right;
  }

  .sort-ind {
    display: inline-block;
    width: 8px;
    height: 8px;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity var(--motion-fast, 100ms) ease;
    background-image: linear-gradient(45deg, transparent 45%, currentColor 45%, currentColor 55%, transparent 55%);
    /* Idle hint (light triangle) — show on hover for inactive columns. */
  }

  .sort-btn:hover .sort-ind:not(.active) {
    opacity: 0.35;
  }

  .sort-ind.active {
    opacity: 1;
    color: var(--accent);
  }

  /* Up/down arrow rendered via two diagonal strokes. asc points up,
     desc points down. */
  .sort-ind.active[data-dir='asc'] {
    background-image:
      linear-gradient(45deg, transparent 38%, currentColor 38%, currentColor 50%, transparent 50%),
      linear-gradient(-45deg, transparent 38%, currentColor 38%, currentColor 50%, transparent 50%);
    background-position: left top, right top;
    background-size: 50% 100%, 50% 100%;
    background-repeat: no-repeat;
  }

  .sort-ind.active[data-dir='desc'] {
    background-image:
      linear-gradient(-45deg, transparent 38%, currentColor 38%, currentColor 50%, transparent 50%),
      linear-gradient(45deg, transparent 38%, currentColor 38%, currentColor 50%, transparent 50%);
    background-position: left bottom, right bottom;
    background-size: 50% 100%, 50% 100%;
    background-repeat: no-repeat;
  }

  th[aria-sort='ascending'],
  th[aria-sort='descending'] {
    color: var(--accent);
  }

  /* Schedule group header — one per shoot date in the default view.
     Reads as a typographic eyebrow, not a data row: bigger top padding,
     accent left rule, mixed-weight label + meta. (#136) */
  .schedule-group-header td {
    padding: 14px 12px 8px;
    background: var(--surface-base);
    border-top: 1px solid var(--border-medium);
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-primary);
    font-size: 11.5px;
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .schedule-group:first-child .schedule-group-header td {
    border-top: none;
  }

  .schedule-group-header.unscheduled td {
    box-shadow: inset 3px 0 0 var(--text-muted);
    color: var(--text-secondary);
  }

  .sg-label {
    font-weight: 600;
    letter-spacing: 0.02em;
    margin-right: 14px;
  }

  .sg-meta {
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
    font-size: 10.5px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .sg-sep {
    color: var(--border-medium);
  }

  /* Don't paint hover on the group header rows. */
  .schedule-group-header:hover td {
    background: var(--surface-base);
  }

  .data-table td {
    padding: 9px 12px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-subtle);
    vertical-align: middle;
  }

  .data-table tbody tr:last-child td {
    border-bottom: none;
  }

  .data-table tbody tr:hover td {
    background: var(--surface-hover);
  }

  .col-num {
    text-align: right;
    font-variant-numeric: tabular-nums;
    width: 80px;
  }

  .col-snum {
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
    font-size: 11.5px;
    width: 56px;
  }

  .col-name {
    font-weight: 500;
    color: var(--text-primary);
  }

  .col-meta {
    color: var(--text-secondary);
    font-size: 11.5px;
  }

  .col-bar {
    width: 36%;
  }

  /* Subtle leading rank number on character / location rows so the eye
     can scan ordering without losing the alphabet. */
  .rank {
    display: inline-block;
    width: 28px;
    margin-right: 8px;
    color: var(--text-muted);
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.04em;
  }

  /* Inline percentage bar — visual sibling of the breakdown bars on the
     overview tab so the two views read as the same system. */
  .inline-bar {
    display: inline-block;
    width: calc(100% - 44px);
    height: 6px;
    background: var(--surface-base);
    border-radius: 3px;
    overflow: hidden;
    vertical-align: middle;
    margin-right: 8px;
  }

  .inline-bar-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
  }

  .inline-bar-value {
    display: inline-block;
    width: 36px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--text-primary);
    font-size: 11.5px;
    vertical-align: middle;
  }

  /* Pill marker for INT./EXT./Both on the locations table — replaces the
     plain text "Interior"/"Exterior" labels with something scannable. */
  .setting-pill {
    display: inline-flex;
    align-items: center;
    height: 18px;
    padding: 0 8px;
    border-radius: 9px;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    background: var(--surface-base);
    color: var(--text-secondary);
  }

  .setting-pill.setting-interior {
    background: var(--accent-muted);
    color: var(--accent);
  }

  .setting-pill.setting-exterior {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .setting-pill.setting-both {
    background: var(--accent);
    color: var(--text-on-accent);
  }

  /* ─── Episodes table pieces ─── */
  /* Zero-padded Courier pip — same identifier system the navigator
     and the cards view use, so the same episode reads as the same
     thing in every surface. */
  .ep-pip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 26px;
    padding: 1px 6px;
    background: var(--surface-base);
    color: var(--marker-color);
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    font-variant-numeric: tabular-nums;
    border-radius: 4px;
    border: 1px solid var(--border-subtle);
  }

  .ep-untitled {
    color: var(--text-muted);
    font-style: italic;
  }

  /* Status pill — one for each EpisodeStatus. Same hue family as the
     SeriesEpisodeList status pills so the writer sees the same colors
     in the navigator and in the stats table. */
  .status-pill {
    display: inline-block;
    padding: 1px 8px;
    border-radius: 10px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    background: var(--surface-base);
    color: var(--text-secondary);
    border: 1px solid var(--border-subtle);
  }

  .status-pill.status-outline {
    color: var(--text-muted);
  }

  .status-pill.status-draft {
    color: var(--marker-color);
    border-color: var(--accent-warm-muted);
  }

  .status-pill.status-revision {
    color: var(--accent);
    border-color: var(--accent-muted);
  }

  .status-pill.status-final {
    background: var(--accent);
    color: var(--text-on-accent);
    border-color: var(--accent);
  }

  /* Empty state — composed, calm, never apologetic. */
  .empty-pane {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 280px;
    color: var(--text-muted);
    text-align: center;
    gap: 14px;
  }

  .empty-glyph {
    font-size: 40px;
    line-height: 1;
    color: var(--border-medium);
  }

  .empty-pane p {
    margin: 0;
    max-width: 360px;
    font-size: 12.5px;
    line-height: 1.5;
  }

  .empty-pane em {
    font-family: var(--editor-font-en);
    font-style: normal;
    color: var(--text-secondary);
  }
</style>
