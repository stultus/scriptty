// Plugin that displays an auto-generated "characters: X, Y, Z" line below
// every scene heading. Driven by a boolean stored in plugin state so the
// Editor component can toggle it on/off via a transaction meta call.
//
// Enabled state is kept in plugin state (not closure) so ProseMirror will
// re-render decorations whenever the flag flips.

import { Plugin, PluginKey } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';

export const characterListKey = new PluginKey<{ enabled: boolean }>('character-list');

/** Collect unique speaking characters for each scene in document order.
 *  Returns an array parallel to scene_heading positions; each entry is the
 *  comma-separated list to show below that scene. */
function collectCharactersPerScene(doc: import('prosemirror-model').Node): string[] {
	const perScene: string[] = [];
	let currentSet: Set<string> | null = null;

	doc.forEach((node) => {
		const name = node.type.name;
		if (name === 'scene_heading') {
			// Close out the previous scene before starting the new one
			if (currentSet !== null) {
				perScene.push([...currentSet].join(', '));
			}
			currentSet = new Set<string>();
		} else if (name === 'character' && currentSet !== null) {
			const text = node.textContent.trim();
			if (text.length > 0) currentSet.add(text);
		}
	});
	// Flush the last scene
	if (currentSet !== null) {
		perScene.push([...currentSet].join(', '));
	}
	return perScene;
}

export const characterListPlugin = new Plugin<{ enabled: boolean }>({
	key: characterListKey,
	state: {
		init(): { enabled: boolean } {
			return { enabled: false };
		},
		apply(tr, value): { enabled: boolean } {
			const meta = tr.getMeta(characterListKey);
			if (meta && typeof meta.enabled === 'boolean') {
				return { enabled: meta.enabled };
			}
			return value;
		}
	},
	props: {
		decorations(state) {
			const pluginState = characterListKey.getState(state);
			if (!pluginState || !pluginState.enabled) return DecorationSet.empty;

			const perScene = collectCharactersPerScene(state.doc);
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
