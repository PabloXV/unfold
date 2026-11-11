import init, { Attempt, dailyLevel, dailySeed, paperColors, parseCode } from './wasm/unfold_wasm.js';
import wasmUrl from './wasm/unfold_wasm_bg.wasm?url';

export { Attempt, dailyLevel, dailySeed, paperColors, parseCode };

let ready: Promise<unknown> | undefined;

/** Loads the wasm core; await it before using anything else here. */
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

/** Days since the Unix epoch by the local date. */
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
