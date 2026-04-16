<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { focusTrap } from '$lib/actions/focusTrap';

  let { open = $bindable(false) } = $props<{ open: boolean }>();

  interface CharacterStat {
    name: string;
    scenes: number;
    dialogueBlocks: number;
    percentage: number;
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
  }

  let stats = $state<Stats>(computeStats());

  // Recompute stats whenever the modal opens
  $effect(() => {
    if (open) {
      stats = computeStats();
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

  function computeStats(): Stats {
    const empty: Stats = {
      pageCount: 0, sceneCount: 0, wordCount: 0, dialogueLineCount: 0,
      intCount: 0, extCount: 0, dayCount: 0, nightCount: 0,
      screenTimeMinutes: 0, characters: [],
    };

    const doc = documentStore.document;
    if (!doc || !doc.content) return empty;

    const content = doc.content as {
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
        }
      }

      if (type === 'dialogue') {
        dialogueLineCount++;
      }
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

      {#if stats.characters.length > 0}
        <div class="section-label">Characters by dialogue</div>
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
      {/if}
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
    max-height: 80vh;
    overflow-y: auto;
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
</style>
