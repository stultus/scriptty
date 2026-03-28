<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/plugin-dialog';
  import Editor from '$lib/components/Editor.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import LeftPanel from '$lib/components/LeftPanel.svelte';
  import SceneCardsView from '$lib/components/SceneCardsView.svelte';
  import StoryModeView from '$lib/components/StoryModeView.svelte';
  import AboutModal from '$lib/components/AboutModal.svelte';
  import HelpModal from '$lib/components/HelpModal.svelte';
  import StatisticsModal from '$lib/components/StatisticsModal.svelte';
  import MetadataModal from '$lib/components/MetadataModal.svelte';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';
  import { themeStore } from '$lib/stores/themeStore.svelte';
  import { toggleMark } from 'prosemirror-commands';
  import { screenplaySchema } from '$lib/editor/schema';

  let panelOpen = $state(false);
  let showAbout = $state(false);
  let showHelp = $state(false);
  let showSceneCards = $state(false);
  let showStoryMode = $state(false);
  let findReplaceOpen = $state(false);
  let findReplaceMode = $state<'find' | 'replace'>('find');
  let showStatistics = $state(false);
  let showMetadata = $state(false);

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
      // Cmd+Shift+L — Toggle Story Mode
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'l') {
        event.preventDefault();
        showStoryMode = !showStoryMode;
        return;
      }
      // Cmd+Shift+I — Script Statistics
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'i') {
        event.preventDefault();
        showStatistics = true;
        return;
      }
      // Cmd+F — Open Find bar
      if ((event.metaKey || event.ctrlKey) && !event.shiftKey && event.key === 'f') {
        event.preventDefault();
        findReplaceOpen = true;
        findReplaceMode = 'find';
        return;
      }
      // Cmd+Shift+H — Open Find and Replace bar
      // (Cmd+H is macOS "Hide Window", so we use Cmd+Shift+H)
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'h') {
        event.preventDefault();
        findReplaceOpen = true;
        findReplaceMode = 'replace';
        return;
      }
      // Ctrl+\ (Mac: Cmd+\) toggles left panel
      if ((event.metaKey || event.ctrlKey) && event.key === '\\') {
        event.preventDefault();
        panelOpen = !panelOpen;
      }
    }

    window.addEventListener('keydown', handleGlobalKeydown);

    let unlistens: (() => void)[] = [];

    // Listen for native menu events emitted from the Rust backend.
    // Each custom menu item (New, Open, Save, Save As) emits an event
    // that we handle here to call the appropriate store method.
    (async () => {
      unlistens.push(await listen('menu-new', async () => {
        if (!(await documentStore.confirmIfDirty())) return;
        await documentStore.newDocument();
      }));

      unlistens.push(await listen('menu-open', async () => {
        if (!(await documentStore.confirmIfDirty())) return;
        const path = await open({
          multiple: false,
          filters: [{ name: 'Screenplay', extensions: ['screenplay'] }]
        });
        if (typeof path === 'string') {
          await documentStore.openDocument(path);
        }
      }));

      unlistens.push(await listen('menu-save', () => {
        documentStore.saveWithDialog();
      }));

      unlistens.push(await listen('menu-save-as', () => {
        documentStore.saveAsDialog();
      }));

      unlistens.push(await listen('menu-about', () => {
        showAbout = true;
      }));

      unlistens.push(await listen('menu-help-guide', () => {
        showHelp = true;
      }));

      unlistens.push(await listen('menu-statistics', () => {
        showStatistics = true;
      }));

      unlistens.push(await listen('menu-scene-cards', () => {
        showSceneCards = !showSceneCards;
      }));

      unlistens.push(await listen('menu-story-mode', () => {
        showStoryMode = !showStoryMode;
      }));

      unlistens.push(await listen('menu-find', () => {
        findReplaceOpen = true;
        findReplaceMode = 'find';
      }));

      unlistens.push(await listen('menu-find-replace', () => {
        findReplaceOpen = true;
        findReplaceMode = 'replace';
      }));

      // Format menu — toggle inline marks on the editor
      unlistens.push(await listen('menu-bold', () => {
        if (editorStore.view) {
          toggleMark(screenplaySchema.marks.bold)(editorStore.view.state, editorStore.view.dispatch);
          editorStore.view.focus();
        }
      }));

      unlistens.push(await listen('menu-italic', () => {
        if (editorStore.view) {
          toggleMark(screenplaySchema.marks.italic)(editorStore.view.state, editorStore.view.dispatch);
          editorStore.view.focus();
        }
      }));

      unlistens.push(await listen('menu-underline', () => {
        if (editorStore.view) {
          toggleMark(screenplaySchema.marks.underline)(editorStore.view.state, editorStore.view.dispatch);
          editorStore.view.focus();
        }
      }));

      unlistens.push(await listen('menu-toggle-sidebar', () => {
        panelOpen = !panelOpen;
      }));

      unlistens.push(await listen('menu-edit-meta', () => {
        showMetadata = true;
      }));

      unlistens.push(await listen('menu-quit', async () => {
        if (!(await documentStore.confirmIfDirty())) return;
        quitConfirmed = true;
        await getCurrentWindow().close();
      }));

      // Handle file open requests — when a .screenplay file is double-clicked
      // in the OS file manager, the backend emits this event with the file path.
      unlistens.push(await listen<string>('file-open-request', async (event) => {
        const filePath = event.payload;
        if (typeof filePath === 'string' && filePath.endsWith('.screenplay')) {
          if (!(await documentStore.confirmIfDirty())) return;
          await documentStore.openDocument(filePath);
        }
      }));

      // Intercept window close to prompt for unsaved changes
      unlistens.push(await getCurrentWindow().onCloseRequested(async (event) => {
        if (quitConfirmed) return; // Already confirmed via menu-quit
        if (!(await documentStore.confirmIfDirty())) {
          event.preventDefault();
        }
      }));
    })();

    return () => {
      window.removeEventListener('keydown', handleGlobalKeydown);
      unlistens.forEach((fn) => fn());
    };
  });
</script>

<main>
  <TitleBar onToggleSidebar={() => { panelOpen = !panelOpen; }} />
  <div class="workspace">
    {#if showSceneCards}
      <SceneCardsView onClose={() => { showSceneCards = false; }} />
    {/if}
    {#if showStoryMode}
      <StoryModeView onClose={() => { showStoryMode = false; }} />
    {/if}
    <div class="editor-area" class:hidden={showSceneCards || showStoryMode}>
      <LeftPanel isOpen={panelOpen} />
      <Editor bind:findReplaceOpen bind:findReplaceMode />
    </div>
  </div>
</main>

<AboutModal bind:open={showAbout} />
<HelpModal bind:open={showHelp} />
<StatisticsModal bind:open={showStatistics} />
<MetadataModal bind:open={showMetadata} />

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

  .editor-area {
    display: flex;
    flex-direction: row;
    flex: 1;
    overflow: hidden;
  }

  .editor-area.hidden {
    display: none;
  }
</style>
