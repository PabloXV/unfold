import { turn } from './geometry';
import { overlaps } from './stacking';
import type { Point } from './unfold';

/** How far from the board's center a finger can drag a corner. */
export const REACH = 1.15;

/** Whether the paper lies flat: every flap the same side up, and no two overlapping. */
export function flat(flaps: number[][], faceUp: number[], pos: Point[]): boolean {
	const sides = flaps.map((f, k) => Math.sign(turn(f.map((v) => pos[v]))) * faceUp[k]);
	return sides.every((s) => s === sides[0]) && overlaps(flaps, pos).size === 0;
}

/** Each dot at its slot on the circle, where the puzzle starts. */
export function circle(slots: number[]): Point[] {
	return slots.map((s) => ({ x: Math.cos((2 * Math.PI * s) / slots.length), y: Math.sin((2 * Math.PI * s) / slots.length) }));
}

export function inside([a, b, c]: Point[], p: Point): boolean {
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

const along = (pos: Point[], d: Point) => pos.map((p) => p.x * d.x + p.y * d.y);

/** How wide the paper is along a direction. */
export function width(pos: Point[], d: Point): number {
	const s = along(pos, d);
	return Math.max(...s) - Math.min(...s);
}

/** The paper turned over part way along a direction: squeezed toward its middle, mirrored once t reaches 1. */
export function squeeze(pos: Point[], d: Point, t: number): Point[] {
	const s = along(pos, d);
	const mid = (Math.min(...s) + Math.max(...s)) / 2;
	const k = Math.cos(Math.PI * t) - 1;
	return pos.map((p, v) => ({ x: p.x + (s[v] - mid) * k * d.x, y: p.y + (s[v] - mid) * k * d.y }));
}

/** Where a dropped corner settles: at least `space` from every other corner, and `thin` from the far edge of each
 * of its flaps, on the side it was dropped on, or the side it came from if dropped right on the edge. */
export function rest(pos: Point[], flaps: number[][], v: number, from: Point, space: number, thin: number): Point {
	const edges = flaps.filter((f) => f.includes(v)).map((f) => f.filter((w) => w !== v).map((w) => pos[w]));
	const off = ([a, b]: Point[], p: Point) => ((b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)) / Math.hypot(b.x - a.x, b.y - a.y);
	let p = pos[v];
	for (let pass = 0; pass < 8; pass++) {
		const was = p;
		const near = pos.find((q, w) => w !== v && Math.hypot(p.x - q.x, p.y - q.y) < space);
		if (near) {
			const d = Math.hypot(p.x - near.x, p.y - near.y);
			const [ux, uy] = d > 1e-9 ? [(p.x - near.x) / d, (p.y - near.y) / d] : [-p.y || 1, p.x];
			const k = space / Math.hypot(ux, uy);
			p = { x: near.x + ux * k, y: near.y + uy * k };
		}
		for (const [a, b] of edges) {
			const d = off([a, b], p);
			if (Math.abs(d) >= thin) continue;
			const side = Math.sign(d) || Math.sign(off([a, b], from)) || 1;
			const len = Math.hypot(b.x - a.x, b.y - a.y);
			const push = side * thin - d;
			p = { x: p.x - ((b.y - a.y) / len) * push, y: p.y + ((b.x - a.x) / len) * push };
		}
		if (p === was) break;
	}
	return p;
}

/** The corners moved and scaled so the paper's extremes are centered and its farthest corner sits on the unit circle. */
export function fit(pos: Point[]): Point[] {
	const [xs, ys] = [pos.map((p) => p.x), pos.map((p) => p.y)];
	const [cx, cy] = [(Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2];
	const r = Math.max(...pos.map((p) => Math.hypot(p.x - cx, p.y - cy))) || 1;
	return pos.map((p) => ({ x: (p.x - cx) / r, y: (p.y - cy) / r }));
}
