<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
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

    // Window-level keyboard shortcuts — works even when editor isn't focused.
    // Note: Cmd+N, Cmd+O, Cmd+S, Cmd+Shift+S are also handled by the native
    // menu accelerators (which emit Tauri events). We keep these keydown handlers
    // as a fallback — if the native menu intercepts the key first, the keydown
    // event won't reach the webview, so no double-execution occurs.
    function handleGlobalKeydown(event: KeyboardEvent) {
      // Cmd+Shift+S — Save As (must be checked BEFORE Cmd+S since both have metaKey+s)
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key === 's') {
        event.preventDefault();
        documentStore.saveAsDialog();
        return;
      }
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

    // Listen for native menu events emitted from the Rust backend.
    // Each custom menu item (New, Open, Save, Save As) emits an event
    // that we handle here to call the appropriate store method.
    const unlistenNew = await listen('menu-new', () => {
      documentStore.newDocument();
    });

    const unlistenOpen = await listen('menu-open', async () => {
      const path = await open({
        multiple: false,
        filters: [{ name: 'Screenplay', extensions: ['screenplay'] }]
      });
      if (typeof path === 'string') {
        await documentStore.openDocument(path);
      }
    });

    const unlistenSave = await listen('menu-save', () => {
      documentStore.saveWithDialog();
    });

    const unlistenSaveAs = await listen('menu-save-as', () => {
      documentStore.saveAsDialog();
    });

    return () => {
      window.removeEventListener('keydown', handleGlobalKeydown);
      unlistenNew();
      unlistenOpen();
      unlistenSave();
      unlistenSaveAs();
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
