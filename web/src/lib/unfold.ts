// The puzzle core (Rust, compiled to wasm), loaded once, plus the small helpers around it that
// only the web needs.
import init, { Attempt, dailyLevels, dailySeed, parseCode } from './wasm/unfold_wasm.js';
import wasmUrl from './wasm/unfold_wasm_bg.wasm?url';

export { Attempt, dailyLevels, dailySeed, parseCode };

let ready: Promise<unknown> | undefined;

/** Call before anything else from this module, and wait for it. */
export function loadCore(): Promise<unknown> {
	return (ready ??= init({ module_or_path: wasmUrl }));
}

export type Point = { x: number; y: number };

export type Edge = {
	a: number;
	b: number;
	topFlap: number;
	/** Shared by two flaps, as opposed to part of the outline. */
	shared: boolean;
};

/** Days since the Unix epoch by the local date, so a new daily comes at local midnight. */
export function today(): number {
	const now = new Date();
	return Math.floor(Date.UTC(now.getFullYear(), now.getMonth(), now.getDate()) / 86_400_000);
}

/** Like "Nov 9, 2026", in the reader's own style. */
export function formatDay(day: number): string {
	return new Date(day * 86_400_000).toLocaleDateString(undefined, {
		timeZone: 'UTC',
		year: 'numeric',
		month: 'short',
		day: 'numeric'
	});
}

/** Like ABC123. */
export function formatSeed(seed: number): string {
	return seed.toString(16).toUpperCase().padStart(6, '0');
}
