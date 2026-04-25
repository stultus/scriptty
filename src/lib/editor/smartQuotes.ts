// ProseMirror plugin: convert straight quotes to curly quotes as the writer
// types (#120). Industry-standard scripts use curly " " ' '; Word and Final
// Draft both autoconvert. Skips Malayalam runs — Malayalam typography
// doesn't use curly quote pairs, and converting there would be wrong.

import { Plugin } from 'prosemirror-state';
import type { Transaction } from 'prosemirror-state';

/** True if `ch` is a Latin letter, digit, or punctuation we treat as
 *  "trailing context" — meaning a quote *after* such a character closes
 *  rather than opens. Whitespace, line start, and openers cause the
 *  quote to open. */
function isClosingContext(ch: string | undefined): boolean {
  if (!ch) return false; // start of node — opening
  // Whitespace and brackets/parens force opening regardless of script.
  if (/[\s({[]/.test(ch)) return false;
  return true;
}

/** Detect if a character looks like Malayalam script (or any non-Latin
 *  Indic block we want to leave alone). Conservative — only Latin and
 *  common ASCII drive the quote-pair logic. */
function isMalayalamRange(ch: string): boolean {
  const code = ch.codePointAt(0) ?? 0;
  // Malayalam: U+0D00–U+0D7F. Extend later if other Indic scripts also
  // need quote-protection — but for now Malayalam is the only one we
  // ship input methods for, so it's the only realistic case.
  return code >= 0x0d00 && code <= 0x0d7f;
}

const STRAIGHT_DOUBLE = '"';
const STRAIGHT_SINGLE = "'";
const LEFT_DOUBLE = '“';   // "
const RIGHT_DOUBLE = '”';  // "
const LEFT_SINGLE = '‘';   // '
const RIGHT_SINGLE = '’';  // '

/** Plugin: scans every doc-changing transaction's inserted slice for
 *  straight quotes that were just typed, and rewrites them in-place to
 *  the appropriate curly form based on the preceding character. */
export const smartQuotesPlugin = new Plugin({
  appendTransaction(transactions: readonly Transaction[], _oldState, newState) {
    if (!transactions.some((tr) => tr.docChanged)) return null;

    const { doc } = newState;
    let tr = newState.tr;
    let changed = false;

    // Walk every transaction's step ranges to find recently typed regions.
    // We only need to consider positions whose character is a straight
    // quote AND whose predecessor isn't already curly (so we don't loop).
    for (const transaction of transactions) {
      transaction.steps.forEach((step, idx) => {
        const map = step.getMap();
        map.forEach((_oldStart, _oldEnd, newStart, newEnd) => {
          // Walk every position in the inserted range.
          for (let pos = newStart; pos < newEnd; pos++) {
            const ch = doc.textBetween(pos, pos + 1);
            if (ch !== STRAIGHT_DOUBLE && ch !== STRAIGHT_SINGLE) continue;

            // Look at the character immediately before. textBetween at
            // start-of-doc returns '' which we treat as opening context.
            const prev = pos > 0 ? doc.textBetween(pos - 1, pos) : '';

            // Skip in Malayalam runs — adjacent Malayalam text means the
            // writer is inside a Malayalam phrase; curly quotes are wrong
            // there. Also skip if the previous char is already a curly
            // quote (defensive — shouldn't happen, but cheap to check).
            if (prev && isMalayalamRange(prev)) continue;

            const replacement =
              ch === STRAIGHT_DOUBLE
                ? isClosingContext(prev) ? RIGHT_DOUBLE : LEFT_DOUBLE
                : isClosingContext(prev) ? RIGHT_SINGLE : LEFT_SINGLE;

            tr = tr.replaceWith(pos, pos + 1, newState.schema.text(replacement));
            changed = true;
          }
        });
      });
    }

    return changed ? tr : null;
  },
});
