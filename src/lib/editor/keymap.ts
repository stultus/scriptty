// Screenplay element navigation keymap
//
// Shortcut reference:
//   Enter         → create next element (scene_heading→action, character→dialogue, etc.)
//   Shift+Enter   → create new scene_heading below (universal "new scene" shortcut)
//   Tab           → cycle element type: action→character, dialogue→parenthetical, parenthetical→character, character→action
//   Shift+Tab     → revert to action (from character/dialogue), or action→scene_heading at cursor pos 0
//   Mod+1..4       → convert current block to scene_heading / action / character / dialogue
//   Mod+Alt+1..4   → same, alternative chord for Linux DEs where Super/Ctrl+1..4 is reserved
//   Shift+Mod+T   → convert current element to transition
//   Mod+Z         → undo
//   Shift+Mod+Z   → redo

import { keymap } from 'prosemirror-keymap';
import { type Command, TextSelection, type EditorState } from 'prosemirror-state';
import type { NodeType } from 'prosemirror-model';
import { undo, redo } from 'prosemirror-history';
import { toggleMark } from 'prosemirror-commands';
import { screenplaySchema } from './schema';

/**
 * Returns the name of the node type at the current cursor position.
 * Looks at the parent node of the selection's anchor to find the block-level element.
 */
function currentNodeTypeName(state: EditorState): string {
	// $from is the resolved position at the start of the selection.
	// .parent gives us the block node that contains the cursor.
	return state.selection.$from.parent.type.name;
}

/**
 * Returns the resolved position's depth-based start position of the current block,
 * and the node itself.
 */
function currentBlockRange(state: EditorState): { from: number; to: number } {
	const $from = state.selection.$from;
	// $from.before() gives the position right before the current block node
	// $from.after() gives the position right after the current block node
	const from = $from.before();
	const to = $from.after();
	return { from, to };
}

/**
 * Enter key handler: creates a new block of the appropriate type below the current node
 * and moves the cursor into it.
 */
const handleEnter: Command = (state, dispatch) => {
	const typeName = currentNodeTypeName(state);
	const currentText = state.selection.$from.parent.textContent;

	const isEmpty = currentText.length === 0;

	// Empty Character → convert to Action in-place (exit dialogue loop)
	if (isEmpty && typeName === 'character') {
		if (dispatch) {
			const pos = state.selection.$from.before();
			let tr = state.tr.setNodeMarkup(pos, screenplaySchema.nodes.action);
			tr.scrollIntoView();
			dispatch(tr);
		}
		return true;
	}

	// Empty Dialogue → delete this empty dialogue and move cursor back
	// to the end of the Character element above. Without dialogue text,
	// the character element above has no purpose on its own yet — let
	// the writer continue editing it.
	if (isEmpty && typeName === 'dialogue') {
		if (dispatch) {
			const { from, to } = currentBlockRange(state);
			let tr = state.tr.delete(from, to);
			// Place cursor at end of the previous node (the character above)
			tr = tr.setSelection(TextSelection.create(tr.doc, from - 1));
			tr.scrollIntoView();
			dispatch(tr);
		}
		return true;
	}

	// For Action elements with the cursor in the middle of text, split the
	// paragraph at the cursor position — standard text editor behavior.
	// This lets writers break a long action paragraph into two.
	if (typeName === 'action') {
		const $from = state.selection.$from;
		const isAtEnd = $from.parentOffset === $from.parent.content.size;

		if (!isAtEnd && $from.parent.content.size > 0) {
			// Split the node at cursor position into two Action elements
			if (dispatch) {
				const tr = state.tr.split(state.selection.from);
				tr.scrollIntoView();
				dispatch(tr);
			}
			return true;
		}
	}

	// Map from current element type to the type that Enter should create
	const enterTargets: Record<string, NodeType | undefined> = {
		scene_heading: screenplaySchema.nodes.action,
		action: screenplaySchema.nodes.action,
		character: screenplaySchema.nodes.dialogue,
		dialogue: screenplaySchema.nodes.character,
		parenthetical: screenplaySchema.nodes.dialogue,
		// Transitions drop back into Action so mid-scene transitions (SMASH CUT TO:,
		// MATCH CUT:, montage beats) don't force a new scene. For the classic
		// scene-ending "CUT TO: / INT. NEXT LOCATION" flow, Shift+Enter still
		// creates a fresh scene_heading from anywhere.
		transition: screenplaySchema.nodes.action
	};

	const targetType = enterTargets[typeName];
	if (!targetType) {
		// Unknown node type — let default behavior handle it
		return false;
	}

	if (dispatch) {
		const { to } = currentBlockRange(state);
		// Create an empty node of the target type
		const newNode = targetType.create();
		// Insert the new node right after the current block
		let tr = state.tr.insert(to, newNode);
		// Position the cursor inside the newly created empty node.
		// After insertion, the new node starts at `to` and its content starts at `to + 1`
		// (because the node's opening tag occupies position `to`).
		tr = tr.setSelection(TextSelection.create(tr.doc, to + 1));
		tr.scrollIntoView();
		dispatch(tr);
	}

	return true;
};

/**
 * Tab key handler: changes the current node's type in-place (no new node created).
 */
const handleTab: Command = (state, dispatch) => {
	const typeName = currentNodeTypeName(state);

	// Map from current element type to what Tab should change it to.
	// dialogue → parenthetical → character forms a natural cycle within a dialogue block.
	const tabTargets: Record<string, NodeType | undefined> = {
		action: screenplaySchema.nodes.character,
		dialogue: screenplaySchema.nodes.parenthetical,
		parenthetical: screenplaySchema.nodes.character,
		character: screenplaySchema.nodes.action
	};

	const targetType = tabTargets[typeName];
	if (!targetType) {
		// scene_heading and others — do nothing, let default behavior through
		return false;
	}

	if (dispatch) {
		const $from = state.selection.$from;
		// $from.before() is the position of the current block node in the document.
		// setNodeMarkup changes the node's type without altering its content.
		const pos = $from.before();
		const tr = state.tr.setNodeMarkup(pos, targetType);
		tr.scrollIntoView();
		dispatch(tr);
	}

	return true;
};

/**
 * Shift+Enter handler: universal "new scene" shortcut.
 * Creates a new empty scene_heading below the current node from anywhere.
 */
const handleShiftEnter: Command = (state, dispatch) => {
	if (dispatch) {
		const { to } = currentBlockRange(state);
		const newNode = screenplaySchema.nodes.scene_heading.create();
		let tr = state.tr.insert(to, newNode);
		tr = tr.setSelection(TextSelection.create(tr.doc, to + 1));
		tr.scrollIntoView();
		dispatch(tr);
	}
	return true;
};

/**
 * Shift-Tab key handler:
 *  - character or dialogue → convert to action
 *  - action at cursor offset 0 → convert to scene_heading
 *  - everything else → pass through
 */
const handleShiftTab: Command = (state, dispatch) => {
	const typeName = currentNodeTypeName(state);

	if (typeName === 'character' || typeName === 'dialogue') {
		if (dispatch) {
			const $from = state.selection.$from;
			const pos = $from.before();
			const tr = state.tr.setNodeMarkup(pos, screenplaySchema.nodes.action);
			tr.scrollIntoView();
			dispatch(tr);
		}
		return true;
	}

	// Parenthetical → Shift+Tab → revert to dialogue (its natural parent element)
	if (typeName === 'parenthetical') {
		if (dispatch) {
			const $from = state.selection.$from;
			const pos = $from.before();
			const tr = state.tr.setNodeMarkup(pos, screenplaySchema.nodes.dialogue);
			tr.scrollIntoView();
			dispatch(tr);
		}
		return true;
	}

	// Action at cursor offset 0 → convert to scene_heading
	if (typeName === 'action') {
		const $from = state.selection.$from;
		// parentOffset is the cursor position within the parent node (0 = very start)
		if ($from.parentOffset === 0) {
			if (dispatch) {
				const pos = $from.before();
				const tr = state.tr.setNodeMarkup(pos, screenplaySchema.nodes.scene_heading);
				tr.scrollIntoView();
				dispatch(tr);
			}
			return true;
		}
	}

	return false;
};

/**
 * Backspace handler: refuse to delete a scene_heading.
 * Returning true when the cursor is at the start of a scene_heading
 * swallows the keystroke so ProseMirror's default Backspace can't
 * merge the scene into the previous block or remove its node.
 * Anywhere else inside a scene_heading we let the default run so
 * users can still erase text within the heading itself.
 */
const handleBackspace: Command = (state) => {
	const $from = state.selection.$from;
	// Only intercept collapsed selections — otherwise let range-delete proceed
	if (!state.selection.empty) return false;
	if (currentNodeTypeName(state) !== 'scene_heading') return false;
	// parentOffset === 0 means the cursor sits at the very start of the heading
	if ($from.parentOffset === 0) return true;
	return false;
};

/**
 * Mod+T handler: converts the current element to a transition.
 * Transitions are rare, so they get a dedicated shortcut rather than a Tab cycle slot.
 */
const handleModT: Command = (state, dispatch) => {
	if (dispatch) {
		const $from = state.selection.$from;
		const pos = $from.before();
		const tr = state.tr.setNodeMarkup(pos, screenplaySchema.nodes.transition);
		tr.scrollIntoView();
		dispatch(tr);
	}
	return true;
};

/**
 * Factory for Mod+1..Mod+4 element-type shortcuts. Converts the current
 * block to the target type in place, matching how Tab/Shift-Tab work.
 * Returning a Command (not invoking it) keeps each shortcut bindable
 * individually in the keymap below.
 */
function convertCurrentBlockTo(typeName: keyof typeof screenplaySchema.nodes): Command {
	return (state, dispatch) => {
		const target = screenplaySchema.nodes[typeName];
		if (!target) return false;
		// Skip the work (but still consume the keystroke) when already that type
		if (currentNodeTypeName(state) === typeName) return true;
		if (dispatch) {
			const pos = state.selection.$from.before();
			const tr = state.tr.setNodeMarkup(pos, target);
			tr.scrollIntoView();
			dispatch(tr);
		}
		return true;
	};
}

/**
 * The screenplay keymap plugin.
 * Binds Enter, Tab, Shift-Tab, and Mod-T to screenplay-specific navigation commands.
 */
export const screenplayKeymap = keymap({
	Enter: handleEnter,
	'Shift-Enter': handleShiftEnter,
	Tab: handleTab,
	'Shift-Tab': handleShiftTab,
	Backspace: handleBackspace,
	'Shift-Mod-t': handleModT,
	// Mod+1..4 on macOS, Mod+Alt+1..4 as a cross-platform alternative —
	// many Linux DEs (GNOME, KDE) reserve Super/Ctrl+1..4 for workspace
	// switching and swallow the chord before ProseMirror sees it.
	'Mod-1': convertCurrentBlockTo('scene_heading'),
	'Mod-2': convertCurrentBlockTo('action'),
	'Mod-3': convertCurrentBlockTo('character'),
	'Mod-4': convertCurrentBlockTo('dialogue'),
	'Mod-Alt-1': convertCurrentBlockTo('scene_heading'),
	'Mod-Alt-2': convertCurrentBlockTo('action'),
	'Mod-Alt-3': convertCurrentBlockTo('character'),
	'Mod-Alt-4': convertCurrentBlockTo('dialogue'),
	'Mod-z': undo,
	'Shift-Mod-z': redo,
	'Mod-b': toggleMark(screenplaySchema.marks.bold),
	'Mod-i': toggleMark(screenplaySchema.marks.italic),
	'Mod-u': toggleMark(screenplaySchema.marks.underline)
});
