// ProseMirror plugin for Find and Replace.
//
// Provides text search with inline decorations for highlighting matches.
// The Svelte FindReplaceBar component communicates with this plugin via
// transaction metadata on the findReplaceKey PluginKey.

import { Plugin, PluginKey, type EditorState, type Transaction } from 'prosemirror-state';
import { Decoration, DecorationSet, type EditorView } from 'prosemirror-view';
import type { Node as ProseMirrorNode } from 'prosemirror-model';

/** A match position in the document */
export interface Match {
  from: number;
  to: number;
}

/** Plugin state for find/replace */
export interface FindReplaceState {
  query: string;
  caseSensitive: boolean;
  matches: Match[];
  currentIndex: number; // -1 when no matches
  decorations: DecorationSet;
}

/** Actions dispatched via transaction metadata */
type FindReplaceAction =
  | { type: 'search'; query: string; caseSensitive: boolean }
  | { type: 'next' }
  | { type: 'prev' }
  | { type: 'setCurrent'; index: number }
  | { type: 'clear' };

export const findReplaceKey = new PluginKey<FindReplaceState>('findReplace');

/** Find all occurrences of `query` in the document */
function findMatches(doc: ProseMirrorNode, query: string, caseSensitive: boolean): Match[] {
  if (!query) return [];

  const matches: Match[] = [];
  const searchQuery = caseSensitive ? query : query.toLowerCase();

  // Walk every text node in the document
  doc.descendants((node, pos) => {
    if (!node.isText || !node.text) return;

    const text = caseSensitive ? node.text : node.text.toLowerCase();
    let startIndex = 0;

    // Find all occurrences within this text node
    while (startIndex < text.length) {
      const idx = text.indexOf(searchQuery, startIndex);
      if (idx === -1) break;

      matches.push({
        from: pos + idx,
        to: pos + idx + query.length,
      });
      // Advance by 1 to find overlapping matches
      startIndex = idx + 1;
    }
  });

  return matches;
}

/** Build a DecorationSet from matches, highlighting the current one distinctly */
function buildDecorations(
  doc: ProseMirrorNode,
  matches: Match[],
  currentIndex: number
): DecorationSet {
  if (matches.length === 0) return DecorationSet.empty;

  const decorations = matches.map((m, i) => {
    const className = i === currentIndex ? 'find-match find-match-current' : 'find-match';
    return Decoration.inline(m.from, m.to, { class: className });
  });

  return DecorationSet.create(doc, decorations);
}

/** Empty plugin state */
function emptyState(doc: ProseMirrorNode): FindReplaceState {
  return {
    query: '',
    caseSensitive: false,
    matches: [],
    currentIndex: -1,
    decorations: DecorationSet.empty,
  };
}

/** The find/replace ProseMirror plugin */
export const findReplacePlugin = new Plugin<FindReplaceState>({
  key: findReplaceKey,

  state: {
    init(_, state) {
      return emptyState(state.doc);
    },

    apply(tr: Transaction, prev: FindReplaceState, _oldState: EditorState, newState: EditorState): FindReplaceState {
      const action = tr.getMeta(findReplaceKey) as FindReplaceAction | undefined;

      if (action) {
        switch (action.type) {
          case 'search': {
            const matches = findMatches(newState.doc, action.query, action.caseSensitive);
            const currentIndex = matches.length > 0 ? 0 : -1;
            return {
              query: action.query,
              caseSensitive: action.caseSensitive,
              matches,
              currentIndex,
              decorations: buildDecorations(newState.doc, matches, currentIndex),
            };
          }

          case 'next': {
            if (prev.matches.length === 0) return prev;
            const nextIndex = (prev.currentIndex + 1) % prev.matches.length;
            return {
              ...prev,
              currentIndex: nextIndex,
              decorations: buildDecorations(newState.doc, prev.matches, nextIndex),
            };
          }

          case 'prev': {
            if (prev.matches.length === 0) return prev;
            const prevIndex = (prev.currentIndex - 1 + prev.matches.length) % prev.matches.length;
            return {
              ...prev,
              currentIndex: prevIndex,
              decorations: buildDecorations(newState.doc, prev.matches, prevIndex),
            };
          }

          case 'setCurrent': {
            if (action.index < 0 || action.index >= prev.matches.length) return prev;
            return {
              ...prev,
              currentIndex: action.index,
              decorations: buildDecorations(newState.doc, prev.matches, action.index),
            };
          }

          case 'clear': {
            return emptyState(newState.doc);
          }
        }
      }

      // Document changed while a search is active. Map existing matches
      // through the transaction (instant, O(matches)) instead of re-scanning
      // the whole doc on every keystroke (was O(text-nodes) per keystroke,
      // #101). The view-level debounced rescan below catches up with any
      // newly typed text after ~200ms of idle so brand-new matches still
      // appear — just not on the keystroke that added them.
      if (tr.docChanged && prev.query) {
        const remapped: Match[] = [];
        for (const m of prev.matches) {
          const from = tr.mapping.map(m.from, 1);
          const to = tr.mapping.map(m.to, -1);
          // Match was deleted or its range collapsed — drop it.
          if (to <= from) continue;
          // Length changed (text inserted into the match) — drop too,
          // the rescan will recover it with the new shape.
          if (to - from !== prev.query.length) continue;
          remapped.push({ from, to });
        }
        const currentIndex = remapped.length === 0
          ? -1
          : Math.min(Math.max(prev.currentIndex, 0), remapped.length - 1);
        return {
          ...prev,
          matches: remapped,
          currentIndex,
          decorations: prev.decorations.map(tr.mapping, newState.doc),
        };
      }

      return prev;
    },
  },

  props: {
    decorations(state) {
      const pluginState = findReplaceKey.getState(state);
      return pluginState?.decorations ?? DecorationSet.empty;
    },
  },

  // Debounced re-scan after the user stops typing. Existing matches are
  // already remapped synchronously in apply() above, so highlights stay
  // visible during typing; this view plugin picks up newly typed-in
  // matches once the doc settles (~200ms idle) (#101).
  view() {
    let rescanTimer: ReturnType<typeof setTimeout> | null = null;
    return {
      update(view, prevState) {
        const state = findReplaceKey.getState(view.state);
        if (!state || !state.query) {
          if (rescanTimer) {
            clearTimeout(rescanTimer);
            rescanTimer = null;
          }
          return;
        }
        // Only debounce if the doc actually changed.
        if (view.state.doc === prevState.doc) return;
        if (rescanTimer) clearTimeout(rescanTimer);
        rescanTimer = setTimeout(() => {
          rescanTimer = null;
          const cur = findReplaceKey.getState(view.state);
          if (!cur || !cur.query) return;
          // Re-trigger the search action so apply() recomputes from scratch.
          // Preserve the user's current match index when possible.
          const prevIndex = cur.currentIndex;
          view.dispatch(
            view.state.tr.setMeta(findReplaceKey, {
              type: 'search',
              query: cur.query,
              caseSensitive: cur.caseSensitive,
            } as FindReplaceAction)
          );
          const after = findReplaceKey.getState(view.state);
          if (after && after.matches.length > 0 && prevIndex >= 0 && prevIndex < after.matches.length) {
            view.dispatch(
              view.state.tr.setMeta(findReplaceKey, {
                type: 'setCurrent',
                index: prevIndex,
              } as FindReplaceAction)
            );
          }
        }, 200);
      },
      destroy() {
        if (rescanTimer) clearTimeout(rescanTimer);
      },
    };
  },
});

// --- Dispatch helpers called by the FindReplaceBar component ---

/** Dispatch a search action and return the resulting plugin state */
export function dispatchSearch(
  view: EditorView,
  query: string,
  caseSensitive: boolean
): FindReplaceState | undefined {
  const tr = view.state.tr.setMeta(findReplaceKey, {
    type: 'search',
    query,
    caseSensitive,
  } as FindReplaceAction);
  view.dispatch(tr);
  return findReplaceKey.getState(view.state);
}

/** Move to the next match (wraps around) */
export function dispatchNext(view: EditorView): FindReplaceState | undefined {
  const tr = view.state.tr.setMeta(findReplaceKey, { type: 'next' } as FindReplaceAction);
  view.dispatch(tr);
  return findReplaceKey.getState(view.state);
}

/** Move to the previous match (wraps around) */
export function dispatchPrev(view: EditorView): FindReplaceState | undefined {
  const tr = view.state.tr.setMeta(findReplaceKey, { type: 'prev' } as FindReplaceAction);
  view.dispatch(tr);
  return findReplaceKey.getState(view.state);
}

/** Clear all highlights and reset state */
export function dispatchClear(view: EditorView): void {
  const tr = view.state.tr.setMeta(findReplaceKey, { type: 'clear' } as FindReplaceAction);
  view.dispatch(tr);
}

/** Replace the current match with `replacement` text and re-search */
export function replaceCurrentMatch(
  view: EditorView,
  replacement: string
): FindReplaceState | undefined {
  const pluginState = findReplaceKey.getState(view.state);
  if (!pluginState || pluginState.currentIndex < 0) return pluginState;

  const match = pluginState.matches[pluginState.currentIndex];

  // Replace the matched text
  let tr = view.state.tr.insertText(replacement, match.from, match.to);

  // Re-search after replacing — set meta on the same transaction
  tr = tr.setMeta(findReplaceKey, {
    type: 'search',
    query: pluginState.query,
    caseSensitive: pluginState.caseSensitive,
  } as FindReplaceAction);

  view.dispatch(tr);

  // After re-search, adjust currentIndex to stay at the same position
  // (the match that was at currentIndex is now gone, so the next match
  // naturally falls into the same index position)
  const newState = findReplaceKey.getState(view.state);
  if (newState && newState.matches.length > 0 && pluginState.currentIndex < newState.matches.length) {
    // Set currentIndex to keep the user at the same position
    const setCurrent = view.state.tr.setMeta(findReplaceKey, {
      type: 'setCurrent',
      index: Math.min(pluginState.currentIndex, newState.matches.length - 1),
    } as FindReplaceAction);
    view.dispatch(setCurrent);
  }

  return findReplaceKey.getState(view.state);
}

/** Replace all matches in a single undo step */
export function replaceAllMatches(
  view: EditorView,
  replacement: string
): FindReplaceState | undefined {
  const pluginState = findReplaceKey.getState(view.state);
  if (!pluginState || pluginState.matches.length === 0) return pluginState;

  // Replace in reverse order to preserve earlier positions
  let tr = view.state.tr;
  for (let i = pluginState.matches.length - 1; i >= 0; i--) {
    const match = pluginState.matches[i];
    tr = tr.insertText(replacement, match.from, match.to);
  }

  // Re-search on the same transaction (will find 0 matches since all were replaced)
  tr = tr.setMeta(findReplaceKey, {
    type: 'search',
    query: pluginState.query,
    caseSensitive: pluginState.caseSensitive,
  } as FindReplaceAction);

  view.dispatch(tr);
  return findReplaceKey.getState(view.state);
}

/** Scroll the current match into view by querying the DOM for the highlight */
export function scrollToCurrentMatch(view: EditorView): void {
  // Use requestAnimationFrame to wait for decorations to render
  requestAnimationFrame(() => {
    const el = view.dom.querySelector('.find-match-current');
    if (el) {
      el.scrollIntoView({ block: 'center', behavior: 'smooth' });
    }
  });
}
