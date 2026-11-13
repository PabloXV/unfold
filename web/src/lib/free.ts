import { turn } from './geometry';
import { overlaps } from './stacking';
import type { Point } from './unfold';

/** Whether the paper lies flat: every flap the same side up, and no two overlapping. */
export function flat(flaps: number[][], faceUp: number[], pos: Point[]): boolean {
	const sides = flaps.map((f, k) => Math.sign(turn(f.map((v) => pos[v]))) * faceUp[k]);
	return sides.every((s) => s === sides[0]) && overlaps(flaps, pos).size === 0;
}

export function inside([a, b, c]: Point[], p: Point): boolean {
	const s = [turn([a, b, p]), turn([b, c, p]), turn([c, a, p])];
	return s.every((d) => d > 1e-9) || s.every((d) => d < -1e-9);
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

/** The corners moved and scaled so the paper's extremes are centered and its farthest corner sits on the unit circle. */
export function fit(pos: Point[]): Point[] {
	const [xs, ys] = [pos.map((p) => p.x), pos.map((p) => p.y)];
	const [cx, cy] = [(Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2];
	const r = Math.max(...pos.map((p) => Math.hypot(p.x - cx, p.y - cy))) || 1;
	return pos.map((p) => ({ x: (p.x - cx) / r, y: (p.y - cy) / r }));
}
