// Single source of truth for input mode state (English/Malayalam toggle, active input scheme).
// This is a singleton — use InputModeManager.getInstance() to access it from anywhere.

import { processInscript1Key } from './inscript1';
import { processInscript2Key } from './inscript2';
import { MozhiEngine } from './mozhi';
import type { InputResult } from './mozhi';

// Re-export InputResult so consumers can import it from here
export type { InputResult };

/** The three supported Malayalam input schemes */
export type InputScheme = 'mozhi' | 'inscript1' | 'inscript2';

/**
 * Singleton class that manages input mode state.
 * Controls whether we're typing in English or Malayalam, and which input scheme is active.
 */
export class InputModeManager {
  // "static" means this property belongs to the class itself, not to instances.
  // We use it to store the single shared instance (singleton pattern).
  private static instance: InputModeManager;

  /** Whether Malayalam input is currently active (false = English passthrough) */
  isMalayalam: boolean = false;

  /** The currently selected Malayalam input scheme */
  scheme: InputScheme = 'mozhi';

  /** The Mozhi transliteration engine instance — stateful, tracks recent output */
  private mozhiEngine: MozhiEngine;

  // "private constructor" prevents anyone from calling `new InputModeManager()` directly.
  // They must use getInstance() instead, ensuring only one instance ever exists.
  private constructor() {
    this.mozhiEngine = new MozhiEngine();
  }

  /** Get the singleton instance */
  static getInstance(): InputModeManager {
    if (!InputModeManager.instance) {
      InputModeManager.instance = new InputModeManager();
    }
    return InputModeManager.instance;
  }

  /** Toggle between English and Malayalam input modes. Returns the new state. */
  toggle(): boolean {
    this.isMalayalam = !this.isMalayalam;
    // Reset Mozhi buffer when toggling modes — the context is no longer valid
    this.mozhiEngine.reset();
    return this.isMalayalam;
  }

  /** Set the active input scheme */
  setScheme(scheme: InputScheme): void {
    this.scheme = scheme;
    // Reset Mozhi buffer when switching schemes
    this.mozhiEngine.reset();
  }

  /**
   * Reset the Mozhi engine's internal buffer.
   * Call this on word boundaries (space, enter) and cursor movements
   * so the engine doesn't try to combine across word boundaries.
   */
  resetMozhi(): void {
    this.mozhiEngine.reset();
  }

  /**
   * Process a keypress through the active input scheme.
   * Returns an InputResult with the text to insert and how many characters to delete,
   * or null if the key should pass through unchanged.
   */
  processKey(key: string): InputResult | null {
    // If we're in English mode, don't transform anything
    if (!this.isMalayalam) {
      return null;
    }

    // Delegate to the correct scheme handler
    switch (this.scheme) {
      case 'inscript1': {
        // Inscript schemes are stateless 1:1 keymaps — wrap result in InputResult
        const ch = processInscript1Key(key);
        return ch ? { text: ch, deleteBack: 0 } : null;
      }
      case 'inscript2': {
        const ch = processInscript2Key(key);
        return ch ? { text: ch, deleteBack: 0 } : null;
      }
      case 'mozhi':
        // Mozhi is stateful — delegate to the engine which tracks context
        return this.mozhiEngine.processKey(key);
      default:
        return null;
    }
  }
}
