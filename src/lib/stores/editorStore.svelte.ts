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

  // Current element type at cursor (e.g. 'SCENE HEADING', 'ACTION')
  currentElement = $state<string>('SCENE HEADING');

  // 0-based index of the scene containing the cursor, or -1 if the cursor
  // sits before the first scene_heading. The outline peek strip reads this
  // to highlight the active segment.
  currentSceneIndex = $state<number>(-1);

}

export const editorStore = new EditorStore();
