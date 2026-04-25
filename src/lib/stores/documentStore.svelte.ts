// Svelte 5 reactive store for the currently open screenplay document.
// Uses $state runes for reactivity — this requires a .svelte.ts file extension.

import { invoke } from '@tauri-apps/api/core';
import { save, message } from '@tauri-apps/plugin-dialog';
import type { MessageDialogResult } from '@tauri-apps/plugin-dialog';

/** TypeScript interface matching the Rust ScreenplayDocument struct */
export interface ScreenplayMeta {
  title: string;
  author: string;
  director: string;
  /** One-line tagline / logline rendered below the title on the title page. */
  tagline: string;
  /** Registration / copyright identifier shown alongside contact info. */
  registration_number: string;
  /** Short footnote printed at the bottom of the title page — typically
   *  a confidentiality line, a "based on" credit, or a dedication. */
  footnote: string;
  contact: string;
  draft_number: number;
  draft_date: string;
  created_at: string;
  updated_at: string;
}

export interface ScreenplaySettings {
  font: string;
  default_language: string;
  input_scheme: string;
  scene_number_start: number;
  show_characters_below_header: boolean;
  /** Editor font size in pixels — clamped to 10..=18 by the backend.
   *  PDF output is unaffected (PDFs use their own fixed sizes). (#123) */
  editor_font_size: number;
}

export interface ScreenplayStory {
  idea: string;
  synopsis: string;
  treatment: string;
  narrative: string;
}

export interface SceneCard {
  /** 0-based pointer into the flat ordered list of scene_heading nodes in
   *  `content` — `scene_index: 0` is the first scene in document order.
   *
   *  Not a stable ID: reordering or deleting scenes rewrites every card's
   *  `scene_index` to stay aligned (see SceneCardsView drag/delete).
   *
   *  Series: `buildSeriesExportDocument` flattens episode cards into a
   *  single list by offsetting each episode's `scene_index` by the number
   *  of scene_headings in earlier episodes, so the backend always sees a
   *  flat index against the whole exported document. */
  scene_index: number;
  description: string;
  shoot_notes: string;
  /** Comma-separated characters present in the scene but with no dialogue.
   *  Merged with auto-detected speakers when rendering the characters line. */
  extra_characters: string;
}

/** One episode in a Series project — mirrors the film-level shape so every
 *  editor feature (navigator, export, scene cards, story) can run against it
 *  without branching on project type. */
export interface Episode {
  id: string;
  number: number;
  title: string;
  content: unknown;
  meta: ScreenplayMeta;
  settings: ScreenplaySettings;
  story: ScreenplayStory;
  scene_cards: SceneCard[];
}

export interface SeriesData {
  title: string;
  episodes: Episode[];
}

/** `"film"` means the top-level meta/settings/story/content/scene_cards are
 *  authoritative (the original shape). `"series"` means those are ignored and
 *  real data lives inside `series.episodes[]`. Old files lack this field and
 *  are treated as films for full backward compatibility. */
export type ProjectType = 'film' | 'series';

export interface ScreenplayDocument {
  type?: ProjectType;
  series?: SeriesData | null;
  content: unknown;
  meta: ScreenplayMeta;
  settings: ScreenplaySettings;
  story: ScreenplayStory;
  scene_cards: SceneCard[];
}

/** Returned by the `load_autosave` Tauri command when an autosave sidecar
 *  is newer than the file the user just opened (#118). The frontend
 *  prompts the writer with Restore / Discard before applying. */
export interface AutosaveInfo {
  document: ScreenplayDocument;
  autosave_time_ms: number;
  original_time_ms: number;
}

/** Convert a ProseMirror-ish content payload into canonical
 *  `{type:'doc', content:[...]}` shape. Accepts three inputs:
 *    - an already-canonical doc (returned unchanged),
 *    - a flat `[{type, text}, ...]` array (wrapped into a doc with each node's
 *      text promoted to an inline `{type:'text', text}` child),
 *    - a single block node (wrapped into a doc).
 *
 *  Mirrors the Editor's own `normalizeProseMirrorDoc` so readers in the store
 *  layer (Scene Navigator, Scene Cards, PDF export) see the same shape the
 *  Editor does — without pulling ProseMirror into the store module. */
function normalizeContentPayload(content: unknown): unknown {
  const emptyDoc = { type: 'doc', content: [] as unknown[] };
  if (!content) return emptyDoc;
  const wrapNode = (raw: unknown): unknown => {
    if (!raw || typeof raw !== 'object') return null;
    const node = raw as { type?: string; text?: unknown; content?: unknown };
    if (typeof node.type !== 'string') return null;
    if (Array.isArray(node.content)) return node;
    if (typeof node.text === 'string') {
      const text = node.text;
      const inline = text.length > 0 ? [{ type: 'text', text }] : [];
      return { type: node.type, content: inline };
    }
    return { type: node.type, content: [] };
  };
  if (Array.isArray(content)) {
    const children = content.map(wrapNode).filter((n): n is object => n !== null);
    return { type: 'doc', content: children };
  }
  if (typeof content === 'object') {
    const obj = content as { type?: string; content?: unknown };
    if (obj.type === 'doc') return content;
    const wrapped = wrapNode(content);
    return { type: 'doc', content: wrapped ? [wrapped] : [] };
  }
  return emptyDoc;
}

/** Reactive document store — tracks the open file, its path, and dirty state */
class DocumentStore {
  document = $state<ScreenplayDocument | null>(null);
  currentPath = $state<string | null>(null);
  isDirty = $state(false);

  /** Which episode the editor is currently showing, when the document is a
   *  Series project. Zero for film projects and ignored then. Persists
   *  across saves within a session but not across files — switching files
   *  resets to 0. */
  activeEpisodeIndex = $state(0);

  /** Timestamp (ms since epoch) of the last successful save. Null until the
   *  document has been saved at least once this session. Consumed by the
   *  status bar to render "Saved 2 min ago" relative time. */
  lastSavedAt = $state<number | null>(null);

  /** Incremented only on newDocument() and openDocument() — signals the editor
   *  to reload its ProseMirror state. Not incremented by setContent(). */
  loadTrigger = $state(0);

  /** Snapshot of the document content at load time — only updated by New/Open,
   *  never by setContent(). The editor $effect reads this instead of document.content
   *  to avoid re-triggering on every keystroke. */
  loadedContent = $state<unknown>(null);

  /** Monotonically increasing on every content change (setContent, episode
   *  switch, new/open). Consumers that only need eventually-consistent views
   *  of the document (Scene Navigator, Scene Cards, Statistics, etc.) read
   *  `contentVersionDebounced` below — bumped ~200 ms after the latest
   *  edit — to skip the per-keystroke recompute storm (#98–#101). */
  contentVersion = $state(0);

  /** Trails `contentVersion` by ~200 ms of idle. Reading this in a $derived
   *  re-runs once per "burst of typing" rather than once per keystroke.
   *  Components that need real-time updates (the editor itself) keep reading
   *  `activeContent` directly. */
  contentVersionDebounced = $state(0);

  /** Internal debounce timer for `contentVersionDebounced`. Module-level
   *  rather than $state because a setTimeout handle is not reactive data. */
  #debounceTimer: ReturnType<typeof setTimeout> | null = null;
  #DEBOUNCE_MS = 200;

  /** Bump both content versions. The debounced one trails by DEBOUNCE_MS
   *  of typing-idle; the immediate one fires synchronously for any consumer
   *  that wants tightly coupled reactivity. Episode switches and new/open
   *  flush both immediately (the writer expects an instant view refresh
   *  when changing context). */
  #bumpContentVersion(immediate: boolean = false): void {
    this.contentVersion++;
    if (immediate) {
      if (this.#debounceTimer) {
        clearTimeout(this.#debounceTimer);
        this.#debounceTimer = null;
      }
      this.contentVersionDebounced = this.contentVersion;
      return;
    }
    if (this.#debounceTimer) clearTimeout(this.#debounceTimer);
    this.#debounceTimer = setTimeout(() => {
      this.contentVersionDebounced = this.contentVersion;
      this.#debounceTimer = null;
    }, this.#DEBOUNCE_MS);
  }

  /** True when the open document is a multi-episode Series. Missing or
   *  `"film"` on the top-level `type` field both count as film (old files
   *  never wrote the field). */
  get isSeries(): boolean {
    return this.document?.type === 'series';
  }

  /** The episode currently in focus for a Series project; null for films or
   *  when no document is open. Safe to access without null-checking the
   *  whole series structure. */
  get activeEpisode(): Episode | null {
    if (!this.document || this.document.type !== 'series') return null;
    const eps = this.document.series?.episodes ?? [];
    if (eps.length === 0) return null;
    const i = Math.max(0, Math.min(this.activeEpisodeIndex, eps.length - 1));
    return eps[i];
  }

  /** Meta for the part of the document the editor is currently showing.
   *  Series → active episode's meta; Film → top-level meta. Returns null when
   *  no document is open. Consumers that edit should route through the
   *  corresponding setters (or mutate in-place; svelte-5 state reacts either
   *  way) so both shapes stay consistent. */
  get activeMeta(): ScreenplayMeta | null {
    if (!this.document) return null;
    if (this.isSeries) return this.activeEpisode?.meta ?? null;
    return this.document.meta;
  }

  get activeSettings(): ScreenplaySettings | null {
    if (!this.document) return null;
    if (this.isSeries) return this.activeEpisode?.settings ?? null;
    return this.document.settings;
  }

  get activeStory(): ScreenplayStory | null {
    if (!this.document) return null;
    if (this.isSeries) return this.activeEpisode?.story ?? null;
    return this.document.story;
  }

  get activeContent(): unknown {
    if (!this.document) return null;
    if (this.isSeries) return this.activeEpisode?.content ?? null;
    return this.document.content;
  }

  get activeSceneCards(): SceneCard[] {
    if (!this.document) return [];
    if (this.isSeries) return this.activeEpisode?.scene_cards ?? [];
    return this.document.scene_cards;
  }

  /** Replace the active scene_cards array. Needed by flows that rebuild the
   *  list after a reorder/delete rather than mutating in place. */
  setActiveSceneCards(cards: SceneCard[]): void {
    if (!this.document) return;
    if (this.isSeries) {
      const ep = this.activeEpisode;
      if (ep) ep.scene_cards = cards;
    } else {
      this.document.scene_cards = cards;
    }
  }

  /** Switch which episode is active in the editor. Bumps `loadTrigger` so
   *  the editor reloads its ProseMirror state from the new episode's content;
   *  otherwise the view would keep showing the previous episode's doc. */
  setActiveEpisode(index: number): void {
    if (!this.document || this.document.type !== 'series') return;
    const eps = this.document.series?.episodes ?? [];
    if (eps.length === 0) return;
    const clamped = Math.max(0, Math.min(index, eps.length - 1));
    if (clamped === this.activeEpisodeIndex) return;
    this.activeEpisodeIndex = clamped;
    this.loadedContent = eps[clamped].content;
    this.loadTrigger++;
    // Episode switch is a context change — refresh derived views immediately,
    // not on the typing-debounce.
    this.#bumpContentVersion(true);
  }

  /** Create a brand-new Series project with one empty episode. Title can
   *  be edited later via the Scene Navigator's inline rename. */
  async newSeries(seriesTitle: string): Promise<void> {
    try {
      const doc = await invoke<ScreenplayDocument>('new_screenplay');
      const ep = this.createEmptyEpisode(1, '');
      doc.type = 'series';
      doc.series = { title: seriesTitle, episodes: [ep] };
      this.document = doc;
      this.currentPath = null;
      this.isDirty = false;
      this.lastSavedAt = null;
      this.activeEpisodeIndex = 0;
      this.loadedContent = ep.content;
      this.loadTrigger++;
      this.#bumpContentVersion(true);
    } catch (error) {
      console.error('Failed to create new series:', error);
    }
  }

  /** Append a new empty episode to the open series and activate it. */
  async addEpisode(title: string = ''): Promise<void> {
    if (!this.document || this.document.type !== 'series' || !this.document.series) return;
    const number = this.document.series.episodes.length + 1;
    const ep = this.createEmptyEpisode(number, title);
    this.document.series.episodes.push(ep);
    this.markDirty();
    this.setActiveEpisode(this.document.series.episodes.length - 1);
  }

  /** Remove the episode at `index`. Refuses to drop the last remaining
   *  episode — a series with zero episodes isn't a meaningful state. */
  removeEpisode(index: number): void {
    if (!this.document || this.document.type !== 'series' || !this.document.series) return;
    const eps = this.document.series.episodes;
    if (eps.length <= 1) return;
    if (index < 0 || index >= eps.length) return;
    eps.splice(index, 1);
    this.renumberEpisodes();
    // Keep the active selection valid even if we removed at/before it.
    const nextIndex = Math.max(0, Math.min(this.activeEpisodeIndex, eps.length - 1));
    this.activeEpisodeIndex = -1; // force loadTrigger bump even if clamping lands on same index
    this.setActiveEpisode(nextIndex);
    this.markDirty();
  }

  /** Move an episode up or down in the list; renumbers sequentially. */
  reorderEpisode(from: number, to: number): void {
    if (!this.document || this.document.type !== 'series' || !this.document.series) return;
    const eps = this.document.series.episodes;
    if (from < 0 || from >= eps.length || to < 0 || to >= eps.length || from === to) return;
    const moved = eps.splice(from, 1)[0];
    eps.splice(to, 0, moved);
    this.renumberEpisodes();
    // If the active episode moved, keep the same Episode object active.
    if (from === this.activeEpisodeIndex) this.activeEpisodeIndex = to;
    else if (from < this.activeEpisodeIndex && to >= this.activeEpisodeIndex) this.activeEpisodeIndex--;
    else if (from > this.activeEpisodeIndex && to <= this.activeEpisodeIndex) this.activeEpisodeIndex++;
    this.markDirty();
  }

  /** Rename an episode in-place. */
  renameEpisode(index: number, title: string): void {
    const eps = this.document?.series?.episodes;
    if (!eps || index < 0 || index >= eps.length) return;
    eps[index].title = title;
    this.markDirty();
  }

  /** Rename the series as a whole. */
  renameSeries(title: string): void {
    if (!this.document?.series) return;
    this.document.series.title = title;
    this.markDirty();
  }

  private renumberEpisodes(): void {
    const eps = this.document?.series?.episodes;
    if (!eps) return;
    for (let i = 0; i < eps.length; i++) eps[i].number = i + 1;
  }

  private createEmptyEpisode(number: number, title: string): Episode {
    // Episodes inherit the document's current font, default language,
    // and input scheme so the series stays typographically coherent —
    // falling back to blank defaults only when there is no document yet
    // (e.g. creating the very first episode of a brand-new series).
    const docSettings = this.document?.settings;
    const settings = this.blankSettings();
    if (docSettings) {
      settings.font = docSettings.font;
      settings.default_language = docSettings.default_language;
      settings.input_scheme = docSettings.input_scheme;
    }
    return {
      id: typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : String(Date.now() + Math.random()),
      number,
      title,
      content: { type: 'doc', content: [{ type: 'scene_heading' }] },
      meta: this.blankMeta(),
      settings,
      story: { idea: '', synopsis: '', treatment: '', narrative: '' },
      scene_cards: [],
    };
  }

  private blankMeta(): ScreenplayMeta {
    return {
      title: '',
      author: '',
      director: '',
      tagline: '',
      registration_number: '',
      footnote: '',
      contact: '',
      draft_number: 1,
      draft_date: '',
      created_at: '',
      updated_at: '',
    };
  }

  private blankSettings(): ScreenplaySettings {
    return {
      font: 'manjari',
      default_language: 'malayalam',
      input_scheme: 'mozhi',
      scene_number_start: 1,
      show_characters_below_header: false,
      editor_font_size: 14,
    };
  }

  /** Create a new empty screenplay via the Rust backend */
  async newDocument(): Promise<void> {
    try {
      const doc = await invoke<ScreenplayDocument>('new_screenplay');
      this.document = doc;
      this.currentPath = null;
      this.isDirty = false;
      this.lastSavedAt = null;
      this.activeEpisodeIndex = 0;
      this.loadedContent = doc.content;
      this.loadTrigger++;
      this.#bumpContentVersion(true);
    } catch (error) {
      console.error('Failed to create new screenplay:', error);
    }
  }

  /** Save the current document. If path is provided, save there; otherwise use currentPath. */
  async saveDocument(path?: string): Promise<void> {
    const savePath = path ?? this.currentPath;
    if (!savePath || !this.document) return;

    try {
      await invoke('save_screenplay', { path: savePath, document: this.document });
      this.currentPath = savePath;
      this.isDirty = false;
      this.lastSavedAt = Date.now();
      // Real save committed — the autosave sidecar is now stale, drop it
      // (see #118). Cancel any pending autosave timer so we don't write
      // a fresh sidecar moments after deleting the old one.
      this.cancelPendingAutosave();
      void this.#discardAutosave(savePath);

      // If no explicit title set, derive it from the filename
      if (!this.document.meta.title) {
        const filename = savePath.split('/').pop() ?? savePath.split('\\').pop() ?? savePath;
        this.document.meta.title = filename.replace(/\.screenplay$/, '');
      }
    } catch (error) {
      console.error('Failed to save screenplay:', error);
    }
  }

  /** Open a screenplay file from disk */
  async openDocument(path: string): Promise<void> {
    try {
      let doc = await invoke<ScreenplayDocument>('open_screenplay', { path });

      // Crash-recovery (#118): if a `<path>.autosave` sidecar exists and
      // is newer than the file we just loaded, the previous session
      // crashed (or quit without saving) with unsaved work. Offer the
      // writer a choice — restore the autosave or keep the saved file.
      try {
        const autosave = await invoke<AutosaveInfo | null>('load_autosave', { path });
        if (autosave) {
          const minutesOld = Math.max(
            1,
            Math.round((autosave.autosave_time_ms - autosave.original_time_ms) / 60_000),
          );
          const result: MessageDialogResult = await message(
            `Scriptty found unsaved changes from your last session (about ${minutesOld} min newer than the saved file). Restore them?`,
            {
              title: 'Recover unsaved changes?',
              kind: 'info',
              buttons: { yes: 'Restore', no: 'Discard', cancel: 'Decide later' },
            },
          );
          if (result === 'Restore') {
            doc = autosave.document;
            // Keep the autosave file in place; the user is now editing
            // recovered content, and a save will overwrite both.
          } else if (result === 'Discard') {
            // Drop the stale sidecar so we don't keep prompting.
            void this.#discardAutosave(path);
          }
          // 'Cancel' (Decide later) — keep the autosave file untouched and
          // proceed with the saved version. We'll prompt again next open.
        }
      } catch (recoverErr) {
        // Recovery is best-effort; never block opening the document.
        console.warn('Autosave recovery check failed:', recoverErr);
      }

      // Normalize every content payload into canonical ProseMirror shape
      // before anything else reads it. Slim-format files (series authored
      // by hand, Fountain-like episode blocks) can store content as a flat
      // [{type,text},...] array — the Editor's own normalizer handles it on
      // load, but the Scene Navigator / Scene Cards read content straight
      // from the store and would otherwise find no `content.content` to walk.
      doc.content = normalizeContentPayload(doc.content);
      if (doc.type === 'series' && doc.series) {
        for (const ep of doc.series.episodes) {
          ep.content = normalizeContentPayload(ep.content);
        }
      }
      this.document = doc;
      this.currentPath = path;
      this.isDirty = false;
      this.lastSavedAt = null;
      this.activeEpisodeIndex = 0;
      // In series mode, hand the active episode's content to the editor.
      // Fall back to the top-level content for film and any malformed series.
      const loaded = doc.type === 'series' ? doc.series?.episodes?.[0]?.content ?? doc.content : doc.content;
      this.loadedContent = loaded;
      this.loadTrigger++;
      this.#bumpContentVersion(true);
    } catch (error) {
      console.error('Failed to open screenplay:', error);
    }
  }

  /** Save with dialog — if currentPath exists, saves directly; otherwise opens a save dialog. */
  async saveWithDialog(): Promise<void> {
    console.log('[saveWithDialog] called');
    if (!this.document) return;

    if (this.currentPath) {
      await this.saveDocument(this.currentPath);
    } else {
      const path = await save({
        filters: [{ name: 'Screenplay', extensions: ['screenplay'] }]
      });
      if (!path) return; // User cancelled
      await this.saveDocument(path);
    }
  }

  /** Save As — always opens a file dialog, even if the document has a current path */
  async saveAsDialog(): Promise<void> {
    if (!this.document) return;

    const defaultTitle = this.displayTitle;
    const path = await save({
      defaultPath: defaultTitle
        ? `${defaultTitle}.screenplay`
        : 'untitled.screenplay',
      filters: [{ name: 'Screenplay', extensions: ['screenplay'] }]
    });
    if (!path) return; // User cancelled
    await this.saveDocument(path);
  }

  /** Get the current font setting slug (e.g. 'noto-sans-malayalam' or 'manjari') */
  get currentFont(): string {
    return this.document?.settings.font ?? 'manjari';
  }

  /** Update the font setting and mark the document as dirty.
   *  Font is a series-wide choice, so we mirror it into every episode's
   *  settings alongside the top-level doc settings. Without the mirror,
   *  series exports (which read the first episode's settings) would keep
   *  rendering in the stale default font. */
  setFont(font: string): void {
    if (!this.document) return;
    this.document.settings.font = font;
    if (this.document.type === 'series' && this.document.series) {
      for (const ep of this.document.series.episodes) {
        ep.settings.font = font;
      }
    }
    this.isDirty = true;
  }

  /** Mark the document as having unsaved changes */
  markDirty(): void {
    this.isDirty = true;
    this.#scheduleAutosave();
  }

  // ─── Autosave (#118) ──────────────────────────────────────────────
  // The writer is the source of truth — never lose more than ~15s of
  // typing to a crash, OS sleep, or accidental close. Autosave writes
  // to `<path>.autosave` (a sidecar) on a debounced timer; the real
  // save deletes the sidecar; on next open we check if a sidecar is
  // newer than the file and offer to restore it.
  //
  // Untitled documents (no currentPath) are skipped — they have nothing
  // to autosave alongside. The writer still gets the dirty-state guard
  // on close, which is the existing protection for that case.
  #autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  #AUTOSAVE_DELAY_MS = 15_000;

  #scheduleAutosave(): void {
    if (this.#autosaveTimer) clearTimeout(this.#autosaveTimer);
    if (!this.currentPath || !this.document) return;
    this.#autosaveTimer = setTimeout(() => {
      this.#autosaveTimer = null;
      this.#runAutosave();
    }, this.#AUTOSAVE_DELAY_MS);
  }

  async #runAutosave(): Promise<void> {
    // Bail if state changed under us between scheduling and firing.
    if (!this.currentPath || !this.document || !this.isDirty) return;
    try {
      await invoke('autosave_screenplay', {
        path: this.currentPath,
        document: this.document,
      });
    } catch (error) {
      // Best-effort — log but don't surface a toast for transient I/O failure.
      console.warn('Autosave failed:', error);
    }
  }

  /** Cancel any pending autosave timer (called on close-without-save). */
  cancelPendingAutosave(): void {
    if (this.#autosaveTimer) {
      clearTimeout(this.#autosaveTimer);
      this.#autosaveTimer = null;
    }
  }

  /** Force an immediate autosave (called from beforeunload / window close
   *  so a crash mid-typing never loses the most recent edits). */
  async flushAutosave(): Promise<void> {
    this.cancelPendingAutosave();
    if (this.isDirty) await this.#runAutosave();
  }

  async #discardAutosave(path: string): Promise<void> {
    try {
      await invoke('discard_autosave', { path });
    } catch (error) {
      console.warn('Failed to discard autosave:', error);
    }
  }

  /**
   * If the document has unsaved changes, prompt the user to Save / Don't Save / Cancel.
   * Returns true if it's safe to proceed (saved or discarded), false if cancelled.
   */
  async confirmIfDirty(): Promise<boolean> {
    if (!this.isDirty) return true;

    try {
      const result: MessageDialogResult = await message(
        'You have unsaved changes. Do you want to save before continuing?',
        {
          title: 'Unsaved Changes',
          kind: 'warning',
          buttons: { yes: 'Save', no: "Don't Save", cancel: 'Cancel' },
        }
      );

      if (result === 'Cancel') return false;
      if (result === 'Save') await this.saveWithDialog();
      else {
        // "Don't Save" — the user explicitly said discard. Drop the
        // autosave sidecar so the next open doesn't offer to restore
        // changes the user just rejected (#118).
        this.cancelPendingAutosave();
        if (this.currentPath) void this.#discardAutosave(this.currentPath);
      }
      return true;
    } catch (error) {
      console.error('[confirmIfDirty] dialog error:', error);
      return true;
    }
  }

  /** Update the document's content without marking dirty.
   *  Called by the editor on every doc-changing transaction to keep the store in sync.
   *  In series mode, routes the content into the active episode so the editor and
   *  stored series tree stay in sync without having to duplicate the editor plumbing. */
  setContent(content: unknown): void {
    if (!this.document) return;
    if (this.isSeries) {
      const ep = this.activeEpisode;
      if (ep) ep.content = content;
    } else {
      this.document.content = content;
    }
    // Debounced bump — Navigator/Cards/Statistics will catch up in ~200ms.
    this.#bumpContentVersion();
  }

  /** Title used for save-dialog defaults and window-title formatting. In
   *  series mode the series title trumps the (usually blank) top-level meta
   *  title. */
  get displayTitle(): string {
    if (!this.document) return '';
    if (this.isSeries) return this.document.series?.title ?? '';
    return this.document.meta.title ?? '';
  }

  /** A film-shaped view of the current working unit, suitable for passing to
   *  the Rust export pipeline. For a film project this is the document
   *  itself; for a series it's a shallow film-shaped wrapper around the
   *  active episode, with the series title prefixed into the meta.title so
   *  the title page shows both. Returns null if no document is open. */
  get activeExportDocument(): ScreenplayDocument | null {
    const doc = this.document;
    if (!doc) return null;
    if (!this.isSeries) return doc;
    const ep = this.activeEpisode;
    if (!ep) return doc;
    const seriesTitle = doc.series?.title ?? '';
    const epTitle = ep.title.trim();
    const composedTitle = epTitle
      ? (seriesTitle ? `${seriesTitle} — Ep ${ep.number}: ${epTitle}` : `Ep ${ep.number}: ${epTitle}`)
      : (seriesTitle ? `${seriesTitle} — Ep ${ep.number}` : `Episode ${ep.number}`);
    return {
      type: 'film',
      series: null,
      content: ep.content,
      meta: { ...ep.meta, title: ep.meta.title || composedTitle },
      settings: ep.settings,
      story: ep.story,
      scene_cards: ep.scene_cards,
    };
  }
}

export const documentStore = new DocumentStore();
