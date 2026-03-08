<script lang="ts">
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import Editor from '$lib/components/Editor.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import SceneNavigator from '$lib/components/SceneNavigator.svelte';
  import { documentStore } from '$lib/stores/documentStore.svelte';

  let navigatorOpen = $state(false);

  // Module-level guard — prevents newDocument() from firing again on HMR re-mount
  let appInitialized = false;

  onMount(async () => {
    if (!appInitialized) {
      appInitialized = true;
      if (!documentStore.document) {
        await documentStore.newDocument();
      }
    }

    // Window-level keyboard shortcuts — works even when editor isn't focused
    function handleGlobalKeydown(event: KeyboardEvent) {
      // Cmd+S (Mac) / Ctrl+S (Windows/Linux)
      if ((event.metaKey || event.ctrlKey) && event.key === 's') {
        event.preventDefault();
        documentStore.saveWithDialog();
      }
      // Cmd+O (Mac) / Ctrl+O (Windows/Linux)
      if ((event.metaKey || event.ctrlKey) && event.key === 'o') {
        event.preventDefault();
        open({
          multiple: false,
          filters: [{ name: 'Screenplay', extensions: ['screenplay'] }]
        }).then((path) => {
          if (typeof path === 'string') {
            documentStore.openDocument(path);
          }
        });
      }
      // Ctrl+B (Mac: Cmd+B) toggles scene navigator
      if ((event.metaKey || event.ctrlKey) && event.key === 'b') {
        event.preventDefault();
        navigatorOpen = !navigatorOpen;
      }
    }

    window.addEventListener('keydown', handleGlobalKeydown);

    return () => {
      window.removeEventListener('keydown', handleGlobalKeydown);
    };
  });
</script>

<main>
  <TitleBar />
  <div class="workspace">
    <SceneNavigator isOpen={navigatorOpen} />
    <Editor />
  </div>
</main>

<style>
  main {
    width: 100vw;
    height: 100vh;
    background: #1a1a1a;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .workspace {
    flex: 1;
    display: flex;
    flex-direction: row;
    overflow: hidden;
  }
</style>
