// ProseMirror plugin for character name autocomplete.
//
// When the cursor is inside a `character` element and the writer has typed
// 2+ characters, a dropdown appears with matching names from the document.
// Arrow keys navigate, Enter/Tab selects + advances to Dialogue, Escape dismisses.

import { Plugin, PluginKey, TextSelection } from 'prosemirror-state';
import type { EditorState } from 'prosemirror-state';
import type { EditorView } from 'prosemirror-view';
import { screenplaySchema } from './schema';

/** Plugin key so other code can look up autocomplete state if needed */
export const autocompleteKey = new PluginKey<AutocompleteState>('characterAutocomplete');

interface AutocompleteState {
  /** Whether the dropdown is currently visible */
  active: boolean;
  /** The text the user has typed so far in the character element */
  query: string;
  /** Filtered + sorted suggestion list */
  suggestions: string[];
  /** Index of the currently highlighted suggestion (0-based) */
  selectedIndex: number;
}

/** Collect all unique character names from the document */
function collectCharacterNames(state: EditorState): string[] {
  const names = new Set<string>();
  state.doc.descendants((node) => {
    if (node.type.name === 'character' && node.textContent.trim().length > 0) {
      names.add(node.textContent.trim());
    }
  });
  return Array.from(names);
}

/**
 * Build a matchable "skeleton" of a Malayalam (or Latin) string.
 *
 * The skeleton preserves the consonant / base-letter sequence while
 * dropping vowel marks, virama, and joiners. This matters because when
 * writers type via Mozhi they progress consonant → vowel-sign whereas
 * Malayalam encodes the vowel sign AFTER the consonant — so the literal
 * intermediate text "രമ്" is not a prefix of the final name "രാമൻ"
 * (which encodes as ര, ാ, മ, ൻ). Both reduce to the same skeleton "രമൻ"
 * vs query "രമ", making prefix search behave as writers expect.
 *
 * Stripped ranges (Malayalam block, U+0D00–U+0D7F):
 *   U+0D3E–U+0D4C dependent vowel signs (matras)
 *   U+0D4D        virama (chandrakkala)
 *   U+0D57        au length mark
 *   U+0D62–U+0D63 vocalic-l vowel signs
 *   U+0D81–U+0D83 sign chars (candrabindu etc.)
 * Plus ZWJ (U+200D) and ZWNJ (U+200C) which appear in Mozhi intermediates
 * (e.g. `c` → "ക്\u200D" before the `h`) but never carry lexical weight.
 */
function skeleton(s: string): string {
  return s
    .normalize('NFC')
    .replace(/[\u0D3E-\u0D4D\u0D57\u0D62\u0D63\u0D81-\u0D83\u200C\u200D]/g, '')
    .toLowerCase();
}

interface MatchResult {
  /** Whether the name is a candidate suggestion */
  match: boolean;
  /** 0 = prefix match (strongest), 1 = substring match (weaker) */
  rank: number;
}

/**
 * Match a stored character name against the writer's in-progress query
 * using consonant-skeleton comparison. Prefix beats substring.
 */
function matchesQuery(name: string, query: string): MatchResult {
  const nname = skeleton(name);
  const nquery = skeleton(query);
  if (nquery.length === 0) return { match: false, rank: 99 };
  // Don't suggest the exact same name the writer has already fully typed
  if (nname === nquery) return { match: false, rank: 99 };
  if (nname.startsWith(nquery)) return { match: true, rank: 0 };
  if (nname.includes(nquery)) return { match: true, rank: 1 };
  return { match: false, rank: 99 };
}

/** Compute the autocomplete state from the current editor state */
function computeState(editorState: EditorState): AutocompleteState {
  const inactive: AutocompleteState = {
    active: false,
    query: '',
    suggestions: [],
    selectedIndex: 0,
  };

  const { selection } = editorState;
  const $from = selection.$from;
  const parentType = $from.parent.type.name;

  // Only activate inside character elements
  if (parentType !== 'character') return inactive;

  const query = $from.parent.textContent;

  // Need at least 2 skeletal chars to trigger. A single Malayalam
  // consonant ("ക്") skeletonizes to 1 char — too broad to suggest on.
  const querySkeleton = skeleton(query);
  if (querySkeleton.length < 2) return inactive;

  const allNames = collectCharacterNames(editorState);
  const scored = allNames
    .map((name) => ({ name, result: matchesQuery(name, query) }))
    .filter((entry) => entry.result.match);

  // Sort: prefix matches first, then substring matches; alphabetically within each rank.
  scored.sort((a, b) => {
    if (a.result.rank !== b.result.rank) return a.result.rank - b.result.rank;
    return a.name.localeCompare(b.name);
  });

  const suggestions = scored.map((entry) => entry.name);
  if (suggestions.length === 0) return inactive;

  return {
    active: true,
    query,
    suggestions,
    selectedIndex: 0,
  };
}

/**
 * Creates the autocomplete dropdown DOM element.
 * Appended to the editor's parent so it can be positioned absolutely
 * relative to the editor container.
 */
function createDropdown(): HTMLUListElement {
  const ul = document.createElement('ul');
  ul.className = 'character-autocomplete';
  ul.style.display = 'none';
  return ul;
}

/** Render the suggestion list into the dropdown element */
function renderDropdown(
  dropdown: HTMLUListElement,
  state: AutocompleteState,
  view: EditorView
): void {
  if (!state.active) {
    dropdown.style.display = 'none';
    return;
  }

  // Build list items
  dropdown.innerHTML = '';
  state.suggestions.forEach((name, i) => {
    const li = document.createElement('li');
    li.textContent = name;
    li.className = 'autocomplete-item';
    if (i === state.selectedIndex) {
      li.classList.add('selected');
    }
    // Click to select
    li.addEventListener('mousedown', (e) => {
      // Prevent the editor from losing focus
      e.preventDefault();
      acceptSuggestion(view, name);
    });
    dropdown.appendChild(li);
  });

  // Position the dropdown below the current character element
  positionDropdown(dropdown, view);
  dropdown.style.display = 'block';
}

/**
 * Position the dropdown near the cursor, flipping above if needed.
 *
 * The dropdown is a child of `.editor-container` (which has position: relative),
 * but the visible area is clipped by `.editor-scroll` (overflow-y: auto).
 * We need to check whether the dropdown fits below the cursor within the
 * scroll container's visible viewport — if not, flip it above the cursor.
 */
function positionDropdown(dropdown: HTMLUListElement, view: EditorView): void {
  const { from } = view.state.selection;
  const coords = view.coordsAtPos(from);

  const editorRect = view.dom.getBoundingClientRect();

  // The scroll container is `.editor-scroll` — the parent of `.editor-container`.
  // It defines the visible viewport for the editor content.
  const scrollContainer = view.dom.parentElement?.parentElement;
  const scrollRect = scrollContainer?.getBoundingClientRect();

  // Left position relative to the ProseMirror editor DOM
  const left = coords.left - editorRect.left;

  // Temporarily make dropdown visible but off-screen so we can measure its height
  dropdown.style.visibility = 'hidden';
  dropdown.style.display = 'block';
  const dropdownHeight = dropdown.offsetHeight;
  dropdown.style.visibility = '';

  // Gap between the cursor line and the dropdown
  const gap = 4;

  // Default: place below the cursor line
  let top = coords.bottom - editorRect.top + gap;

  // Check if the dropdown would overflow the scroll container's visible area.
  // If there isn't enough room below, flip it above the cursor.
  if (scrollRect) {
    const spaceBelow = scrollRect.bottom - coords.bottom;
    const spaceAbove = coords.top - scrollRect.top;

    if (spaceBelow < dropdownHeight + gap && spaceAbove > dropdownHeight + gap) {
      // Flip above: position the dropdown's bottom edge at the cursor's top
      top = coords.top - editorRect.top - dropdownHeight - gap;
    }
  }

  dropdown.style.left = `${left}px`;
  dropdown.style.top = `${top}px`;
}

/**
 * Accept a suggestion: replace the character element's text with the
 * selected name, then create a new Dialogue element below and move
 * the cursor into it (same as pressing Enter on a Character element).
 */
function acceptSuggestion(view: EditorView, name: string): void {
  const state = view.state;
  const $from = state.selection.$from;

  // Replace the entire content of the character node with the selected name
  const nodeStart = $from.start(); // start of the character node's content
  const nodeEnd = $from.end();     // end of the character node's content

  let tr = state.tr.replaceWith(
    nodeStart,
    nodeEnd,
    state.schema.text(name)
  );

  // Now create a Dialogue element below, mimicking Enter behavior on Character.
  // After replacing text, the character node's end position has shifted.
  // Recalculate: the character node's outer end is nodeStart - 1 (before) + node size.
  // Simpler: resolve the new position after replacement and find .after().
  const newState = view.state.apply(tr);
  const $newFrom = newState.selection.$from;
  const afterCharacter = $newFrom.after(); // position right after the character node

  const dialogueNode = screenplaySchema.nodes.dialogue.create();
  tr = tr.insert(afterCharacter, dialogueNode);
  // Move cursor inside the new dialogue node (afterCharacter + 1 for opening tag)
  tr = tr.setSelection(TextSelection.create(tr.doc, afterCharacter + 1));
  tr.scrollIntoView();

  view.dispatch(tr);
  view.focus();
}

/**
 * The character autocomplete ProseMirror plugin.
 *
 * Uses a plugin view to manage the dropdown DOM element and keyboard
 * interception via handleKeyDown.
 */
export const characterAutocompletePlugin = new Plugin<AutocompleteState>({
  key: autocompleteKey,

  state: {
    init(_, state) {
      return computeState(state);
    },
    apply(_tr, _prevPluginState, _oldState, newState) {
      return computeState(newState);
    },
  },

  props: {
    // Intercept keys when the autocomplete dropdown is visible
    handleKeyDown(view, event) {
      const pluginState = autocompleteKey.getState(view.state);
      if (!pluginState?.active) return false;

      switch (event.key) {
        case 'ArrowDown': {
          event.preventDefault();
          // Move selection down, wrapping around
          const next =
            (pluginState.selectedIndex + 1) % pluginState.suggestions.length;
          updateSelectedIndex(view, next);
          return true;
        }

        case 'ArrowUp': {
          event.preventDefault();
          // Move selection up, wrapping around
          const prev =
            (pluginState.selectedIndex - 1 + pluginState.suggestions.length) %
            pluginState.suggestions.length;
          updateSelectedIndex(view, prev);
          return true;
        }

        case 'Enter':
        case 'Tab': {
          event.preventDefault();
          const name = pluginState.suggestions[pluginState.selectedIndex];
          acceptSuggestion(view, name);
          return true;
        }

        case 'Escape': {
          event.preventDefault();
          // Dismiss — the plugin state will recalculate on next update,
          // but we force-hide the dropdown immediately via the view
          const dropdown = findDropdown(view);
          if (dropdown) dropdown.style.display = 'none';
          // Move cursor to end of current text to "commit" and stop matching
          return true;
        }
      }

      return false;
    },
  },

  view(editorView) {
    const dropdown = createDropdown();
    // Append to the ProseMirror editor DOM so positioning works relative to it
    editorView.dom.parentElement?.appendChild(dropdown);

    return {
      update(view) {
        const pluginState = autocompleteKey.getState(view.state);
        if (pluginState) {
          renderDropdown(dropdown, pluginState, view);
        }
      },
      destroy() {
        dropdown.remove();
      },
    };
  },
});

/** Find the dropdown element for a given editor view */
function findDropdown(view: EditorView): HTMLUListElement | null {
  return view.dom.parentElement?.querySelector('.character-autocomplete') ?? null;
}

/**
 * Update the selected index in the dropdown without going through
 * ProseMirror's state cycle. We directly re-render the dropdown
 * because the selection index is a UI concern, not document state.
 */
function updateSelectedIndex(view: EditorView, newIndex: number): void {
  const dropdown = findDropdown(view);
  if (!dropdown) return;

  const pluginState = autocompleteKey.getState(view.state);
  if (!pluginState) return;

  // Update the visual selection
  const items = dropdown.querySelectorAll('.autocomplete-item');
  items.forEach((item, i) => {
    item.classList.toggle('selected', i === newIndex);
  });

  // Store the new index — we mutate the plugin state object directly here.
  // This is safe because the state will be fully recomputed on the next
  // transaction anyway.
  pluginState.selectedIndex = newIndex;
}
