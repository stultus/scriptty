// ProseMirror plugin that auto-uppercases Latin letters in scene_heading and character nodes.
// Non-Latin scripts (Malayalam, etc.) are left untouched — toUpperCase() only affects Latin/ASCII.

import { Plugin } from 'prosemirror-state';
import type { Transaction } from 'prosemirror-state';

/** Node types where Latin text should be auto-uppercased */
const UPPERCASE_TYPES = new Set(['scene_heading', 'character']);

/** Regex matching lowercase Latin letters only */
const LOWERCASE_LATIN = /[a-z]/;

/**
 * Plugin that watches for text insertions in scene_heading and character nodes,
 * and replaces any lowercase Latin letters with their uppercase equivalents.
 *
 * Uses appendTransaction so it works with typed input, paste, and IME.
 */
export const autoUppercasePlugin = new Plugin({
  appendTransaction(transactions: readonly Transaction[], _oldState, newState) {
    // Only process if any transaction changed the document
    if (!transactions.some((tr) => tr.docChanged)) return null;

    const { doc, selection } = newState;
    const $from = selection.$from;

    // Check if the cursor is inside an auto-uppercase node type
    const parentType = $from.parent.type.name;
    if (!UPPERCASE_TYPES.has(parentType)) return null;

    // Scan the current node's text content for lowercase Latin letters
    const parent = $from.parent;
    let tr = newState.tr;
    let changed = false;

    parent.forEach((child, offset) => {
      if (!child.isText || !child.text) return;

      const text = child.text;
      // Position of this text node within the document
      // $from.start() gives the start position of the parent node's content
      const basePos = $from.start() + offset;

      for (let i = 0; i < text.length; i++) {
        if (LOWERCASE_LATIN.test(text[i])) {
          const pos = basePos + i;
          tr = tr.replaceWith(pos, pos + 1, newState.schema.text(text[i].toUpperCase()));
          changed = true;
        }
      }
    });

    return changed ? tr : null;
  },
});
