// Inscript 2 (Enhanced) static keymap for Malayalam input
// This is the standard Indian government keyboard layout for Malayalam.
// Each key on a US QWERTY keyboard maps to a specific Malayalam Unicode character.

const INSCRIPT2_MAP: Record<string, string> = {
	// Vowels (unshifted)
	D: 'അ', // a
	E: 'ആ', // aa
	F: 'ഇ', // i
	R: 'ഈ', // ii
	G: 'ഉ', // u
	T: 'ഊ', // uu
	S: 'ഋ', // ri
	'+': 'ഌ', // li

	// Vowel signs (matras) — used after consonants
	e: 'ാ', // aa matra
	f: 'ി', // i matra
	r: 'ീ', // ii matra
	g: 'ു', // u matra
	t: 'ൂ', // uu matra
	s: 'ൃ', // ri matra
	Z: 'ൈ', // ai matra
	w: 'േ', // ee matra
	W: 'ഐ', // ai vowel
	a: 'ോ', // oo matra
	A: 'ഔ', // au vowel
	q: 'ൌ', // au matra
	d: 'ൊ', // o matra

	// Consonants
	k: 'ക', // ka
	K: 'ഖ', // kha
	i: 'ഗ', // ga
	I: 'ഘ', // gha
	U: 'ങ', // nga
	';': 'ച', // cha
	':': 'ഛ', // chha
	p: 'ജ', // ja
	P: 'ഝ', // jha
	'}': 'ഞ', // nja
	"'": 'ട', // tta
	'"': 'ഠ', // ttha
	'[': 'ഡ', // dda
	'{': 'ഢ', // ddha
	C: 'ണ', // nna
	l: 'ത', // tha
	L: 'ഥ', // thha
	o: 'ദ', // da
	O: 'ധ', // dha
	v: 'ന', // na
	h: 'പ', // pa
	H: 'ഫ', // pha
	y: 'ബ', // ba
	Y: 'ഭ', // bha
	c: 'മ', // ma
	'/': 'യ', // ya
	j: 'ര', // ra
	n: 'ല', // la
	b: 'വ', // va
	M: 'ശ', // sha
	'<': 'ഷ', // ssa
	m: 'സ', // sa
	u: 'ഹ', // ha
	N: 'ള', // lla
	'>': 'ഴ', // zha
	B: 'റ', // rra

	// Special characters
	x: '്', // virama (chandrakkala) — removes inherent vowel
	_: 'ഃ', // visarga
	X: 'ം', // anusvara
	'\\': 'ർ', // chillu r

	// Chillu letters (using dead key or special mappings)
	'|': 'ൽ', // chillu l

	// Digits — Malayalam numerals (shifted number keys)
	')': '൦', // 0
	'!': '൧', // 1
	'@': '൨', // 2
	'#': '൩', // 3
	$: '൪', // 4
	'%': '൫', // 5
	'^': '൬', // 6
	'&': '൭', // 7
	'*': '൮', // 8
	'(': '൯' // 9
};

/**
 * Process a single keypress through the Inscript 2 keymap.
 * Returns the corresponding Malayalam character, or null if the key has no mapping.
 */
export function processInscript2Key(key: string): string | null {
	return INSCRIPT2_MAP[key] ?? null;
}
