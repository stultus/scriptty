// Shared EditorView instance so other components (like SceneNavigator) can
// interact with the ProseMirror editor.
// Also tracks which inline marks (bold, italic, underline) are active at the
// cursor position, so UI components like TitleBar can show active state.

import type { EditorView } from 'prosemirror-view';

/** Which inline marks are currently active at the cursor position */
interface MarkState {
  bold: boolean;
  italic: boolean;
  underline: boolean;
}

class EditorStore {
  view = $state<EditorView | null>(null);

  // Reactive mark state — updated by the Editor component on every selection change
  markState = $state<MarkState>({ bold: false, italic: false, underline: false });
}

export const editorStore = new EditorStore();
