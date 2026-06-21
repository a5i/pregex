// Type definitions for `eregex-wasm`.
//
// Re-exports everything the wasm-bindgen glue declares (`Regex`, `Match`,
// `PartialMatch`, `parseFlags`, `escape`, ...) and adds the flag *constants*,
// which wasm-bindgen cannot emit as `const` exports (see `index.js`).

export * from './eregex_wasm';
export { flags } from './eregex_wasm';

export const IGNORECASE: number;
export const MULTILINE: number;
export const DOTALL: number;
export const UNICODE: number;
export const ASCII: number;
export const VERBOSE: number;
export const FULLCASE: number;
export const WORD: number;
export const LOCALE: number;
export const VERSION0: number;
export const VERSION1: number;
