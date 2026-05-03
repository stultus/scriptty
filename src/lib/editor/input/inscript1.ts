// Inscript 1 (original) static keymap for Malayalam input
// This is the older Indian standard keyboard layout for Malayalam.
// It shares many mappings with Inscript 2 but has some differences.

const INSCRIPT1_MAP: Record<string, string> = {
	// Vowels
	D: 'അ',
	E: 'ആ',
	F: 'ഇ',
	R: 'ഈ',
	G: 'ഉ',
	T: 'ഊ',
	S: 'ഋ',

	// Vowel signs (matras)
	e: 'ാ',
	f: 'ി',
	r: 'ീ',
	g: 'ു',
	t: 'ൂ',
	s: 'ൃ',
	w: 'േ',
	W: 'ഐ',
	a: 'ോ',
	A: 'ഔ',
	q: 'ൌ',
	d: 'ൊ',
	z: 'െ',
	Z: 'ൈ',

	// Consonants
	k: 'ക',
	K: 'ഖ',
	i: 'ഗ',
	I: 'ഘ',
	U: 'ങ',
	';': 'ച',
	':': 'ഛ',
	p: 'ജ',
	P: 'ഝ',
	'}': 'ഞ',
	"'": 'ട',
	'"': 'ഠ',
	'[': 'ഡ',
	'{': 'ഢ',
	C: 'ണ',
	l: 'ത',
	L: 'ഥ',
	o: 'ദ',
	O: 'ധ',
	v: 'ന',
	h: 'പ',
	H: 'ഫ',
	y: 'ബ',
	Y: 'ഭ',
	c: 'മ',
	'/': 'യ',
	j: 'ര',
	n: 'ല',
	b: 'വ',
	M: 'ശ',
	'<': 'ഷ',
	m: 'സ',
	u: 'ഹ',
	N: 'ള',
	'>': 'ഴ',
	B: 'റ',

	// Special
	x: '്',
	X: 'ം',
	_: 'ഃ',

	// Conjunct helper
	']': '്'
};

/**
 * Process a single keypress through the Inscript 1 keymap.
 * Returns the corresponding Malayalam character, or null if the key has no mapping.
 */
export function processInscript1Key(key: string): string | null {
	return INSCRIPT1_MAP[key] ?? null;
}
