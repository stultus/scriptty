// Plugin that displays an auto-generated "characters: X, Y, Z" line below
// every scene heading. Driven by a boolean stored in plugin state so the
// Editor component can toggle it on/off via a transaction meta call.
//
// Enabled state is kept in plugin state (not closure) so ProseMirror will
// re-render decorations whenever the flag flips.

import { Plugin, PluginKey } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';

/** Plugin state carries the toggle, the per-scene extras, and the
 *  decoration cache. Caching the DecorationSet here means we only walk
 *  the document when something that affects the output actually changes
 *  (doc edit, enabled flip, or extras update) — not on every transaction
 *  the editor dispatches (#102). */
export interface CharacterListState {
	enabled: boolean;
	extras: Record<number, string[]>;
	decorations: DecorationSet;
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

/** Build the DecorationSet from the document. This is the expensive walk
 *  we want to call as rarely as possible — `apply()` below decides when. */
function buildDecorations(
	doc: import('prosemirror-model').Node,
	extras: Record<number, string[]>
): DecorationSet {
	const perScene = collectCharactersPerScene(doc, extras);
	const decos: Decoration[] = [];
	let sceneIdx = 0;

	doc.forEach((node, pos) => {
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

	return DecorationSet.create(doc, decos);
}

export const characterListPlugin = new Plugin<CharacterListState>({
	key: characterListKey,
	state: {
		init(_, state): CharacterListState {
			return { enabled: false, extras: {}, decorations: DecorationSet.empty };
		},
		apply(tr, value, _oldState, newState): CharacterListState {
			const meta = tr.getMeta(characterListKey) as
				| Partial<Pick<CharacterListState, 'enabled' | 'extras'>>
				| undefined;

			const enabled = meta && typeof meta.enabled === 'boolean' ? meta.enabled : value.enabled;
			const extras = meta?.extras ?? value.extras;

			// Disabled — keep cache empty, skip the walk entirely.
			if (!enabled) {
				return value.enabled
					? { enabled: false, extras, decorations: DecorationSet.empty }
					: { enabled, extras, decorations: value.decorations };
			}

			// Enabled and nothing affecting output changed — reuse cache.
			// Must still map decorations through the transaction so positions
			// stay aligned with the new doc.
			const extrasChanged = meta?.extras !== undefined && meta.extras !== value.extras;
			const enabledChanged = enabled !== value.enabled;
			if (!tr.docChanged && !extrasChanged && !enabledChanged) {
				return value;
			}

			// Rebuild on real structural change (doc, extras, or enable flip).
			return {
				enabled,
				extras,
				decorations: buildDecorations(newState.doc, extras)
			};
		}
	},
	props: {
		decorations(state) {
			const pluginState = characterListKey.getState(state);
			return pluginState?.decorations ?? DecorationSet.empty;
		}
	}
});
