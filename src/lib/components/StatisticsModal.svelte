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
    }
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
    <div class="modal-card" use:focusTrap>
      <div class="modal-header">
        <h2>Script Statistics</h2>
        <div class="header-actions">
          <button class="btn-ghost" onclick={refresh}>Refresh</button>
          <button class="btn-close" onclick={() => { open = false; }}>&times;</button>
        </div>
      </div>

      <!-- Tab strip: Overview is the default view; the other three tabs are
           production-prep reports added in #119. They all read the same
           snapshot computed at modal open, so switching is instant. -->
      <div class="tabs" role="tablist" aria-label="Statistics views">
        <button
          class="tab"
          class:active={activeTab === 'overview'}
          role="tab"
          aria-selected={activeTab === 'overview'}
          onclick={() => { activeTab = 'overview'; }}
        >Overview</button>
        <button
          class="tab"
          class:active={activeTab === 'characters'}
          role="tab"
          aria-selected={activeTab === 'characters'}
          onclick={() => { activeTab = 'characters'; }}
        >Characters <span class="tab-count">{stats.characters.length}</span></button>
        <button
          class="tab"
          class:active={activeTab === 'locations'}
          role="tab"
          aria-selected={activeTab === 'locations'}
          onclick={() => { activeTab = 'locations'; }}
        >Locations <span class="tab-count">{stats.locations.length}</span></button>
        <button
          class="tab"
          class:active={activeTab === 'schedule'}
          role="tab"
          aria-selected={activeTab === 'schedule'}
          onclick={() => { activeTab = 'schedule'; }}
        >Schedule <span class="tab-count">{stats.schedule.length}</span></button>
      </div>

      {#if activeTab === 'overview'}
        <div class="stats-grid">
          <div class="stat-item">
            <span class="stat-value">{stats.pageCount}</span>
            <span class="stat-label">Pages</span>
          </div>
          <div class="stat-item">
            <span class="stat-value">{stats.sceneCount}</span>
            <span class="stat-label">Scenes</span>
          </div>
          <div class="stat-item">
            <span class="stat-value">{stats.wordCount.toLocaleString()}</span>
            <span class="stat-label">Words</span>
          </div>
          <div class="stat-item">
            <span class="stat-value">{stats.dialogueLineCount}</span>
            <span class="stat-label">Dialogue blocks</span>
          </div>
          <div class="stat-item">
            <span class="stat-value">~{stats.screenTimeMinutes} min</span>
            <span class="stat-label">Est. screen time</span>
          </div>
        </div>

        <div class="scene-breakdown">
          <div class="section-label">Scene breakdown</div>
          <div class="breakdown-row">
            <span class="breakdown-pair"><strong>{stats.intCount}</strong> Interior</span>
            <span class="breakdown-pair"><strong>{stats.extCount}</strong> Exterior</span>
            <span class="breakdown-pair"><strong>{stats.dayCount}</strong> Day</span>
            <span class="breakdown-pair"><strong>{stats.nightCount}</strong> Night</span>
          </div>
        </div>
      {:else if activeTab === 'characters'}
        {#if stats.characters.length > 0}
          <div class="char-table-wrap">
            <table class="char-table">
              <thead>
                <tr>
                  <th class="col-name">Character</th>
                  <th class="col-num">Scenes</th>
                  <th class="col-num">Dialogue</th>
                  <th class="col-num">%</th>
                </tr>
              </thead>
              <tbody>
                {#each stats.characters as char}
                  <tr>
                    <td class="col-name">{char.name}</td>
                    <td class="col-num">{char.scenes}</td>
                    <td class="col-num">{char.dialogueBlocks}</td>
                    <td class="col-num">{char.percentage}%</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <div class="empty-tab">No characters yet — add a Character element to populate this report.</div>
        {/if}
      {:else if activeTab === 'locations'}
        {#if stats.locations.length > 0}
          <div class="char-table-wrap">
            <table class="char-table">
              <thead>
                <tr>
                  <th class="col-name">Location</th>
                  <th class="col-num">Scenes</th>
                  <th class="col-meta">Setting</th>
                </tr>
              </thead>
              <tbody>
                {#each stats.locations as loc}
                  <tr>
                    <td class="col-name">{loc.name}</td>
                    <td class="col-num">{loc.scenes}</td>
                    <td class="col-meta">{loc.setting}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <div class="empty-tab">No locations parsed — write scene headings like <em>INT. KITCHEN — DAY</em> to populate this report.</div>
        {/if}
      {:else}
        <!-- Schedule tab: scenes in document order with setting / time /
             character count. The producer can scan top-to-bottom for the
             shoot sequence the writer intended. -->
        {#if stats.schedule.length > 0}
          <div class="char-table-wrap">
            <table class="char-table schedule-table">
              <thead>
                <tr>
                  <th class="col-num">#</th>
                  <th class="col-meta">Setting</th>
                  <th class="col-name">Location</th>
                  <th class="col-meta">Time</th>
                  <th class="col-num">Cast</th>
                  <th class="col-meta">Group</th>
                  <th class="col-meta">Day</th>
                </tr>
              </thead>
              <tbody>
                {#each stats.schedule as row}
                  <tr>
                    <td class="col-num">{row.sceneNumber}</td>
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
          <div class="empty-tab">No scenes yet.</div>
        {/if}
      {/if}
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

  .modal-card {
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: var(--modal-radius);
    padding: var(--modal-padding);
    width: var(--modal-w-base);
    max-width: 90vw;
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: var(--modal-shadow);
    animation: modal-in var(--modal-anim-duration) ease-out;
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
    font-size: var(--modal-header-size);
    color: var(--text-primary);
    font-weight: var(--modal-header-weight);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .btn-ghost {
    height: 28px;
    padding: 0 10px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 11px;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-ghost:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
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

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
    margin-bottom: 20px;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 12px 8px;
    background: var(--surface-base);
    border-radius: 8px;
    border: 1px solid var(--border-subtle);
  }

  .stat-value {
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1.2;
  }

  .stat-label {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-top: 4px;
  }

  .scene-breakdown {
    margin-bottom: 20px;
  }

  .section-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 8px;
  }

  .breakdown-row {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }

  .breakdown-pair {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .breakdown-pair strong {
    color: var(--text-primary);
    font-weight: 600;
  }

  .char-table-wrap {
    max-height: 240px;
    overflow-y: auto;
  }

  .char-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  .char-table thead {
    position: sticky;
    top: 0;
    background: var(--surface-float);
  }

  .char-table th {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border-subtle);
    text-align: left;
  }

  .char-table td {
    padding: 5px 8px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-subtle);
  }

  .char-table tr:hover td {
    background: var(--surface-hover);
  }

  .col-num {
    text-align: right;
  }

  .col-num:is(th) {
    text-align: right;
  }

  .col-name {
    font-weight: 500;
    color: var(--text-primary);
  }

  .col-meta {
    color: var(--text-secondary);
    font-size: 11.5px;
    text-align: left;
  }

  /* ─── Tab strip (#119) ─── */
  .tabs {
    display: flex;
    gap: 2px;
    margin: 0 0 16px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .tab {
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-muted);
    font-family: var(--ui-font);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    margin-bottom: -1px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: color var(--motion-fast, 100ms) ease, border-color var(--motion-fast, 100ms) ease;
  }

  .tab:hover {
    color: var(--text-secondary);
  }

  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .tab-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 16px;
    padding: 0 5px;
    border-radius: 8px;
    background: var(--surface-base);
    color: var(--text-muted);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  .tab.active .tab-count {
    background: var(--accent-muted);
    color: var(--accent);
  }

  .empty-tab {
    padding: 28px 12px;
    text-align: center;
    color: var(--text-muted);
    font-size: 12.5px;
    line-height: 1.5;
  }

  .empty-tab em {
    font-family: var(--editor-font-en);
    font-style: normal;
    color: var(--text-secondary);
  }

  /* Schedule's first column is the scene number, narrower than other tables. */
  .schedule-table .col-num:first-child {
    width: 48px;
  }
</style>
