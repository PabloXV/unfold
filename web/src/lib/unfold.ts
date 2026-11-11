import init, { Attempt, dailyCode, dailyLevel, dailySeed, paperColors, parseCode } from './wasm/unfold_wasm.js';
import wasmUrl from './wasm/unfold_wasm_bg.wasm?url';

export { Attempt, dailyCode, dailyLevel, dailySeed, paperColors, parseCode };

let ready: Promise<unknown> | undefined;

/** Loads the wasm core; await it before using anything else here. */
export function loadCore(): Promise<unknown> {
	return (ready ??= init({ module_or_path: wasmUrl }));
}

export type Point = { x: number; y: number };

/** Days since the Unix epoch by the local date. */
export function today(): number {
	const now = new Date();
	return Math.floor(Date.UTC(now.getFullYear(), now.getMonth(), now.getDate()) / 86_400_000);
}

/** Like "Nov 9, 2026", in the given language. */
export function formatDay(day: number, locale: string): string {
	return new Date(day * 86_400_000).toLocaleDateString(locale, {
		timeZone: 'UTC',
		year: 'numeric',
		month: 'short',
		day: 'numeric'
	});
}
