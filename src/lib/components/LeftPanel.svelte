<script lang="ts">
  import SceneNavigator from './SceneNavigator.svelte';
  import SeriesEpisodeList from './SeriesEpisodeList.svelte';
  import { documentStore } from '$lib/stores/documentStore.svelte';

  let { isOpen }: { isOpen: boolean } = $props();

  let panelWidth = 320;
</script>

<aside class="left-panel" class:open={isOpen} style:--panel-width="{panelWidth}px">
  <div class="panel-content">
    {#if documentStore.isSeries}
      <SeriesEpisodeList />
    {:else}
      <SceneNavigator />
    {/if}
  </div>
</aside>

<style>
  /* The panel floats over the editor area so opening/closing it doesn't
     reflow the editor — the page stays centered in the viewport. */
  .left-panel {
    position: absolute;
    top: 0;
    left: 0;
    height: 100%;
    width: var(--panel-width, 240px);
    overflow: hidden;
    background: var(--surface-base);
    border-right: 1px solid var(--border-subtle);
    transform: translateX(-100%);
    transition: transform 200ms cubic-bezier(0.4, 0, 0.2, 1);
    z-index: 10;
    box-shadow: 2px 0 8px var(--shadow-soft);
  }

  .left-panel.open {
    transform: translateX(0);
  }

  .panel-content {
    width: var(--panel-width, 240px);
    height: 100%;
    display: flex;
    flex-direction: column;
  }
</style>
