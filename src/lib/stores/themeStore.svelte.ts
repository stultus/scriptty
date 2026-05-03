// Theme store — manages dark/light mode toggle with localStorage persistence.
// Uses Svelte 5 $state rune for reactivity.

const STORAGE_KEY = 'scriptty-theme';
type Theme = 'dark' | 'light';

function getInitialTheme(): Theme {
	if (typeof window === 'undefined') return 'dark';
	const stored = localStorage.getItem(STORAGE_KEY);
	if (stored === 'dark' || stored === 'light') return stored;
	return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

class ThemeStore {
	current = $state<Theme>('dark');

	init() {
		this.current = getInitialTheme();
		this.apply();
	}

	toggle() {
		this.current = this.current === 'dark' ? 'light' : 'dark';
		localStorage.setItem(STORAGE_KEY, this.current);
		this.apply();
	}

	get isDark(): boolean {
		return this.current === 'dark';
	}

	private apply() {
		document.documentElement.setAttribute('data-theme', this.current);
	}
}

export const themeStore = new ThemeStore();
