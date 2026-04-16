// Focus trap Svelte action for modal dialogs.
//
// Apply to the modal-card element with `use:focusTrap`. On mount:
//   - Saves the element that had focus before the modal opened.
//   - Moves focus to the first focusable descendant (or the card itself if
//     none exist, so Escape/Tab handlers on the card still receive keys).
// While mounted:
//   - Tab / Shift+Tab cycle through focusable descendants without leaving
//     the trap. This prevents the user from tabbing back into the editor
//     behind the modal.
// On destroy (i.e. when the modal unmounts):
//   - Focus returns to the previously focused element, so a keyboard user
//     lands back on the button that opened the modal.
//
// A Svelte "action" is a function that runs when an element is mounted
// and returns an optional object with `destroy` / `update` methods. It's
// wired up at the element with `use:focusTrap`.

const FOCUSABLE_SELECTOR = [
	'a[href]',
	'button:not([disabled])',
	'input:not([disabled]):not([type="hidden"])',
	'select:not([disabled])',
	'textarea:not([disabled])',
	'[tabindex]:not([tabindex="-1"])'
].join(',');

function getFocusable(root: HTMLElement): HTMLElement[] {
	return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
		(el) => !el.hasAttribute('aria-hidden') && el.offsetParent !== null
	);
}

export function focusTrap(node: HTMLElement) {
	const previouslyFocused = document.activeElement as HTMLElement | null;

	// Defer the initial focus move so any animation-in transform doesn't
	// interfere with the browser's scroll-into-view on focus.
	queueMicrotask(() => {
		const focusable = getFocusable(node);
		if (focusable.length > 0) {
			focusable[0].focus();
		} else {
			// Ensure the node itself can receive focus so keydown handlers
			// attached to it (Escape, etc.) still fire.
			if (!node.hasAttribute('tabindex')) node.setAttribute('tabindex', '-1');
			node.focus();
		}
	});

	function handleKeydown(event: KeyboardEvent) {
		if (event.key !== 'Tab') return;

		const focusable = getFocusable(node);
		if (focusable.length === 0) {
			event.preventDefault();
			return;
		}

		const first = focusable[0];
		const last = focusable[focusable.length - 1];
		const active = document.activeElement as HTMLElement | null;

		if (event.shiftKey) {
			if (active === first || !node.contains(active)) {
				event.preventDefault();
				last.focus();
			}
		} else {
			if (active === last || !node.contains(active)) {
				event.preventDefault();
				first.focus();
			}
		}
	}

	node.addEventListener('keydown', handleKeydown);

	return {
		destroy() {
			node.removeEventListener('keydown', handleKeydown);
			// Restore focus to whatever was focused before the modal opened.
			// Guard against the element being gone from the DOM.
			if (previouslyFocused && document.contains(previouslyFocused)) {
				previouslyFocused.focus();
			}
		}
	};
}
