// Disable SSR for the editor page — ProseMirror manipulates the DOM directly
// and conflicts with SvelteKit's server-side rendering / hydration.
export const ssr = false;
