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
}

export interface ScreenplayStory {
  idea: string;
  synopsis: string;
  treatment: string;
  narrative: string;
}

export interface SceneCard {
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
    return {
      id: typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : String(Date.now() + Math.random()),
      number,
      title,
      content: { type: 'doc', content: [{ type: 'scene_heading' }] },
      meta: this.blankMeta(),
      settings: this.blankSettings(),
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
      const doc = await invoke<ScreenplayDocument>('open_screenplay', { path });
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

  /** Update the font setting and mark the document as dirty */
  setFont(font: string): void {
    if (this.document) {
      this.document.settings.font = font;
      this.isDirty = true;
    }
  }

  /** Mark the document as having unsaved changes */
  markDirty(): void {
    this.isDirty = true;
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
      // "Don't Save" — proceed without saving
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
  }

  /** Title used for save-dialog defaults and window-title formatting. In
   *  series mode the series title trumps the (usually blank) top-level meta
   *  title. */
  get displayTitle(): string {
    if (!this.document) return '';
    if (this.isSeries) return this.document.series?.title ?? '';
    return this.document.meta.title ?? '';
  }
}

export const documentStore = new DocumentStore();
