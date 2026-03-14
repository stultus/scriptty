// Svelte 5 reactive store for the currently open screenplay document.
// Uses $state runes for reactivity — this requires a .svelte.ts file extension.

import { invoke } from '@tauri-apps/api/core';
import { save, message } from '@tauri-apps/plugin-dialog';
import type { MessageDialogResult } from '@tauri-apps/plugin-dialog';

/** TypeScript interface matching the Rust ScreenplayDocument struct */
export interface ScreenplayMeta {
  title: string;
  author: string;
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
}

export interface ScreenplayStory {
  idea: string;
  synopsis: string;
  treatment: string;
}

export interface SceneCard {
  scene_index: number;
  description: string;
  shoot_notes: string;
}

export interface ScreenplayDocument {
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

  /** Incremented only on newDocument() and openDocument() — signals the editor
   *  to reload its ProseMirror state. Not incremented by setContent(). */
  loadTrigger = $state(0);

  /** Snapshot of the document content at load time — only updated by New/Open,
   *  never by setContent(). The editor $effect reads this instead of document.content
   *  to avoid re-triggering on every keystroke. */
  loadedContent = $state<unknown>(null);

  /** Create a new empty screenplay via the Rust backend */
  async newDocument(): Promise<void> {
    try {
      const doc = await invoke<ScreenplayDocument>('new_screenplay');
      this.document = doc;
      this.currentPath = null;
      this.isDirty = false;
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
      this.loadedContent = doc.content;
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

    const path = await save({
      defaultPath: this.document.meta.title
        ? `${this.document.meta.title}.screenplay`
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
    // 'Don\'t Save' — proceed without saving
    return true;
  }

  /** Update the document's content without marking dirty.
   *  Called by the editor on every doc-changing transaction to keep the store in sync. */
  setContent(content: unknown): void {
    if (this.document) {
      this.document.content = content;
    }
  }
}

export const documentStore = new DocumentStore();
