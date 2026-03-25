// ProseMirror screenplay element schema: SceneHeading, Action, Character, Parenthetical, Dialogue, Transition

import { Schema, type NodeSpec, type MarkSpec } from 'prosemirror-model';

/**
 * Union type representing the six screenplay element types.
 * Used throughout the app to identify which kind of block a node is.
 */
export type ScreenplayNodeType =
	| 'scene_heading'
	| 'action'
	| 'character'
	| 'parenthetical'
	| 'dialogue'
	| 'transition';

/**
 * Node specifications for every node type in the screenplay schema.
 *
 * Each screenplay element is a block-level <p> tag distinguished by a
 * `data-type` attribute so ProseMirror can round-trip them through the DOM.
 * The `text` node is ProseMirror's built-in inline text node.
 */
const nodes: Record<string, NodeSpec> = {
	doc: {
		// The top-level document must contain one or more block nodes.
		content: 'block+'
	},

	scene_heading: {
		group: 'block',
		content: 'inline*',
		toDOM() {
			return ['p', { class: 'scene-heading', 'data-type': 'scene_heading' }, 0];
		},
		parseDOM: [{ tag: 'p[data-type="scene_heading"]' }]
	},

	action: {
		group: 'block',
		content: 'inline*',
		toDOM() {
			return ['p', { class: 'action', 'data-type': 'action' }, 0];
		},
		parseDOM: [{ tag: 'p[data-type="action"]' }]
	},

	character: {
		group: 'block',
		content: 'inline*',
		toDOM() {
			return ['p', { class: 'character', 'data-type': 'character' }, 0];
		},
		parseDOM: [{ tag: 'p[data-type="character"]' }]
	},

	parenthetical: {
		group: 'block',
		content: 'inline*',
		toDOM() {
			return ['p', { class: 'parenthetical', 'data-type': 'parenthetical' }, 0];
		},
		parseDOM: [{ tag: 'p[data-type="parenthetical"]' }]
	},

	dialogue: {
		group: 'block',
		content: 'inline*',
		toDOM() {
			return ['p', { class: 'dialogue', 'data-type': 'dialogue' }, 0];
		},
		parseDOM: [{ tag: 'p[data-type="dialogue"]' }]
	},

	transition: {
		group: 'block',
		content: 'inline*',
		toDOM() {
			return ['p', { class: 'transition', 'data-type': 'transition' }, 0];
		},
		parseDOM: [{ tag: 'p[data-type="transition"]' }]
	},

	text: {
		group: 'inline'
	}
};

/**
 * Mark specifications for inline formatting.
 *
 * A "mark" in ProseMirror is inline formatting applied to a range of text
 * (like bold). Marks are stored on text nodes as an array, e.g.:
 * { "type": "text", "text": "hello", "marks": [{ "type": "bold" }] }
 */
const marks: Record<string, MarkSpec> = {
	bold: {
		// Render bold text as a <strong> tag in the DOM
		toDOM() {
			return ['strong', 0];
		},
		// Parse <strong> and <b> tags back into the bold mark
		parseDOM: [
			{ tag: 'strong' },
			{ tag: 'b' },
			{ style: 'font-weight=bold' },
			{ style: 'font-weight=700' }
		]
	},

	italic: {
		// Render italic text as an <em> tag in the DOM
		toDOM() {
			return ['em', 0];
		},
		// Parse <em> and <i> tags back into the italic mark
		parseDOM: [
			{ tag: 'em' },
			{ tag: 'i' },
			{ style: 'font-style=italic' }
		]
	},

	underline: {
		// Render underlined text as a <u> tag in the DOM
		toDOM() {
			return ['u', 0];
		},
		// Parse <u> tags and text-decoration style back into the underline mark
		parseDOM: [
			{ tag: 'u' },
			{ style: 'text-decoration=underline' }
		]
	}
};

/**
 * The screenplay ProseMirror schema.
 * Supports bold, italic, and underline inline formatting via marks.
 */
export const screenplaySchema = new Schema({ nodes, marks });
