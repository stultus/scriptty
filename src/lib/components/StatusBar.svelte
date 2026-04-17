<script lang="ts">
  import { InputModeManager } from '$lib/editor/input/InputModeManager';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import SettingsModal from './SettingsModal.svelte';

  let {
    rightContent,
    showAnnotations = $bindable(true),
    onShowHelp,
  }: {
    rightContent?: import('svelte').Snippet;
    showAnnotations?: boolean;
    onShowHelp?: () => void;
  } = $props();

  const inputManager = InputModeManager.getInstance();
  let isMalayalam = $state(inputManager.isMalayalam);
  let showSettings = $state(false);

  // Subscribe to input manager changes so the indicator stays in sync
  // when toggled via Ctrl+Space from any view. No polling.
  $effect(() => {
    return inputManager.subscribe(() => {
      isMalayalam = inputManager.isMalayalam;
    });
  });

  // A ticking clock that re-renders the "Saved N min ago" label. Using a
  // reactive tick keeps the relative time honest without the label ever
  // going stale while the app sits idle. 20s cadence is granular enough
  // for minute-level copy and cheap enough to ignore.
  let nowTick = $state(Date.now());
  $effect(() => {
    const id = setInterval(() => { nowTick = Date.now(); }, 20_000);
    return () => clearInterval(id);
  });

  function formatRelative(from: number, now: number): string {
    const diffMs = Math.max(0, now - from);
    const sec = Math.floor(diffMs / 1000);
    if (sec < 10) return 'Saved just now';
    if (sec < 60) return `Saved ${sec}s ago`;
    const min = Math.floor(sec / 60);
    if (min < 60) return `Saved ${min} min ago`;
    const hr = Math.floor(min / 60);
    if (hr < 24) return `Saved ${hr} hr ago`;
    const day = Math.floor(hr / 24);
    return `Saved ${day}d ago`;
  }

  let saveLabel = $derived.by(() => {
    if (documentStore.isDirty) return 'Unsaved changes';
    if (documentStore.lastSavedAt === null) return '';
    return formatRelative(documentStore.lastSavedAt, nowTick);
  });

  let saveState = $derived.by<'dirty' | 'saved' | 'idle'>(() => {
    if (documentStore.isDirty) return 'dirty';
    if (documentStore.lastSavedAt !== null) return 'saved';
    return 'idle';
  });
</script>

<div class="status-bar">
  <div class="status-left">
    <button class="settings-btn" onclick={() => { showSettings = true; }} title="Settings">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="4" y1="21" x2="4" y2="14"></line><line x1="4" y1="10" x2="4" y2="3"></line>
        <line x1="12" y1="21" x2="12" y2="12"></line><line x1="12" y1="8" x2="12" y2="3"></line>
        <line x1="20" y1="21" x2="20" y2="16"></line><line x1="20" y1="12" x2="20" y2="3"></line>
        <line x1="1" y1="14" x2="7" y2="14"></line><line x1="9" y1="8" x2="15" y2="8"></line>
        <line x1="17" y1="16" x2="23" y2="16"></line>
      </svg>
    </button>
    {#if onShowHelp}
      <button class="settings-btn" onclick={() => onShowHelp?.()} title="How to use Scriptty">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"></circle>
          <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"></path>
          <line x1="12" y1="17" x2="12.01" y2="17"></line>
        </svg>
      </button>
    {/if}
    <span class="status-lang" class:malayalam={isMalayalam} title="Input language — toggle with ⌃Space">
      {isMalayalam ? 'MAL' : 'ENG'}
    </span>
  </div>
  <div class="status-right">
    {#if saveLabel}
      <span class="status-save" data-state={saveState} title={saveState === 'dirty' ? 'You have unsaved changes — ⌘S to save' : 'Last successful save'}>
        <span class="status-save-dot" aria-hidden="true"></span>
        {saveLabel}
      </span>
    {/if}
    {#if rightContent}
      {@render rightContent()}
    {/if}
  </div>
</div>

<SettingsModal bind:open={showSettings} bind:showAnnotations />

<style>
  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 28px;
    padding: 0 16px;
    background: var(--surface-elevated);
    border-top: 1px solid var(--border-subtle);
    font-size: 11px;
    font-family: system-ui, -apple-system, sans-serif;
    color: var(--text-muted);
    user-select: none;
    flex-shrink: 0;
    letter-spacing: 0.04em;
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .status-lang {
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 3px;
    transition: background 120ms ease, color 120ms ease;
  }

  .status-lang.malayalam {
    color: var(--accent);
    background: var(--accent-muted);
  }

  .settings-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    width: 24px;
    height: 24px;
    border-radius: 4px;
    transition: background 120ms, color 120ms;
  }

  .settings-btn:hover {
    color: var(--text-primary);
    background: var(--surface-hover);
  }

  .status-save {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    transition: color 160ms ease;
  }

  .status-save-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-muted);
    transition: background 160ms ease, box-shadow 160ms ease;
  }

  .status-save[data-state='saved'] {
    color: var(--text-secondary);
  }
  .status-save[data-state='saved'] .status-save-dot {
    background: var(--success);
  }

  .status-save[data-state='dirty'] {
    color: var(--accent-warm);
  }
  .status-save[data-state='dirty'] .status-save-dot {
    background: var(--accent-warm);
    box-shadow: 0 0 0 3px var(--accent-warm-muted);
  }
</style>
