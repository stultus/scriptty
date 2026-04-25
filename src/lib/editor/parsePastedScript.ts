// Heuristic plain-text → ProseMirror screenplay JSON parser (#122).
//
// First-launch ergonomics: writers often have draft material in plain text
// (notes app, email, OCR'd photos). Final Draft / Fountain import are
// explicitly deferred, but accepting raw text is a low-friction entry
// point. The parser is intentionally permissive — we'd rather over-detect
// scene headings and let the writer fix one false positive than miss a
// real heading and dump everything into action.
//
// Detection rules, evaluated in order on each non-blank line:
//
//   scene_heading   — starts with INT./EXT./INT/EXT or "I/E"
//   transition      — all-caps Latin and ends with ":" (e.g. "CUT TO:")
//   character       — Latin: all-caps, ≤ 60 chars, no trailing punct.
//                   — Malayalam: short Malayalam line (≤ 30 chars /
//                     ≤ 4 words), no sentence-ending punctuation, AND
//                     either ends with ":" (an explicit name marker)
//                     or is followed by a parenthetical line. Without
//                     a high-confidence signal we treat as action —
//                     short dialogues like "ശരി" make any "next is
//                     longer" rule produce false positives.
//                   — `Name: Dialogue` on one line is also detected
//                     and split into two nodes (common in plain-text
//                     Malayalam drafts).
//   parenthetical   — wholly wrapped in parentheses, only valid right
//                     after a character or another parenthetical
//   dialogue        — any non-blank line right after a character or
//                     parenthetical
//   action          — anything else
//
// Blank lines reset the "in dialogue block" state. Multiple consecutive
// non-blank action lines are joined with a hard line break inside one
// action node so paragraph shape survives the round-trip.
//
// Malayalam caveat: Malayalam has no upper/lower case, so the all-caps
// signal can't apply. The Malayalam character rule above is necessarily
// stricter (requires the lookahead). Malayalam screenplays often write
// character names in English uppercase regardless — those still match
// the Latin rule and are the reliable path.

interface PMNode {
  type: string;
  content?: PMText[];
}

interface PMText {
  type: 'text';
  text: string;
}

const SCENE_HEADING = /^(INT\.?\/EXT\.?|EXT\.?\/INT\.?|INT\.?|EXT\.?|I\/E)\b/i;

function textNode(text: string): PMText {
  return { type: 'text', text };
}

function block(type: string, text: string): PMNode {
  if (!text) return { type };
  return { type, content: [textNode(text)] };
}

/** All caps + no lowercase letters anywhere. Numbers, punctuation, and
 *  whitespace are allowed. Empty strings return false. */
function isAllCaps(line: string): boolean {
  if (!line.trim()) return false;
  // Has at least one Latin letter and no lowercase Latin letter. Malayalam
  // text has no concept of case — we deliberately don't treat Malayalam-only
  // lines as character names because that would misclassify a long action
  // sentence in Malayalam as a speaker name.
  if (!/[A-Za-z]/.test(line)) return false;
  return line === line.toUpperCase();
}

/** Latin-script character name: all-caps, no terminal punctuation,
 *  conservative shape. */
function looksLikeLatinCharacterName(line: string): boolean {
  const t = line.trim();
  if (t.length === 0 || t.length > 60) return false;
  if (!isAllCaps(t)) return false;
  // Character names typically don't end with sentence punctuation.
  // "JOHN (V.O.)" and "JANE (CONT'D)" are common forms.
  return /^[A-Z0-9][A-Z0-9 .'()/,&-]*$/.test(t);
}

/** Malayalam-script character name candidate. Returns true if the line
 *  *could* be a name based on its own shape — caller still confirms with
 *  a lookahead at the next non-blank line, since a short Malayalam phrase
 *  on its own is otherwise indistinguishable from an action sentence. */
function looksLikeMalayalamNameCandidate(line: string): boolean {
  const t = line.trim();
  if (t.length === 0 || t.length > 30) return false;
  // Has Malayalam content (at least one Malayalam codepoint).
  if (!/[ഀ-ൿ]/.test(t)) return false;
  // Reject lines with sentence-ending punctuation — character names
  // never end a sentence. Includes Malayalam danda (।) which some
  // writers use as a full stop.
  if (/[.!?,;:।]$/.test(t)) return false;
  // Reject lines with quotation marks — those are dialogue or quoted
  // content, not a speaker name.
  if (/[“”"‘’']/.test(t)) return false;
  // Conservative word cap — names are 1–4 words; longer is almost
  // certainly an action sentence.
  if (t.split(/\s+/).length > 4) return false;
  return true;
}

/** Combined check used by the parser. Latin all-caps is decisive on its
 *  own; Malayalam requires a high-confidence signal because the case
 *  trick that gives Latin its certainty doesn't exist for Malayalam.
 *
 *  Signals accepted for Malayalam:
 *    1. Line ends with ":" — `രമേശ്:` is a common plain-text convention
 *       for marking a speaker before their line of dialogue.
 *    2. Next non-blank line is a parenthetical (`(softly)`-style) — only
 *       a character is followed by a parenthetical in screenplay format.
 *
 *  We deliberately drop the earlier "next line is longer than the name"
 *  rule — short dialogues like "ശരി" easily make a normal action line
 *  pass that test, producing false positives. With these stricter
 *  signals, Malayalam name detection is conservative on purpose: missed
 *  names get fixed in the editor, but we don't sprinkle phantom
 *  "characters" through pasted prose. */
function looksLikeCharacterName(line: string, nextNonBlank: string | null): boolean {
  if (looksLikeLatinCharacterName(line)) return true;
  if (!looksLikeMalayalamNameCandidate(line)) return false;

  // Signal 1: explicit name marker (`:` at the end of the line). The
  // colon is dropped at character emission so the editor sees just the
  // name — caller handles that.
  if (line.trim().endsWith(':')) return true;

  // Signal 2: next non-blank is a parenthetical.
  if (nextNonBlank && isParenthetical(nextNonBlank)) return true;

  return false;
}

function looksLikeTransition(line: string): boolean {
  const t = line.trim();
  if (!isAllCaps(t)) return false;
  return t.endsWith(':') || /\bTO:$/i.test(t);
}

function isParenthetical(line: string): boolean {
  const t = line.trim();
  return t.startsWith('(') && t.endsWith(')') && t.length >= 2;
}

/** Detect the "Name: Dialogue" same-line shape and split it into two
 *  nodes. Conservative — only the FIRST `:` counts as the separator,
 *  the name half must be a Latin all-caps name OR a Malayalam name
 *  candidate, and the dialogue half must be non-empty. Times-of-day
 *  ("DAY:") are excluded because they'd already be inside a
 *  scene_heading line, which is matched earlier in the pipeline. */
function trySplitNameColonDialogue(line: string): { name: string; dialogue: string } | null {
  const idx = line.indexOf(':');
  if (idx < 1) return null;
  const namePart = line.slice(0, idx).trim();
  const dialoguePart = line.slice(idx + 1).trim();
  if (namePart.length === 0 || dialoguePart.length === 0) return null;
  // Don't trigger on URLs (`https://...`) or scene heading sub-parts.
  if (/[/\\]/.test(namePart)) return null;
  // Name must look like a name on its own — reuse the existing detectors.
  if (looksLikeLatinCharacterName(namePart)) {
    return { name: namePart, dialogue: dialoguePart };
  }
  if (looksLikeMalayalamNameCandidate(namePart)) {
    return { name: namePart, dialogue: dialoguePart };
  }
  return null;
}

/**
 * Parse plain text into the ProseMirror screenplay document JSON.
 * Returns the canonical `{ type: 'doc', content: [...] }` shape that
 * documentStore.normalizeContentPayload expects.
 */
export function parsePastedScript(input: string): {
  type: 'doc';
  content: PMNode[];
} {
  const nodes: PMNode[] = [];
  // Normalize line endings so split('\n') works the same on every OS.
  const lines = input.replace(/\r\n?/g, '\n').split('\n');

  // "Are we currently inside a character dialogue block?" — set true when
  // we emit a `character` node, stays true through any number of dialogue
  // and parenthetical lines, reset on a blank line or any other element.
  let inDialogueBlock = false;
  // Action lines accumulate so a paragraph of action stays as one node
  // rather than fragmenting into many. Flushed on every element transition.
  let actionBuffer: string[] = [];

  const flushAction = () => {
    if (actionBuffer.length === 0) return;
    nodes.push(block('action', actionBuffer.join(' ').trim()));
    actionBuffer = [];
  };

  /** Find the next non-blank line strictly after index `i`, trimmed.
   *  Returns null at end of input. Used by the Malayalam character
   *  detector to disambiguate "short Malayalam phrase" from "speaker
   *  name followed by dialogue". */
  const peekNextNonBlank = (i: number): string | null => {
    for (let j = i + 1; j < lines.length; j++) {
      const t = lines[j].trim();
      if (t.length > 0) return t;
    }
    return null;
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();

    if (line.length === 0) {
      flushAction();
      inDialogueBlock = false;
      continue;
    }

    if (SCENE_HEADING.test(line)) {
      flushAction();
      inDialogueBlock = false;
      // Uppercase the heading to match the editor's auto-uppercase behavior.
      nodes.push(block('scene_heading', line.toUpperCase()));
      continue;
    }

    if (looksLikeTransition(line)) {
      flushAction();
      inDialogueBlock = false;
      nodes.push(block('transition', line));
      continue;
    }

    if (inDialogueBlock && isParenthetical(line)) {
      // Keep the parens — the editor stores them in content (see #59).
      nodes.push(block('parenthetical', line));
      continue;
    }

    if (inDialogueBlock) {
      // Anything that follows a character (and isn't a parenthetical /
      // scene heading / transition) is dialogue.
      nodes.push(block('dialogue', line));
      continue;
    }

    // Same-line "Name: Dialogue" form — common in plain-text Malayalam
    // (and some English) screenplay drafts. Detected here so we emit
    // both nodes from the single source line.
    const colonSplit = trySplitNameColonDialogue(line);
    if (colonSplit) {
      flushAction();
      nodes.push(block('character', colonSplit.name));
      nodes.push(block('dialogue', colonSplit.dialogue));
      inDialogueBlock = true;
      continue;
    }

    if (looksLikeCharacterName(line, peekNextNonBlank(i))) {
      flushAction();
      // Strip the trailing colon (it was a marker, not part of the name).
      const name = line.replace(/:\s*$/, '').trim();
      nodes.push(block('character', name));
      inDialogueBlock = true;
      continue;
    }

    // Default — accumulate into the action buffer so a wrapped paragraph
    // stays one node.
    actionBuffer.push(line);
  }

  flushAction();

  // Editor schema requires at least one block. If the input parsed to
  // nothing, hand back the same minimal-doc shape new_screenplay uses.
  if (nodes.length === 0) {
    nodes.push({ type: 'scene_heading' });
  }

  return { type: 'doc', content: nodes };
}
