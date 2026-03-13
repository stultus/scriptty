<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/plugin-dialog';
  import Editor from '$lib/components/Editor.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import LeftPanel from '$lib/components/LeftPanel.svelte';
  import SceneCardsView from '$lib/components/SceneCardsView.svelte';
  import AboutModal from '$lib/components/AboutModal.svelte';
  import HelpModal from '$lib/components/HelpModal.svelte';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { themeStore } from '$lib/stores/themeStore.svelte';

  let panelOpen = $state(false);
  let showAbout = $state(false);
  let showHelp = $state(false);
  let showSceneCards = $state(false);

  // Module-level guard — prevents newDocument() from firing again on HMR re-mount
  let appInitialized = false;
  // Set to true after menu-quit confirms — prevents onCloseRequested from prompting again
  let quitConfirmed = false;

  onMount(async () => {
    // Initialize theme on first mount
    themeStore.init();

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
      // Cmd+Shift+K — Toggle scene cards view
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key === 'k') {
        event.preventDefault();
        showSceneCards = !showSceneCards;
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
        documentStore.confirmIfDirty().then((proceed) => {
          if (!proceed) return;
          open({
            multiple: false,
            filters: [{ name: 'Screenplay', extensions: ['screenplay'] }]
          }).then((path) => {
            if (typeof path === 'string') {
              documentStore.openDocument(path);
            }
          });
        });
      }
      // Ctrl+B (Mac: Cmd+B) toggles left panel
      if ((event.metaKey || event.ctrlKey) && event.key === 'b') {
        event.preventDefault();
        panelOpen = !panelOpen;
      }
    }

    window.addEventListener('keydown', handleGlobalKeydown);

    // Listen for native menu events emitted from the Rust backend.
    // Each custom menu item (New, Open, Save, Save As) emits an event
    // that we handle here to call the appropriate store method.
    const unlistenNew = await listen('menu-new', async () => {
      if (!(await documentStore.confirmIfDirty())) return;
      await documentStore.newDocument();
    });

    const unlistenOpen = await listen('menu-open', async () => {
      if (!(await documentStore.confirmIfDirty())) return;
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

    const unlistenAbout = await listen('menu-about', () => {
      showAbout = true;
    });

    const unlistenHelpGuide = await listen('menu-help-guide', () => {
      showHelp = true;
    });

    const unlistenQuit = await listen('menu-quit', async () => {
      if (!(await documentStore.confirmIfDirty())) return;
      // All clear — skip the onCloseRequested check and close the window
      quitConfirmed = true;
      await getCurrentWindow().close();
    });

    // Intercept window close to prompt for unsaved changes
    const unlistenClose = await getCurrentWindow().onCloseRequested(async (event) => {
      if (quitConfirmed) return; // Already confirmed via menu-quit
      if (!(await documentStore.confirmIfDirty())) {
        event.preventDefault();
      }
    });

    return () => {
      window.removeEventListener('keydown', handleGlobalKeydown);
      unlistenNew();
      unlistenOpen();
      unlistenSave();
      unlistenSaveAs();
      unlistenAbout();
      unlistenHelpGuide();
      unlistenQuit();
      unlistenClose();
    };
  });
</script>

<main>
  <TitleBar />
  <div class="workspace">
    {#if showSceneCards}
      <SceneCardsView onClose={() => { showSceneCards = false; }} />
    {:else}
      <LeftPanel isOpen={panelOpen} />
      <Editor />
    {/if}
  </div>
</main>

<AboutModal bind:open={showAbout} />
<HelpModal bind:open={showHelp} />

<style>
  main {
    width: 100vw;
    height: 100vh;
    background: var(--surface-base);
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
