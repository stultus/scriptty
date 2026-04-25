// Plugin that tags each scene_heading node with a time-of-day class
// (`time-day` / `time-night`) so the editor's gutter scene-number
// numeral can be tinted warm/cool to match the SceneCardsView gutter
// numerals. Mirrors the time-of-day parsing the navigator uses.
//
// Implementation: Decoration.node attaches a class via DecorationAttrs
// — no DOM widget, no node attrs in the schema, and the schema /
// .screenplay JSON shape stay unchanged.

import { Plugin, PluginKey } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';

export interface SceneTimeState {
	decorations: DecorationSet;
}

export const sceneTimeKey = new PluginKey<SceneTimeState>('scene-time');

/** Pull the time-of-day class from a scene heading text. Returns
 *  `'time-day'` / `'time-night'` for tinted classes, or empty string
 *  when no recognized time word appears (so the numeral stays neutral). */
function classFor(headingText: string): string {
	const tail = headingText.split(/\s[-–—]\s|\s-\s/).pop()?.trim().toUpperCase() ?? '';
	if (/\b(NIGHT|DUSK|EVENING)\b/.test(tail)) return 'time-night';
	if (/\b(DAY|DAWN|MORNING|AFTERNOON)\b/.test(tail)) return 'time-day';
	return '';
}

function buildDecorations(doc: import('prosemirror-model').Node): DecorationSet {
	const decos: Decoration[] = [];
	doc.forEach((node, pos) => {
		if (node.type.name !== 'scene_heading') return;
		const cls = classFor(node.textContent);
		if (!cls) return;
		decos.push(Decoration.node(pos, pos + node.nodeSize, { class: cls }));
	});
	return DecorationSet.create(doc, decos);
}

export const sceneTimeOfDayPlugin = new Plugin<SceneTimeState>({
	key: sceneTimeKey,
	state: {
		init(_, state): SceneTimeState {
			return { decorations: buildDecorations(state.doc) };
		},
		apply(tr, value, _oldState, newState): SceneTimeState {
			if (!tr.docChanged) return value;
			return { decorations: buildDecorations(newState.doc) };
		}
	},
	props: {
		decorations(state) {
			return sceneTimeKey.getState(state)?.decorations ?? DecorationSet.empty;
		}
	}
});
