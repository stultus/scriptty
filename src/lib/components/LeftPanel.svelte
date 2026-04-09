<script lang="ts">
  import SceneNavigator from './SceneNavigator.svelte';
  import StoryPanel from './StoryPanel.svelte';

  let { isOpen }: { isOpen: boolean } = $props();

  let panelWidth = 320;

  // Which tab is active: 'scenes' or 'story'
  let activeTab = $state<'scenes' | 'story'>('scenes');
</script>

<aside class="left-panel" class:open={isOpen} style:--panel-width="{panelWidth}px">
  <div class="panel-content">
    <!-- Tab switcher -->
    <div class="tab-bar">
      <button
        class="tab-btn"
        class:active={activeTab === 'scenes'}
        onclick={() => { activeTab = 'scenes'; }}
      >Scenes</button>
      <button
        class="tab-btn"
        class:active={activeTab === 'story'}
        onclick={() => { activeTab = 'story'; }}
      >Story</button>
    </div>

    <!-- Tab content -->
    <div class="tab-content">
      {#if activeTab === 'scenes'}
        <SceneNavigator />
      {:else}
        <StoryPanel />
      {/if}
    </div>
  </div>
</aside>

<style>
  .left-panel {
    width: 0;
    min-width: 0;
    overflow: hidden;
    background: var(--surface-base);
    border-right: 1px solid var(--border-subtle);
    transition: width 200ms cubic-bezier(0.4, 0, 0.2, 1),
                min-width 200ms cubic-bezier(0.4, 0, 0.2, 1);
    flex-shrink: 0;
  }

  .left-panel.open {
    width: var(--panel-width, 240px);
    min-width: var(--panel-width, 240px);
  }

  .panel-content {
    width: var(--panel-width, 240px);
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .tab-bar {
    display: flex;
    padding: 8px 12px 0;
    gap: 2px;
    flex-shrink: 0;
  }

  .tab-btn {
    flex: 1;
    padding: 5px 0;
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    background: transparent;
    color: var(--text-muted);
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: color 120ms ease, border-color 120ms ease;
  }

  .tab-btn:hover {
    color: var(--text-secondary);
  }

  .tab-btn.active {
    color: var(--text-primary);
    border-bottom-color: var(--accent);
  }

  .tab-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
</style>
