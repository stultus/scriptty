// Plugin that displays an auto-generated "characters: X, Y, Z" line below
// every scene heading. Driven by a boolean stored in plugin state so the
// Editor component can toggle it on/off via a transaction meta call.
//
// Enabled state is kept in plugin state (not closure) so ProseMirror will
// re-render decorations whenever the flag flips.

import { Plugin, PluginKey } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';

/** Plugin state carries two flags:
 *   - enabled: whether to render the decoration at all
 *   - extras: per-scene-index list of user-supplied non-speaking characters,
 *             merged with auto-detected speakers at render time. Scene index is
 *             the 0-based position of the scene_heading in the document. */
export interface CharacterListState {
	enabled: boolean;
	extras: Record<number, string[]>;
}

export const characterListKey = new PluginKey<CharacterListState>('character-list');

/** Collect unique characters for each scene in document order.
 *  Speaking characters come from `character` nodes; any extras supplied by the
 *  caller are merged per scene (user-supplied order wins — they appear first). */
function collectCharactersPerScene(
	doc: import('prosemirror-model').Node,
	extras: Record<number, string[]>
): string[] {
	const perScene: string[] = [];
	let sceneIdx = -1;
	let currentSpeakers: Set<string> | null = null;

	const flush = () => {
		if (currentSpeakers === null) return;
		const extraList = extras[sceneIdx] ?? [];
		const merged: string[] = [];
		const seen = new Set<string>();
		// Extras first so they keep their author-supplied order.
		for (const name of extraList) {
			if (name.length > 0 && !seen.has(name)) {
				merged.push(name);
				seen.add(name);
			}
		}
		for (const name of currentSpeakers) {
			if (!seen.has(name)) {
				merged.push(name);
				seen.add(name);
			}
		}
		perScene.push(merged.join(', '));
	};

	doc.forEach((node) => {
		const name = node.type.name;
		if (name === 'scene_heading') {
			flush();
			sceneIdx++;
			currentSpeakers = new Set<string>();
		} else if (name === 'character' && currentSpeakers !== null) {
			const text = node.textContent.trim();
			if (text.length > 0) currentSpeakers.add(text);
		}
	});
	flush();
	return perScene;
}

export const characterListPlugin = new Plugin<CharacterListState>({
	key: characterListKey,
	state: {
		init(): CharacterListState {
			return { enabled: false, extras: {} };
		},
		apply(tr, value): CharacterListState {
			const meta = tr.getMeta(characterListKey) as Partial<CharacterListState> | undefined;
			if (!meta) return value;
			return {
				enabled: typeof meta.enabled === 'boolean' ? meta.enabled : value.enabled,
				extras: meta.extras ?? value.extras
			};
		}
	},
	props: {
		decorations(state) {
			const pluginState = characterListKey.getState(state);
			if (!pluginState || !pluginState.enabled) return DecorationSet.empty;

			const perScene = collectCharactersPerScene(state.doc, pluginState.extras);
			const decos: Decoration[] = [];
			let sceneIdx = 0;

			state.doc.forEach((node, pos) => {
				if (node.type.name !== 'scene_heading') return;
				const line = perScene[sceneIdx] ?? '';
				sceneIdx++;
				if (line.length === 0) return;

				// Place the widget just after the scene heading node's closing
				// token so it renders visually on its own line below the heading.
				const afterPos = pos + node.nodeSize;
				decos.push(
					Decoration.widget(
						afterPos,
						() => {
							const el = document.createElement('div');
							el.className = 'scene-characters-line';
							el.setAttribute('aria-hidden', 'true');
							el.setAttribute('contenteditable', 'false');

							// Label + names are separate spans so CSS can style each
							// distinctly — label is small-caps/muted, names read as prose.
							const label = document.createElement('span');
							label.className = 'scene-characters-label';
							label.textContent = 'Characters';

							const sep = document.createElement('span');
							sep.className = 'scene-characters-sep';
							sep.textContent = '·';

							const names = document.createElement('span');
							names.className = 'scene-characters-names';
							names.textContent = line;

							el.append(label, sep, names);
							return el;
						},
						{ side: -1, ignoreSelection: true }
					)
				);
			});

			return DecorationSet.create(state.doc, decos);
		}
	}
});
