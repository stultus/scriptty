<script lang="ts">
  import { InputModeManager } from '$lib/editor/input/InputModeManager';

  let {
    rightContent,
    onOpenPalette,
  }: {
    rightContent?: import('svelte').Snippet;
    onOpenPalette?: () => void;
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
</script>

<div class="status-bar">
  <div class="status-left">
    <button
      class="palette-hint"
      onclick={() => onOpenPalette?.()}
      title="Open the command palette (⌘K) to access settings, export, help, and all other actions"
    >
      <span class="kbd">⌘K</span>
      <span class="palette-hint-label">Commands</span>
    </button>
    <span class="status-lang" class:malayalam={isMalayalam} title="Input language — toggle with ⌃Space">
      {isMalayalam ? 'MAL' : 'ENG'}
    </span>
  </div>
  <div class="status-right">
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
    gap: 10px;
  }

  .status-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  /* The palette hint replaces the cluster of icon buttons — it surfaces
     the one shortcut the writer needs to reach every other action, and
     stays visually quiet so it doesn't compete with the writing surface. */
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

  .status-lang {
    color: var(--text-secondary);
    font-size: 12px;
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
</style>
