<script lang="ts">
  import { InputModeManager } from '$lib/editor/input/InputModeManager';
  import { documentStore } from '$lib/stores/documentStore.svelte';

  let {
    rightContent,
    onOpenPalette,
    onOpenSettings,
    onShowHelp,
  }: {
    rightContent?: import('svelte').Snippet;
    onOpenPalette?: () => void;
    onOpenSettings?: () => void;
    onShowHelp?: () => void;
  } = $props();

  const inputManager = InputModeManager.getInstance();
  let isMalayalam = $state(inputManager.isMalayalam);

  // Subscribe to input manager changes so the indicator stays in sync
  // when toggled via Ctrl+Space from any view. No polling.
  $effect(() => {
    return inputManager.subscribe(() => {
      isMalayalam = inputManager.isMalayalam;
    });
  });

  // Ticking clock for the "Saved N min ago" relative timestamp.
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
    <!-- Zone 1: chrome actions -->
    <div class="zone">
      <button
        class="icon-btn"
        onclick={() => onOpenSettings?.()}
        title="Settings — language, input scheme, font, theme"
        aria-label="Open settings"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="4" y1="21" x2="4" y2="14"></line><line x1="4" y1="10" x2="4" y2="3"></line>
          <line x1="12" y1="21" x2="12" y2="12"></line><line x1="12" y1="8" x2="12" y2="3"></line>
          <line x1="20" y1="21" x2="20" y2="16"></line><line x1="20" y1="12" x2="20" y2="3"></line>
          <line x1="1" y1="14" x2="7" y2="14"></line><line x1="9" y1="8" x2="15" y2="8"></line>
          <line x1="17" y1="16" x2="23" y2="16"></line>
        </svg>
      </button>
      {#if onShowHelp}
        <button
          class="icon-btn"
          onclick={() => onShowHelp?.()}
          title="How to use Scriptty"
          aria-label="Open help"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"></path>
            <line x1="12" y1="17" x2="12.01" y2="17"></line>
          </svg>
        </button>
      {/if}
    </div>

    <span class="separator" aria-hidden="true"></span>

    <!-- Zone 2: command palette entry -->
    <div class="zone">
      <button
        class="palette-hint"
        onclick={() => onOpenPalette?.()}
        title="Open the command palette (⌘K) to run any action"
      >
        <span class="kbd">⌘K</span>
        <span>Commands</span>
      </button>
    </div>

    <span class="separator" aria-hidden="true"></span>

    <!-- Zone 3: input-mode toggle. Now actually clickable so the chip
         matches the affordance the tooltip implies. -->
    <div class="zone">
      <button
        type="button"
        class="status-lang"
        class:malayalam={isMalayalam}
        onclick={() => inputManager.toggle()}
        title="Input language — click or ⌃Space to toggle"
        aria-label={isMalayalam ? 'Switch to English input' : 'Switch to Malayalam input'}
      >
        {isMalayalam ? 'MAL' : 'ENG'}
      </button>
    </div>
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

<style>
  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 28px;
    padding: 0 16px;
    background: var(--surface-elevated);
    border-top: 1px solid var(--border-subtle);
    font-size: 12px;
    font-family: var(--ui-font);
    color: var(--text-secondary);
    user-select: none;
    flex-shrink: 0;
    letter-spacing: 0.03em;
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  /* Group of related controls — separators visually demarcate the zones
     so the eye reads "actions | palette | input mode" instead of one
     undifferentiated row of chips (#109). */
  .zone {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .separator {
    display: inline-block;
    width: 1px;
    height: 14px;
    background: var(--border-subtle);
    margin: 0 4px;
    flex-shrink: 0;
  }

  .status-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .icon-btn {
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
    transition: background 120ms ease, color 120ms ease;
  }

  .icon-btn:hover {
    color: var(--text-primary);
    background: var(--surface-hover);
  }

  /* Palette hint sits alongside the icon cluster — the ⌘K kbd pill is
     the discoverability surface for the command palette without pushing
     out the familiar Settings/Help buttons the writer already knows. */
  .palette-hint {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px 2px 6px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-family: var(--ui-font);
    font-size: 12px;
    letter-spacing: 0.03em;
    cursor: pointer;
    border-radius: 4px;
    transition: background 120ms ease, color 120ms ease;
  }

  .palette-hint:hover {
    color: var(--text-primary);
    background: var(--surface-hover);
  }

  .palette-hint .kbd {
    display: inline-flex;
    align-items: center;
    height: 16px;
    padding: 0 5px;
    border-radius: 3px;
    border: 1px solid var(--border-medium);
    background: var(--surface-base);
    color: var(--text-secondary);
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0;
  }

  /* Input-mode toggle. Reads as a chip (state-bearing) but is now a
     real button — click toggles the input mode just like ⌃Space. */
  .status-lang {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-family: var(--ui-font);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 3px;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .status-lang:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .status-lang.malayalam {
    color: var(--accent);
    background: var(--accent-muted);
  }

  .status-lang.malayalam:hover {
    background: var(--accent-muted);
    color: var(--accent);
    filter: brightness(1.05);
  }

  .status-save {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    letter-spacing: 0.03em;
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

  .status-save[data-state='saved'] { color: var(--text-secondary); }
  .status-save[data-state='saved'] .status-save-dot { background: var(--success); }

  .status-save[data-state='dirty'] { color: var(--accent-warm); }
  .status-save[data-state='dirty'] .status-save-dot {
    background: var(--accent-warm);
    box-shadow: 0 0 0 3px var(--accent-warm-muted);
  }
</style>
