// Single source of truth for input mode state (English/Malayalam toggle, active input scheme).
// This is a singleton — use InputModeManager.getInstance() to access it from anywhere.

import { processInscript1Key } from './inscript1';
import { processInscript2Key } from './inscript2';

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

  /** The currently selected Malayalam input scheme.
   *  Defaults to 'inscript2' because Mozhi (Varnam JS) is not yet integrated. */
  scheme: InputScheme = 'inscript2';

  // "private constructor" prevents anyone from calling `new InputModeManager()` directly.
  // They must use getInstance() instead, ensuring only one instance ever exists.
  private constructor() {}

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
    return this.isMalayalam;
  }

  /** Set the active input scheme */
  setScheme(scheme: InputScheme): void {
    this.scheme = scheme;
  }

  /**
   * Process a keypress through the active input scheme.
   * Returns the Malayalam character(s) to insert, or null if the key should pass through unchanged.
   */
  processKey(key: string): string | null {
    // If we're in English mode, don't transform anything
    if (!this.isMalayalam) {
      return null;
    }

    // Delegate to the correct scheme handler
    switch (this.scheme) {
      case 'inscript1':
        return processInscript1Key(key);
      case 'inscript2':
        return processInscript2Key(key);
      case 'mozhi':
        // Mozhi is a stub for now — pass through until Varnam JS is integrated
        return null;
      default:
        return null;
    }
  }
}
