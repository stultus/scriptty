// SMC Mozhi transliteration engine for Malayalam
// Mozhi is a phonetic (stateful) scheme: sequences of Latin keystrokes map to Malayalam
// characters. Unlike Inscript (1:1 keymap), Mozhi tracks recently output Malayalam text
// and matches it against a conversion hash where keys are "Malayalam output + Latin input".
// This means the engine sometimes needs to delete previously inserted characters and
// replace them with new ones.

/** Result from processing a keystroke through an input scheme */
export interface InputResult {
  /** The Malayalam text to insert */
  text: string;
  /** Number of characters to delete backwards before inserting (for Mozhi backtracking) */
  deleteBack: number;
}

/**
 * Build the Mozhi conversion hash.
 * Keys are either a single Latin character or a Malayalam output + Latin character sequence.
 * Values are the Malayalam text that should replace the matched key.
 */
function buildConversionHash(): Map<string, string> {
  const hash = new Map<string, string>();

  // === VOWEL SIGNS (virama + Latin → vowel sign) ===
  // When a consonant has virama (്) and you type a vowel letter,
  // the virama is replaced with the corresponding vowel sign
  hash.set('്a', '');      // inherent 'a' — just removes virama
  hash.set('്e', 'െ');    // e matra
  hash.set('്i', 'ി');    // i matra
  hash.set('്o', 'ൊ');    // o matra
  hash.set('്u', 'ു');    // u matra
  hash.set('്A', 'ാ');    // aa matra
  hash.set('്E', 'േ');    // ee matra
  hash.set('്I', 'ീ');    // ii matra
  hash.set('്O', 'ോ');    // oo matra
  hash.set('്U', 'ൂ');    // uu matra
  hash.set('്Y', 'ൈ');    // ai matra
  // Double-vowel upgrades (e.g., short e → long ii)
  hash.set('െe', 'ീ');    // e + e → ii
  hash.set('ൊo', 'ൂ');    // o + o → uu
  hash.set('ിi', 'ീ');    // i + i → ii
  hash.set('ിe', 'ീ');    // i + e → ii
  hash.set('ുu', 'ൂ');    // u + u → uu
  hash.set('ുo', 'ൂ');    // u + o → uu (alternate)
  hash.set('്r', '്ര്');  // r after virama → ra-virama (conjunct)

  // === ROMAN → CONSONANT (with virama) ===
  // Single Latin letters map to Malayalam consonant + virama
  hash.set('k', 'ക്');
  hash.set('g', 'ഗ്');
  hash.set('j', 'ജ്');
  hash.set('T', 'ട്');
  hash.set('D', 'ഡ്');
  hash.set('d', 'ദ്');
  hash.set('p', 'പ്');
  hash.set('f', 'ഫ്');
  hash.set('b', 'ബ്');
  hash.set('y', 'യ്');
  hash.set('v', 'വ്');
  hash.set('w', 'വ്');
  hash.set('z', 'ശ്');
  hash.set('S', 'ശ്');
  hash.set('s', 'സ്');
  hash.set('h', 'ഹ്');
  hash.set('x', 'ക്ഷ്');
  hash.set('R', 'റ്');
  hash.set('t', 'റ്റ്');
  // c maps to ക് with ZWJ (used as intermediate for ch → ച്)
  hash.set('c', 'ക്\u200D');

  // Aspirated consonants (consonant output + 'h' → aspirated consonant)
  hash.set('ക്h', 'ഖ്');           // k + h → kh
  hash.set('ഗ്h', 'ഘ്');           // g + h → gh
  hash.set('ക്\u200Dh', 'ച്');     // c + h → ch
  hash.set('ച്h', 'ഛ്');           // ch + h → chh
  hash.set('ജ്h', 'ഝ്');           // j + h → jh
  hash.set('ട്h', 'ഠ്');           // T + h → Th
  hash.set('ഡ്h', 'ഢ്');           // D + h → Dh
  hash.set('റ്റ്h', 'ത്');         // t + h → th (via റ്റ്)
  hash.set('ത്h', 'ഥ്');           // th + h → thh
  hash.set('ദ്h', 'ധ്');           // d + h → dh
  hash.set('പ്h', 'ഫ്');           // p + h → ph
  hash.set('ബ്h', 'ഭ്');           // b + h → bh
  hash.set('സ്h', 'ഷ്');           // s + h → sh
  hash.set('ശ്h', 'ഴ്');           // S/z + h → zh

  // Nasal consonants from chillu combinations
  hash.set('ൻg', 'ങ്');            // n + g → ng
  hash.set('ൻj', 'ഞ്');            // n + j → nj
  hash.set('ൻh', 'ഞ്');            // n + h → nj (alternate)

  // === CHILLU LETTERS (standalone consonant finals) ===
  hash.set('N', 'ൺ');   // retroflex N chillu
  hash.set('n', 'ൻ');   // dental n chillu
  hash.set('m', 'ം');   // anusvara (m)
  hash.set('r', 'ർ');   // chillu r
  hash.set('l', 'ൽ');   // chillu l
  hash.set('L', 'ൾ');   // chillu L (retroflex l)

  // === STANDALONE VOWELS ===
  hash.set('a', 'അ');
  hash.set('അa', 'ആ');    // a + a → aa
  hash.set('A', 'ആ');
  hash.set('e', 'എ');
  hash.set('E', 'ഏ');
  hash.set('എe', 'ഈ');    // e + e → ii (Mozhi convention)
  hash.set('i', 'ഇ');
  hash.set('ഇi', 'ഈ');    // i + i → ii
  hash.set('ഇe', 'ഈ');    // i + e → ii
  hash.set('അi', 'ഐ');    // a + i → ai
  hash.set('I', 'ഐ');
  hash.set('o', 'ഒ');
  hash.set('ഒo', 'ഊ');    // o + o → uu
  hash.set('O', 'ഓ');
  hash.set('അu', 'ഔ');    // a + u → au
  hash.set('ഒu', 'ഔ');    // o + u → au
  hash.set('u', 'ഉ');
  hash.set('ഉu', 'ഊ');    // u + u → uu
  hash.set('U', 'ഊ');
  hash.set('H', 'ഃ');     // visarga
  hash.set('റ്h', 'ഋ');    // special: R + h → Ri
  hash.set('ർ^', 'ഋ');     // chillu r + ^ → Ri
  hash.set('ഋ^', 'ൠ');     // Ri + ^ → long Ri
  hash.set('ൽ^', 'ഌ');     // chillu l + ^ → Li
  hash.set('ഌ^', 'ൡ');     // Li + ^ → long Li

  // === NUMERALS ===
  hash.set('1', '൧');
  hash.set('2', '൨');
  hash.set('3', '൩');
  hash.set('4', '൪');
  hash.set('5', '൫');
  hash.set('6', '൬');
  hash.set('7', '൭');
  hash.set('8', '൮');
  hash.set('9', '൯');
  hash.set('0', '൦');

  // === CONJUNCTS (chillu + consonant → nasal+consonant cluster) ===
  hash.set('ൻt', 'ന്റ്');          // n + t → nta
  hash.set('ന്റ്h', 'ന്ത്');       // nta + h → ntha
  hash.set('ൻk', 'ങ്ക്');          // n + k → nka
  hash.set('ൻn', 'ന്ന്');          // n + n → nna
  hash.set('ൺN', 'ണ്ണ്');          // N + N → NNa
  hash.set('ൾL', 'ള്ള്');          // L + L → LLa
  hash.set('ൽl', 'ല്ല്');          // l + l → lla
  hash.set('ംm', 'മ്മ്');          // m + m → mma
  hash.set('ൻm', 'ന്മ്');          // n + m → nma
  hash.set('ന്ന്g', 'ങ്ങ്');      // nn + g → nnga
  hash.set('ൻd', 'ന്ദ്');          // n + d → nda
  hash.set('ൺm', 'ണ്മ്');          // N + m → Nma
  hash.set('ൽp', 'ല്പ്');          // l + p → lpa
  hash.set('ംp', 'മ്പ്');          // m + p → mpa
  hash.set('റ്റ്t', 'ട്ട്');       // t + t → TTa
  hash.set('ൻT', 'ണ്ട്');          // n + T → NDa
  hash.set('ൺT', 'ണ്ട്');          // N + T → NDa
  hash.set('്ര്^', 'ൃ');           // ra-virama + ^ → Ri sign
  hash.set('ൻc', 'ൻ\u200D');       // n + c → intermediate
  hash.set('ൻ\u200Dh', 'ഞ്ച്');    // n + ch → ncha
  hash.set('ൺD', 'ണ്ഡ്');          // N + D → NDa (retroflex)

  // === CAPS (double/geminate consonants) ===
  hash.set('B', 'ബ്ബ്');
  hash.set('C', 'ക്ക്\u200D');
  hash.set('F', 'ഫ്');
  hash.set('G', 'ഗ്ഗ്');
  hash.set('J', 'ജ്ജ്');
  hash.set('K', 'ക്ക്');
  hash.set('M', 'മ്മ്');
  hash.set('P', 'പ്പ്');
  hash.set('Q', 'ക്യൂ');
  hash.set('V', 'വ്വ്');
  hash.set('W', 'വ്വ്');
  hash.set('X', 'ക്ഷ്');
  hash.set('Y', 'യ്യ്');
  hash.set('Z', 'ശ്ശ്');

  // === OTHERS (special combinations) ===
  hash.set('്L', '്ല്');           // virama + L → la-virama (conjunct)
  hash.set('~', '്\u200C');        // tilde → virama + ZWNJ
  hash.set('്~', '്\u200C');       // already virama + tilde
  hash.set('\u200C~', '\u200C');   // ZWNJ + tilde stays ZWNJ
  hash.set('ം~', 'മ്');            // anusvara + tilde → ma-virama
  hash.set('ക്\u200Dc', 'ക്ക്\u200D');  // c + c → kka-ZWJ
  hash.set('ക്ക്\u200Dh', 'ച്ച്');      // cc + h → ccha
  hash.set('q', 'ക്യൂ');
  hash.set('_', '\u200C');          // underscore → ZWNJ

  // === DYNAMICALLY GENERATED: consonant + second vowel ===
  // When a consonant already has inherent 'a' (no virama), typing another
  // vowel letter changes it to a long vowel sign
  const consonantList = [
    'ക','ഖ','ഗ','ഘ','ങ','ച','ഛ','ജ','ഝ','ഞ',
    'ട','ഠ','ഡ','ഢ','ണ','ത','ഥ','ദ','ധ','ന',
    'പ','ഫ','ബ','ഭ','മ','യ','ര','ല','വ',
    'ശ','ഷ','സ','ഹ','ള','ഴ','റ','റ്റ'
  ];
  for (const c of consonantList) {
    hash.set(c + 'a', c + 'ാ');    // a after consonant → aa matra
    hash.set(c + 'e', c + 'േ');    // e after consonant → ee matra
    hash.set(c + 'i', c + 'ൈ');    // i after consonant → ai matra
    hash.set(c + 'o', c + 'ോ');    // o after consonant → oo matra
    hash.set(c + 'u', c + 'ൗ');    // u after consonant → au sign
  }

  // === DYNAMICALLY GENERATED: chillu + vowel/consonant ===
  // Chillus are "half consonants". When followed by a vowel, they revert
  // to the full consonant form with the vowel sign.
  const chilluEntries: [string, string][] = [
    ['ൺ', 'ണ'], ['ൻ', 'ന'], ['ം', 'മ'], ['ർ', 'ര'], ['ൽ', 'ല'], ['ൾ', 'ള'],
    ['്\u200D', '']  // virama+ZWJ → empty base
  ];
  for (const [chillu, base] of chilluEntries) {
    hash.set(chillu + 'a', base);            // + a → base consonant (inherent a)
    hash.set(chillu + 'e', base + 'െ');      // + e → base + e matra
    hash.set(chillu + 'i', base + 'ി');      // + i → base + i matra
    hash.set(chillu + 'o', base + 'ൊ');      // + o → base + o matra
    hash.set(chillu + 'u', base + 'ു');      // + u → base + u matra
    hash.set(chillu + 'A', base + 'ാ');      // + A → base + aa matra
    hash.set(chillu + 'E', base + 'േ');      // + E → base + ee matra
    hash.set(chillu + 'I', base + 'ീ');      // + I → base + ii matra
    hash.set(chillu + 'O', base + 'ോ');      // + O → base + oo matra
    hash.set(chillu + 'U', base + 'ൂ');      // + U → base + uu matra
    hash.set(chillu + 'Y', base + 'ൈ');      // + Y → base + ai matra
    hash.set(chillu + 'r', base + '്ര്');    // + r → base + ra conjunct
    hash.set(chillu + 'y', base + '്യ്');    // + y → base + ya conjunct
    hash.set(chillu + 'v', base + '്വ്');    // + v → base + va conjunct
    hash.set(chillu + 'w', base + '്വ്');    // + w → base + va conjunct
    hash.set(chillu + '~', base + '്\u200C'); // + ~ → base + virama + ZWNJ
  }

  return hash;
}

/**
 * Mozhi transliteration engine.
 *
 * Maintains a buffer of recently output Malayalam characters (cyrBuffer).
 * On each keystroke, combines the buffer with the new Latin key and runs
 * greedy left-to-right transliteration against the conversion hash.
 * Returns how many characters to delete backwards and what new text to insert.
 */
export class MozhiEngine {
  /** The conversion hash: maps (Malayalam context + Latin key) → Malayalam output */
  private hash: Map<string, string>;
  /** Maximum key length to try when matching in the hash */
  private maxKeyLength: number;
  /** Recent Malayalam output buffer — tracks what was last inserted into the editor */
  private cyrBuffer: string;

  constructor() {
    this.hash = buildConversionHash();
    // Find the longest key in the hash to know how far back to look
    this.maxKeyLength = 0;
    for (const key of this.hash.keys()) {
      if (key.length > this.maxKeyLength) {
        this.maxKeyLength = key.length;
      }
    }
    this.cyrBuffer = '';
  }

  /** Reset the engine state. Call on space, punctuation, cursor change, mode toggle. */
  reset(): void {
    this.cyrBuffer = '';
  }

  /**
   * Process a single keystroke through the Mozhi engine.
   *
   * Combines the recent output buffer (cyrBuffer) with the new Latin key,
   * then runs greedy transliteration on the combined string. Computes a diff
   * between the old buffer and the new result to determine how many characters
   * to delete backwards and what new text to insert.
   *
   * Returns null if no conversion happened (the key should pass through to the editor).
   */
  processKey(key: string): InputResult | null {
    // Combine recent Malayalam output with the new Latin keystroke.
    // The hash has entries like "ക്h" → "ഖ്" that match Malayalam+Latin sequences.
    const input = this.cyrBuffer + key;

    // Run greedy left-to-right transliteration on the combined string
    const result = this.transliterate(input);

    // Find the longest common prefix between old cyrBuffer and new result.
    // Characters in the common prefix don't need to change in the editor.
    let commonPrefix = 0;
    const minLen = Math.min(this.cyrBuffer.length, result.length);
    for (let i = 0; i < minLen; i++) {
      if (this.cyrBuffer[i] === result[i]) {
        commonPrefix++;
      } else {
        break;
      }
    }

    // deleteBack: how many chars of the old buffer need to be removed from the editor
    const deleteBack = this.cyrBuffer.length - commonPrefix;
    // newText: the new characters to insert after the deletion
    const newText = result.slice(commonPrefix);

    // Update the buffer with the new result (keep only the tail for future matching)
    this.cyrBuffer = result.slice(Math.max(0, result.length - this.maxKeyLength));

    // If no conversion happened — the result is just cyrBuffer + key literally appended —
    // return null to let the key pass through to the editor unchanged
    if (deleteBack === 0 && newText === key) {
      return null;
    }

    return { text: newText, deleteBack };
  }

  /**
   * Greedy left-to-right transliteration.
   *
   * Scans the input string from left to right. At each position, tries to match
   * the longest possible substring against the conversion hash. If a match is found,
   * the matched portion is replaced with the hash value and the position advances
   * past it. If no match is found, the character passes through unchanged.
   */
  private transliterate(src: string): string {
    let output = '';
    let pos = 0;

    while (pos < src.length) {
      let matched = false;
      // Try longest match first, then progressively shorter
      const maxLen = Math.min(this.maxKeyLength, src.length - pos);
      for (let len = maxLen; len > 0; len--) {
        const sub = src.substring(pos, pos + len);
        const mapped = this.hash.get(sub);
        // mapped !== undefined because empty string '' is a valid mapping
        // (e.g., '്a' → '' removes the virama to reveal inherent 'a')
        if (mapped !== undefined) {
          output += mapped;
          pos += len;
          matched = true;
          break;
        }
      }
      if (!matched) {
        // No hash match — pass through the character as-is
        output += src[pos];
        pos++;
      }
    }

    return output;
  }
}
