// A fully static site: the page is rendered once at build time as an empty shell, and the game,
// which needs the wasm, runs only in the browser.
export const prerender = true;
export const ssr = false;
