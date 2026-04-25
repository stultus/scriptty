<script lang="ts">
  import { untrack } from 'svelte';
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
   *  `locationGroup` and `scheduledDate` come from the SceneCard (#124). */
  interface ScheduleEntry {
    sceneNumber: number;
    setting: string;
    location: string;
    time: string;
    characterCount: number;
    locationGroup: string;
    scheduledDate: string;
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

  type Tab = 'overview' | 'characters' | 'locations' | 'schedule';
  let activeTab = $state<Tab>('overview');

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

  let charSortKey = $state<CharSortKey | null>(null);
  let charSortDir = $state<SortDir>('asc');
  let locSortKey = $state<LocSortKey | null>(null);
  let locSortDir = $state<SortDir>('asc');
  let schedSortKey = $state<SchedSortKey | null>(null);
  let schedSortDir = $state<SortDir>('asc');

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
  let activeTabLabel = $derived.by<{ eyebrow: string; title: string }>(() => {
    switch (activeTab) {
      case 'overview':   return { eyebrow: 'At a glance', title: 'Overview' };
      case 'characters': return { eyebrow: 'Cast report', title: 'Characters by dialogue' };
      case 'locations':  return { eyebrow: 'Production prep', title: 'Locations' };
      case 'schedule':   return { eyebrow: 'Production prep', title: 'Shoot schedule' };
    }
  });

  let stats = $state<Stats>(untrack(() => computeStats()));

  // Recompute stats only when the modal opens — not on every keystroke while
  // it's open. The $effect previously read `documentStore.activeContent`
  // transitively through computeStats(), which tracked it as a dependency
  // and re-ran the full ~2000-node walk on every typed character. Wrapping
  // the call in untrack() ensures the only reactive dep is `open` (#100).
  // The Refresh button gives the writer an explicit way to re-snapshot.
  $effect(() => {
    if (open) {
      untrack(() => { stats = computeStats(); });
      // Reset sorts on open so the writer always sees the default
      // ordering first — sort choices are session-only (#135).
      charSortKey = null; charSortDir = 'asc';
      locSortKey = null; locSortDir = 'asc';
      schedSortKey = null; schedSortDir = 'asc';
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

  /** Maps a sortKey + dir to the aria-sort value the table cell needs. */
  function ariaSortFor(active: boolean, dir: SortDir): 'none' | 'ascending' | 'descending' {
    if (!active) return 'none';
    return dir === 'asc' ? 'ascending' : 'descending';
  }

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

  function computeStats(): Stats {
    const empty: Stats = {
      pageCount: 0, sceneCount: 0, wordCount: 0, dialogueLineCount: 0,
      intCount: 0, extCount: 0, dayCount: 0, nightCount: 0,
      screenTimeMinutes: 0, characters: [], locations: [], schedule: [],
    };

    // Stats reflect the active episode in series projects, not the top-level
    // film placeholder. Using activeContent keeps the numbers in sync with
    // what the writer is actually editing.
    const activeContent = documentStore.activeContent;
    if (!activeContent) return empty;

    const content = activeContent as {
      type?: string;
      content?: Array<{
        type?: string;
        content?: Array<{ text?: string }>;
      }>;
    };
    if (!content.content) return empty;

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
    const schedule: ScheduleEntry[] = [];

    // Pull SceneCard scheduling fields up-front so the per-scene loop
    // can attach them by 0-based index. activeSceneCards is small (≤ scene
    // count); a Map is cheaper to look up per scene than a linear find. (#124)
    const cardsByIndex = new Map<number, { scheduled_date: string; location_group: string }>();
    for (const c of documentStore.activeSceneCards) {
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
        });
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

    // Backfill characterCount per schedule row now that the walk is done.
    for (const row of schedule) {
      row.characterCount = sceneCharacters.get(row.sceneNumber)?.size ?? 0;
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
            <span class="pane-eyebrow">{activeTabLabel.eyebrow}</span>
            <h3 class="pane-title">{activeTabLabel.title}</h3>
          </div>
          <button class="btn-close" onclick={() => { open = false; }} aria-label="Close statistics">&times;</button>
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
          {:else}
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
                </table>
              </div>
            {:else}
              <div class="empty-pane">
                <span class="empty-glyph">∅</span>
                <p>No scenes yet.</p>
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
  .stats-rail {
    background: var(--surface-base);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    padding: 22px 14px 14px;
  }

  .rail-header {
    padding: 0 6px 18px;
  }

  .rail-header h2 {
    margin: 0;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--text-muted);
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
  .stats-pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
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
    gap: 4px;
    min-width: 0;
  }

  .pane-eyebrow {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .pane-title {
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

  .hero-value {
    font-family: var(--editor-font-en), var(--ui-font);
    font-size: 30px;
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

  .hero-label {
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-muted);
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
