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
//   character       — all-caps, no trailing punctuation, ≤ 60 chars
//                     (followed by dialogue / parenthetical until blank)
//   parenthetical   — wholly wrapped in parentheses, only valid right
//                     after a character or another parenthetical
//   dialogue        — any non-blank line right after a character or
//                     parenthetical
//   action          — anything else
//
// Blank lines reset the "in dialogue block" state. Multiple consecutive
// non-blank action lines are joined with a hard line break inside one
// action node so paragraph shape survives the round-trip.

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

function looksLikeCharacterName(line: string): boolean {
  const t = line.trim();
  if (t.length === 0 || t.length > 60) return false;
  if (!isAllCaps(t)) return false;
  // Character names typically don't end with punctuation other than ).
  // "JOHN (V.O.)" and "JANE (CONT'D)" are common forms.
  return /^[A-Z0-9][A-Z0-9 .'()/,&-]*$/.test(t);
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

  for (const raw of lines) {
    const line = raw.trim();

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

    if (looksLikeCharacterName(line)) {
      flushAction();
      nodes.push(block('character', line));
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
