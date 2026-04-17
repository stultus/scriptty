<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open, message } from '@tauri-apps/plugin-dialog';
  import { getCurrent as getDeepLinkUrls, onOpenUrl } from '@tauri-apps/plugin-deep-link';
  import Editor from '$lib/components/Editor.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import LeftPanel from '$lib/components/LeftPanel.svelte';
  import SceneCardsView from '$lib/components/SceneCardsView.svelte';
  import StoryModeView from '$lib/components/StoryModeView.svelte';
  import AboutModal from '$lib/components/AboutModal.svelte';
  import HelpModal from '$lib/components/HelpModal.svelte';
  import StatisticsModal from '$lib/components/StatisticsModal.svelte';
  import MetadataModal from '$lib/components/MetadataModal.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import OutlinePeek from '$lib/components/OutlinePeek.svelte';
  import CommandPalette, { type Command } from '$lib/components/CommandPalette.svelte';
  import SettingsModal from '$lib/components/SettingsModal.svelte';
  import ExportModal from '$lib/components/ExportModal.svelte';
  import UpdateToast from '$lib/components/UpdateToast.svelte';
  import WelcomeScreen from '$lib/components/WelcomeScreen.svelte';
  import SeriesTitleDialog from '$lib/components/SeriesTitleDialog.svelte';
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { editorStore } from '$lib/stores/editorStore.svelte';
  import { themeStore } from '$lib/stores/themeStore.svelte';
  import { updateStore } from '$lib/stores/updateStore.svelte';
  import { InputModeManager } from '$lib/editor/input/InputModeManager';
  import { toggleMark } from 'prosemirror-commands';
  import { screenplaySchema } from '$lib/editor/schema';

  let panelOpen = $state(false);
  let showAbout = $state(false);
  let showHelp = $state(false);
  let activeView = $state<'writing' | 'cards' | 'story'>('writing');
  let findReplaceOpen = $state(false);
  let findReplaceMode = $state<'find' | 'replace'>('find');
  let showStatistics = $state(false);
  let editorRef: { editCurrentSceneAnnotation: () => void } | undefined;
  let showMetadata = $state(false);
  let showAnnotations = $state(typeof localStorage !== 'undefined' ? localStorage.getItem('scriptty-annotations') !== 'false' : true);
  let showOutlinePeek = $state(typeof localStorage !== 'undefined' ? localStorage.getItem('scriptty-outline-peek') === 'true' : false);
  let showSettings = $state(false);
  let showExport = $state(false);
  let showCommandPalette = $state(false);
  let showSeriesDialog = $state(false);

  // Remember recent files client-side — welcome screen pulls from this list.
  function pushRecentFile(path: string) {
    if (typeof localStorage === 'undefined' || !path) return;
    try {
      const name = (path.split('/').pop() ?? path.split('\\').pop() ?? path).replace(/\.screenplay$/, '');
      const raw = localStorage.getItem('scriptty-recent-files');
      const list = raw ? (JSON.parse(raw) as { path: string; name: string }[]) : [];
      const filtered = Array.isArray(list) ? list.filter((p) => p && p.path !== path) : [];
      filtered.unshift({ path, name });
      localStorage.setItem('scriptty-recent-files', JSON.stringify(filtered.slice(0, 6)));
    } catch {
      // ignore localStorage quota / JSON errors
    }
  }

  async function handleOpenFromWelcome() {
    await openFileDialog();
  }

  async function handleCreateSeriesFromDialog(title: string) {
    showSeriesDialog = false;
    if (!(await documentStore.confirmIfDirty())) return;
    await documentStore.newSeries(title);
  }

  const inputManager = InputModeManager.getInstance();
  // Reactive mirror of isMalayalam and scheme so command palette labels
  // stay in sync with ⌃Space + scheme toggles happening anywhere in the app.
  let isMalayalam = $state(inputManager.isMalayalam);
  let inputScheme = $state(inputManager.scheme);
  $effect(() => inputManager.subscribe(() => {
    isMalayalam = inputManager.isMalayalam;
    inputScheme = inputManager.scheme;
  }));

  // Whenever a file is opened or saved to a new path, record it in the recent
  // list so the Welcome screen can surface it next launch.
  $effect(() => {
    const path = documentStore.currentPath;
    if (path) pushRecentFile(path);
  });

  // Keep the OS window title in sync with the open document so the dock,
  // taskbar, and Cmd+Tab preview all show something meaningful instead of
  // the bundled app name.
  $effect(() => {
    const doc = documentStore.document;
    if (!doc) {
      getCurrentWindow().setTitle('Scriptty').catch(() => {});
      return;
    }
    let title: string;
    if (doc.type === 'series') {
      const seriesTitle = doc.series?.title || 'Untitled Series';
      const ep = documentStore.activeEpisode;
      const epLabel = ep ? (ep.title.trim() ? `Ep ${ep.number} · ${ep.title}` : `Ep ${ep.number}`) : '';
      title = epLabel ? `${seriesTitle} — ${epLabel}` : seriesTitle;
    } else {
      title = doc.meta.title || documentStore.displayTitle || 'Untitled';
    }
    getCurrentWindow().setTitle(`${title} — Scriptty`).catch(() => {});
  });

  // Word count for story view
  let storyWordCount = $derived(() => {
    const story = documentStore.activeStory;
    if (!story) return 0;
    const text = [story.idea, story.synopsis, story.treatment, story.narrative].join(' ').trim();
    if (!text) return 0;
    return text.split(/\s+/).length;
  });

  async function openFileDialog() {
    if (!(await documentStore.confirmIfDirty())) return;
    const path = await open({
      multiple: false,
      filters: [{ name: 'Screenplay', extensions: ['screenplay'] }]
    });
    if (typeof path === 'string') {
      await documentStore.openDocument(path);
      if (documentStore.currentPath) pushRecentFile(documentStore.currentPath);
    }
  }

  function toggleOutlinePeek() {
    showOutlinePeek = !showOutlinePeek;
    localStorage.setItem('scriptty-outline-peek', String(showOutlinePeek));
  }

  function toggleAnnotations() {
    if (activeView !== 'writing') return;
    showAnnotations = !showAnnotations;
    localStorage.setItem('scriptty-annotations', String(showAnnotations));
  }

  let commands = $derived<Command[]>([
    // File
    { id: 'file.new-film', group: 'File', label: 'New Film', hint: '⌘N',
      action: async () => { if (await documentStore.confirmIfDirty()) await documentStore.newDocument(); } },
    { id: 'file.new-series', group: 'File', label: 'New Series…', hint: '⌘⇧N',
      action: () => { showSeriesDialog = true; } },
    { id: 'file.open', group: 'File', label: 'Open…', hint: '⌘O', action: openFileDialog },
    { id: 'file.save', group: 'File', label: 'Save', hint: '⌘S', action: () => documentStore.saveWithDialog() },
    { id: 'file.saveas', group: 'File', label: 'Save As…', hint: '⌘⇧S', action: () => documentStore.saveAsDialog() },
    { id: 'file.export', group: 'File', label: 'Export…', keywords: 'pdf fountain plain text hollywood indian', action: () => { showExport = true; } },
    { id: 'file.meta', group: 'File', label: 'Edit Metadata…', keywords: 'title author director contact draft', action: () => { showMetadata = true; } },

    // View
    { id: 'view.writing', group: 'View', label: 'Writing View', action: () => { activeView = 'writing'; } },
    { id: 'view.cards', group: 'View', label: 'Scene Cards', hint: '⌘⇧K', action: () => { activeView = activeView === 'cards' ? 'writing' : 'cards'; } },
    { id: 'view.story', group: 'View', label: 'Story Mode', hint: '⌘⇧L', action: () => { activeView = activeView === 'story' ? 'writing' : 'story'; } },
    { id: 'view.sidebar', group: 'View', label: 'Toggle Scene Navigator', hint: '⌘\\', keywords: 'sidebar panel scenes',
      action: () => { if (activeView === 'writing') panelOpen = !panelOpen; } },
    { id: 'view.outline', group: 'View', label: 'Toggle Outline Peek', hint: '⌘⇧O', keywords: 'timeline strip',
      action: toggleOutlinePeek },
    { id: 'view.annotations', group: 'View', label: 'Toggle Scene Annotations', hint: '⌘⇧A', keywords: 'notes comments',
      action: toggleAnnotations },

    // Edit
    { id: 'edit.find', group: 'Edit', label: 'Find…', hint: '⌘F',
      action: () => { findReplaceOpen = true; findReplaceMode = 'find'; } },
    { id: 'edit.replace', group: 'Edit', label: 'Find and Replace…', hint: '⌘⇧H',
      action: () => { findReplaceOpen = true; findReplaceMode = 'replace'; } },
    { id: 'edit.stats', group: 'Edit', label: 'Script Statistics', hint: '⌘⇧I', keywords: 'count pages scenes',
      action: () => { showStatistics = true; } },
    { id: 'edit.annotate', group: 'Edit', label: 'Annotate Current Scene', hint: '⌘⇧D',
      action: () => {
        if (activeView !== 'writing') return;
        if (!showAnnotations) {
          showAnnotations = true;
          localStorage.setItem('scriptty-annotations', 'true');
        }
        editorRef?.editCurrentSceneAnnotation();
      } },

    // Settings
    { id: 'settings.open', group: 'Settings', label: 'Settings…', keywords: 'language font scheme theme preferences', action: () => { showSettings = true; } },
    { id: 'settings.theme', group: 'Settings', label: `Switch to ${themeStore.isDark ? 'Light' : 'Dark'} Mode`, keywords: 'appearance',
      action: () => { themeStore.toggle(); } },
    { id: 'settings.lang', group: 'Settings', label: `Switch to ${isMalayalam ? 'English' : 'Malayalam'} Input`, hint: '⌃Space', keywords: 'mal eng mode',
      action: () => { inputManager.toggle(); } },
    { id: 'settings.scheme.mozhi', group: 'Settings',
      label: inputScheme === 'mozhi' ? 'Malayalam Scheme: Mozhi (active)' : 'Use Malayalam Scheme: Mozhi',
      keywords: 'keyboard transliteration phonetic',
      action: () => { inputManager.setScheme('mozhi'); if (!isMalayalam) inputManager.toggle(); } },
    { id: 'settings.scheme.inscript2', group: 'Settings',
      label: inputScheme === 'inscript2' ? 'Malayalam Scheme: Inscript 2 (active)' : 'Use Malayalam Scheme: Inscript 2',
      keywords: 'keyboard layout',
      action: () => { inputManager.setScheme('inscript2'); if (!isMalayalam) inputManager.toggle(); } },
    { id: 'settings.scheme.inscript1', group: 'Settings',
      label: inputScheme === 'inscript1' ? 'Malayalam Scheme: Inscript 1 (active)' : 'Use Malayalam Scheme: Inscript 1',
      keywords: 'keyboard layout legacy',
      action: () => { inputManager.setScheme('inscript1'); if (!isMalayalam) inputManager.toggle(); } },

    // Help
    { id: 'help.guide', group: 'Help', label: 'How to Use Scriptty', keywords: 'help guide shortcuts',
      action: () => { showHelp = true; } },
    { id: 'help.about', group: 'Help', label: 'About Scriptty', action: () => { showAbout = true; } },
    { id: 'help.updates', group: 'Help', label: 'Check for Updates…',
      action: async () => {
        const result = await updateStore.check();
        if (result === 'current') {
          await message("You're on the latest version of Scriptty.", { title: 'No updates available', kind: 'info' });
        } else if (result === 'error') {
          await message('Could not reach the update server. Check your internet connection and try again.', { title: 'Update check failed', kind: 'warning' });
        }
      } },
  ]);

  // Module-level guard — prevents newDocument() from firing again on HMR re-mount
  let appInitialized = false;
  // Set to true after menu-quit confirms — prevents onCloseRequested from prompting again
  let quitConfirmed = false;

  onMount(() => {
    // Initialize theme on first mount
    themeStore.init();

    // Kick off async startup in an IIFE so onMount itself stays synchronous.
    // Svelte's onMount only treats a synchronously-returned function as its
    // cleanup — returning a Promise would drop the cleanup on the floor.
    (async () => {
      if (!appInitialized) {
        appInitialized = true;
        // Check if the app was launched by double-clicking a .screenplay file.
        // The deep-link plugin buffers URLs on cold launch so getCurrent()
        // returns them reliably after the frontend mounts.
        let openedFile = false;
        try {
          const urls = await getDeepLinkUrls();
          if (urls && urls.length > 0) {
            for (const url of urls) {
              // On macOS, file associations come as file:// URLs
              const filePath = url.startsWith('file://') ? decodeURIComponent(url.replace('file://', '')) : url;
              if (filePath.endsWith('.screenplay')) {
                await documentStore.openDocument(filePath);
                openedFile = true;
                break;
              }
            }
          }
        } catch {
          // Plugin may not be available in dev mode — ignore
        }
        if (openedFile && documentStore.currentPath) {
          pushRecentFile(documentStore.currentPath);
        }
        // If nothing opened, fall through to the Welcome screen instead of
        // auto-creating a new doc — the user picks Film / Series / Open there.
      }
    })();

    // Check for a newer release once, a few seconds after launch so we don't
    // compete with file I/O or editor mount. Silent on failure — the app is
    // fully usable offline and we don't want a nag on first-run network issues.
    const updateCheckTimer = setTimeout(() => {
      updateStore.check();
    }, 3000);

    // Window-level keyboard shortcuts — works even when editor isn't focused.
    // Note: Cmd+N, Cmd+O, Cmd+S, Cmd+Shift+S are also handled by the native
    // menu accelerators (which emit Tauri events). We keep these keydown handlers
    // as a fallback — if the native menu intercepts the key first, the keydown
    // event won't reach the webview, so no double-execution occurs.
    function handleGlobalKeydown(event: KeyboardEvent) {
      // Cmd+K — Command palette (take precedence over all other shortcuts)
      if ((event.metaKey || event.ctrlKey) && !event.shiftKey && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        showCommandPalette = !showCommandPalette;
        return;
      }
      // Cmd+Shift+S — Save As (must be checked BEFORE Cmd+S since both have metaKey+s)
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key === 's') {
        event.preventDefault();
        documentStore.saveAsDialog();
        return;
      }
      // Cmd+Shift+K — Toggle scene cards view
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key === 'k') {
        event.preventDefault();
        activeView = activeView === 'cards' ? 'writing' : 'cards';
        return;
      }
      // Cmd+S (Mac) / Ctrl+S (Windows/Linux)
      if ((event.metaKey || event.ctrlKey) && event.key === 's') {
        event.preventDefault();
        documentStore.saveWithDialog();
      }
      // Cmd+O (Mac) / Ctrl+O (Windows/Linux)
      // Guard against Shift so Cmd+Shift+O (outline toggle) doesn't fall through.
      if ((event.metaKey || event.ctrlKey) && !event.shiftKey && event.key === 'o') {
        event.preventDefault();
        openFileDialog();
      }
      // Cmd+Shift+L — Toggle Story Mode
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'l') {
        event.preventDefault();
        activeView = activeView === 'story' ? 'writing' : 'story';
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
      // Ctrl+\ (Mac: Cmd+\) toggles left panel (writing view only)
      if ((event.metaKey || event.ctrlKey) && event.key === '\\') {
        event.preventDefault();
        if (activeView === 'writing') panelOpen = !panelOpen;
        return;
      }
      // Cmd+Shift+A — Toggle annotations in writing view
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'a') {
        event.preventDefault();
        if (activeView === 'writing') {
          showAnnotations = !showAnnotations;
          localStorage.setItem('scriptty-annotations', String(showAnnotations));
        }
        return;
      }
      // Cmd+Shift+O — Toggle outline peek strip
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'o') {
        event.preventDefault();
        showOutlinePeek = !showOutlinePeek;
        localStorage.setItem('scriptty-outline-peek', String(showOutlinePeek));
        return;
      }
      // Cmd+Shift+D — Add/edit annotation for current scene
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'd') {
        event.preventDefault();
        if (activeView === 'writing') {
          // Ensure annotations are visible first
          if (!showAnnotations) {
            showAnnotations = true;
            localStorage.setItem('scriptty-annotations', 'true');
          }
          editorRef?.editCurrentSceneAnnotation();
        }
      }
    }

    window.addEventListener('keydown', handleGlobalKeydown);

    let unlistens: (() => void)[] = [];

    // Listen for native menu events emitted from the Rust backend.
    // Each custom menu item (New, Open, Save, Save As) emits an event
    // that we handle here to call the appropriate store method.
    (async () => {
      unlistens.push(await listen('menu-new-film', async () => {
        if (!(await documentStore.confirmIfDirty())) return;
        await documentStore.newDocument();
      }));

      unlistens.push(await listen('menu-new-series', async () => {
        if (!(await documentStore.confirmIfDirty())) return;
        showSeriesDialog = true;
      }));

      unlistens.push(await listen('menu-open', async () => {
        if (!(await documentStore.confirmIfDirty())) return;
        const path = await open({
          multiple: false,
          filters: [{ name: 'Screenplay', extensions: ['screenplay'] }]
        });
        if (typeof path === 'string') {
          await documentStore.openDocument(path);
          if (documentStore.currentPath) pushRecentFile(documentStore.currentPath);
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

      unlistens.push(await listen('menu-check-updates', async () => {
        const result = await updateStore.check();
        if (result === 'current') {
          await message("You're on the latest version of Scriptty.", {
            title: 'No updates available',
            kind: 'info'
          });
        } else if (result === 'error') {
          await message('Could not reach the update server. Check your internet connection and try again.', {
            title: 'Update check failed',
            kind: 'warning'
          });
        }
      }));

      unlistens.push(await listen('menu-statistics', () => {
        showStatistics = true;
      }));

      unlistens.push(await listen('menu-scene-cards', () => {
        activeView = activeView === 'cards' ? 'writing' : 'cards';
      }));

      unlistens.push(await listen('menu-story-mode', () => {
        activeView = activeView === 'story' ? 'writing' : 'story';
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
        if (activeView === 'writing') panelOpen = !panelOpen;
      }));

      unlistens.push(await listen('menu-edit-meta', () => {
        showMetadata = true;
      }));

      unlistens.push(await listen('menu-quit', async () => {
        if (!(await documentStore.confirmIfDirty())) return;
        quitConfirmed = true;
        await getCurrentWindow().close();
      }));

      // The Rust export pipeline emits `font-fallback` when a document's
      // font slug isn't in the bundled set — we surface a native dialog so
      // the user knows the exported PDF won't match what they expected
      // instead of shipping a silently different-looking file (issue #50).
      unlistens.push(await listen<{ requested: string; fallback: string }>('font-fallback', (event) => {
        const { requested, fallback } = event.payload;
        message(
          `The font "${requested}" isn't bundled with this version of Scriptty, so "${fallback}" was used for the export instead. Pick a bundled font in Settings if the PDF should match your document.`,
          { title: 'Font not available', kind: 'warning' }
        ).catch(() => {});
      }));

      // Handle file open when app is already running (warm launch).
      // The deep-link plugin calls this when macOS sends an Apple Event
      // for opening a .screenplay file while the app is in the foreground.
      try {
        unlistens.push(await onOpenUrl(async (urls) => {
          for (const url of urls) {
            const filePath = url.startsWith('file://') ? decodeURIComponent(url.replace('file://', '')) : url;
            if (filePath.endsWith('.screenplay')) {
              if (!(await documentStore.confirmIfDirty())) return;
              await documentStore.openDocument(filePath);
              break;
            }
          }
        }));
      } catch {
        // Plugin may not be available in dev mode — ignore
      }

      // Intercept window close to prompt for unsaved changes
      unlistens.push(await getCurrentWindow().onCloseRequested(async (event) => {
        if (quitConfirmed) return; // Already confirmed via menu-quit
        if (!(await documentStore.confirmIfDirty())) {
          event.preventDefault();
        }
      }));
    })();

    return () => {
      clearTimeout(updateCheckTimer);
      window.removeEventListener('keydown', handleGlobalKeydown);
      unlistens.forEach((fn) => fn());
    };
  });
</script>

{#if !documentStore.document}
  <WelcomeScreen onOpen={handleOpenFromWelcome} />
{:else}
<main>
  <TitleBar
    onToggleSidebar={() => { panelOpen = !panelOpen; }}
    {activeView}
    onViewChange={(v) => { activeView = v; }}
    onShowExport={() => { showExport = true; }}
  />
  <div class="workspace">
    {#if activeView === 'cards'}
      <SceneCardsView />
    {/if}
    {#if activeView === 'story'}
      <StoryModeView />
    {/if}
    <div class="editor-area" class:hidden={activeView !== 'writing'}>
      <LeftPanel isOpen={panelOpen} />
      <Editor bind:findReplaceOpen bind:findReplaceMode {showAnnotations} isActive={activeView === 'writing'} bind:this={editorRef} />
    </div>
  </div>
  {#if showOutlinePeek && activeView === 'writing'}
    <OutlinePeek />
  {/if}
  <StatusBar
    onOpenPalette={() => { showCommandPalette = true; }}
    onOpenSettings={() => { showSettings = true; }}
    onShowHelp={() => { showHelp = true; }}
  >
    {#snippet rightContent()}
      {#if activeView === 'writing'}
        <span class="status-info">{editorStore.currentElement}</span>
      {:else if activeView === 'story'}
        <span class="status-info">{storyWordCount()} words</span>
      {/if}
    {/snippet}
  </StatusBar>
</main>
{/if}

<AboutModal bind:open={showAbout} />
<HelpModal bind:open={showHelp} onShowAbout={() => { showAbout = true; }} />
<StatisticsModal bind:open={showStatistics} />
<SettingsModal bind:open={showSettings} bind:showAnnotations />
<ExportModal bind:open={showExport} onEditMetadata={() => { showMetadata = true; }} />
<!-- MetadataModal renders last so it stacks above ExportModal when the
     "Edit metadata" link inside the export flow opens it. -->
<MetadataModal bind:open={showMetadata} />
<CommandPalette bind:open={showCommandPalette} {commands} />
<UpdateToast />
<SeriesTitleDialog bind:open={showSeriesDialog} onConfirm={handleCreateSeriesFromDialog} />

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
    position: relative;
    display: flex;
    flex-direction: row;
    flex: 1;
    overflow: hidden;
  }

  .editor-area.hidden {
    display: none;
  }

  .status-info {
    color: var(--text-secondary);
    text-transform: uppercase;
    font-size: 12px;
    letter-spacing: 0.03em;
  }
</style>
