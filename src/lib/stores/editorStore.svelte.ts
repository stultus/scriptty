// Shared EditorView instance so other components (like SceneNavigator) can
// interact with the ProseMirror editor.

import type { EditorView } from 'prosemirror-view';

class EditorStore {
  view = $state<EditorView | null>(null);
}

export const editorStore = new EditorStore();
