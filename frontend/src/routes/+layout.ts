// Client-side SPA: everything renders in the browser; nginx serves a static
// index.html fallback for unknown routes (see vite.config.ts adapter).
export const ssr = false;
export const prerender = false;
