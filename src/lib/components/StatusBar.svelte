<script lang="ts">
  import { InputModeManager } from '$lib/editor/input/InputModeManager';
  import SettingsModal from './SettingsModal.svelte';

  let {
    rightContent,
    showAnnotations = $bindable(true),
  }: {
    rightContent?: import('svelte').Snippet;
    showAnnotations?: boolean;
  } = $props();

  const inputManager = InputModeManager.getInstance();
  let isMalayalam = $state(inputManager.isMalayalam);
  let showSettings = $state(false);

  // Poll the input manager state so the indicator stays in sync
  // when toggled via Ctrl+Space from any view.
  $effect(() => {
    const interval = setInterval(() => {
      isMalayalam = inputManager.isMalayalam;
    }, 200);
    return () => clearInterval(interval);
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
</style>
