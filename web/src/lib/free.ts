import { turn } from './geometry';
import { overlaps } from './stacking';
import type { Point } from './unfold';

/** How close two corners may start, and how far out any corner may go. */
const SPREAD = 0.14;
export const REACH = 1.15;

/** A seeded generator, so a seed and level always scramble the same way. */
function random(seed: number): () => number {
	let s = seed >>> 0;
	return () => {
		s = (s + 0x6d2b79f5) >>> 0;
		let t = Math.imul(s ^ (s >>> 15), 1 | s);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

/** How many corners a level's scramble moves, which is also the fewest drags it needs at most. */
export function moved(level: number, dots: number): number {
	return Math.min(dots - 1, 1 + Math.floor(level * 0.6));
}

/** Whether the paper lies flat: every flap the same side up, and no two overlapping. */
export function flat(flaps: number[][], faceUp: number[], pos: Point[]): boolean {
	const sides = flaps.map((f, k) => Math.sign(turn(f.map((v) => pos[v]))) * faceUp[k]);
	return sides.every((s) => s === sides[0]) && overlaps(flaps, pos).size === 0;
}

/** The flat sheet with some corners dragged to random spots, tangled and with corners kept apart. */
export function scramble(sheet: Point[], flaps: number[][], faceUp: number[], seed: number, level: number): Point[] {
	const next = random(seed * 977 + level);
	const k = moved(level, sheet.length);
	for (;;) {
		const pos = sheet.map((p) => ({ ...p }));
		const order = sheet.map((_, v) => v).sort(() => next() - 0.5);
		for (const v of order.slice(0, k)) {
			const [a, r] = [next() * 2 * Math.PI, Math.sqrt(next()) * 0.95];
			pos[v] = { x: r * Math.cos(a), y: r * Math.sin(a) };
		}
		const apart = pos.every((p, i) => pos.every((q, j) => i === j || Math.hypot(p.x - q.x, p.y - q.y) >= SPREAD));
		if (apart && !flat(flaps, faceUp, pos)) return pos;
	}
}

function inside([a, b, c]: Point[], p: Point): boolean {
	const s = [turn([a, b, p]), turn([b, c, p]), turn([c, a, p])];
	return s.every((d) => d > 1e-9) || s.every((d) => d < -1e-9);
}

/** Corners hidden under a flap drawn above every flap they belong to. */
export function buried(stack: number[], flaps: number[][], pos: Point[]): number[] {
	const level = new Map(stack.map((k, i) => [k, i]));
	return pos.flatMap((p, v) => {
		const own = Math.max(...flaps.flatMap((f, k) => (f.includes(v) ? [level.get(k)!] : [])));
		return stack.slice(own + 1).some((k) => inside(flaps[k].map((w) => pos[w]), p)) ? [v] : [];
	});
}
