import { fit, flat, inside } from './free';
import { overlap } from './stacking';
import type { Point } from './unfold';

/** A crease shared by two flaps, and the flaps on each side of it, which it cuts the paper into. */
export type Crease = { a: number; b: number; sides: [number[], number[]] };

/** A seeded generator, so a seed and level always fold the same way. */
function random(seed: number): () => number {
	let s = seed >>> 0;
	return () => {
		s = (s + 0x6d2b79f5) >>> 0;
		let t = Math.imul(s ^ (s >>> 15), 1 | s);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

const shared = (f: number[], g: number[]) => f.filter((v) => g.includes(v));

/** The flaps reached from `from` without crossing into `cut`. */
function reach(flaps: number[][], from: number, cut: number): number[] {
	const seen = new Set([from, cut]);
	const out = [from];
	for (let i = 0; i < out.length; i++) {
		flaps.forEach((g, k) => {
			if (!seen.has(k) && shared(flaps[out[i]], g).length === 2) {
				seen.add(k);
				out.push(k);
			}
		});
	}
	return out;
}

/** Every crease between two flaps. */
export function creases(flaps: number[][]): Crease[] {
	const out: Crease[] = [];
	for (let i = 0; i < flaps.length; i++) {
		for (let j = i + 1; j < flaps.length; j++) {
			const ab = shared(flaps[i], flaps[j]);
			if (ab.length === 2) out.push({ a: ab[0], b: ab[1], sides: [reach(flaps, i, j), reach(flaps, j, i)] });
		}
	}
	return out;
}

/** Where a corner lands folded `s` of the way: 1 where it is, -1 mirrored over the crease, between tilted toward it. */
function bend(p: Point, a: Point, b: Point, s: number): Point {
	const [dx, dy] = [b.x - a.x, b.y - a.y];
	const k = ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy);
	const foot = { x: a.x + dx * k, y: a.y + dy * k };
	return { x: foot.x + (p.x - foot.x) * s, y: foot.y + (p.y - foot.y) * s };
}

/** The paper with one side of a crease folded `s` of the way over. */
export function fold(pos: Point[], flaps: number[][], c: Crease, moving: number[], s: number): Point[] {
	const corners = new Set(moving.flatMap((k) => flaps[k]));
	return pos.map((p, w) => (corners.has(w) && w !== c.a && w !== c.b ? bend(p, pos[c.a], pos[c.b], s) : p));
}

/** The stack once the moving flaps pass over: they land on top, turned over, so their own order reverses. */
export function restack(stack: number[], moving: number[], s: number): number[] {
	if (s >= 0) return stack;
	return [...stack.filter((k) => !moving.includes(k)), ...stack.filter((k) => moving.includes(k)).reverse()];
}

/** How far a point lies from a segment. */
function away(p: Point, a: Point, b: Point): number {
	const [dx, dy] = [b.x - a.x, b.y - a.y];
	const k = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy || 1)));
	return Math.hypot(p.x - a.x - dx * k, p.y - a.y - dy * k);
}

/** The crease a tap at `p` means: the nearest within `reach` on any flap from the top down to the first one the tap lands inside. */
export function tapped(pos: Point[], flaps: number[][], stack: number[], all: Crease[], p: Point, reach: number): Crease | undefined {
	let best: { c: Crease; d: number } | undefined;
	for (const k of [...stack].reverse()) {
		const f = flaps[k];
		for (const c of all) {
			const d = away(p, pos[c.a], pos[c.b]);
			if (f.includes(c.a) && f.includes(c.b) && d < reach && (!best || d < best.d)) best = { c, d };
		}
		if (inside(f.map((w) => pos[w]), p)) break;
	}
	return best?.c;
}

/** Whether one side of a crease can swing: no flap off it lies over any of its flaps. */
export function free(pos: Point[], flaps: number[][], stack: number[], moving: number[]): boolean {
	const level = new Map(stack.map((k, i) => [k, i]));
	const tri = (k: number) => flaps[k].map((w) => pos[w]);
	return moving.every((m) => stack.every((k) => moving.includes(k) || level.get(k)! < level.get(m)! || !overlap(tri(m), tri(k))));
}

/** The side of a crease that swings when tapped: the smaller if nothing lies on it, else the larger, else none. */
export function swinger(c: Crease, pos: Point[], flaps: number[][], stack: number[]): number[] | undefined {
	const [small, large] = c.sides[0].length <= c.sides[1].length ? c.sides : [c.sides[1], c.sides[0]];
	return [small, large].find((side) => free(pos, flaps, stack, side));
}

/** The flat sheet folded over up to `k` different creases in turn, each swinging a side nothing lies on, and the side
 * a tap would swing back; so tapping them in reverse unfolds it, and as each needs a tap, that many is the fewest moves. */
export function crumple(sheet: Point[], flaps: number[][], all: Crease[], seed: number, level: number) {
	const rnd = random(seed * 977 + level * 131);
	let pos = sheet;
	let stack = flaps.map((_, i) => i);
	const used = new Set<Crease>();
	for (let i = Math.min(all.length, 1 + Math.floor(level / 2)); i > 0; i--) {
		const options = all.flatMap((c) =>
			used.has(c)
				? []
				: c.sides.flatMap((side) => {
						if (!free(pos, flaps, stack, side)) return [];
						const [p, s] = [fold(pos, flaps, c, side, -1), restack(stack, side, -1)];
						return swinger(c, p, flaps, s) === side ? [{ c, p, s }] : [];
					}),
		);
		if (!options.length) break;
		const pick = options[Math.floor(rnd() * options.length)];
		[pos, stack] = [pick.p, pick.s];
		used.add(pick.c);
	}
	return { pos, stack, par: used.size };
}

/** How different a flap's sides must be, as a share of its longest; its smallest angle, in radians; and how far
 * one flap's corner must land from its neighbor's when folded over their crease, as a share of the crease. */
const SCALENE = 0.12;
const SHARPEST = 0.4;
const UNMIRRORED = 0.3;

const dist = (p: Point, q: Point) => Math.hypot(p.x - q.x, p.y - q.y);

/** How far a sheet is from every flap lopsided, none a sliver, and no two neighbors mirror images. */
function sameness(pos: Point[], flaps: number[][], pairs: number[][]): number {
	let bad = 0;
	for (const f of flaps) {
		const [a, b, c] = f.map((w) => pos[w]);
		const [x, y, z] = [dist(b, c), dist(c, a), dist(a, b)].sort((m, n) => m - n);
		bad += Math.max(0, SCALENE - (y - x) / z) + Math.max(0, SCALENE - (z - y) / z);
		bad += Math.max(0, SHARPEST - Math.acos(Math.min(1, (y * y + z * z - x * x) / (2 * y * z))));
	}
	for (const [a, b, c, d] of pairs) bad += Math.max(0, UNMIRRORED - dist(bend(pos[d], pos[a], pos[b], -1), pos[c]) / dist(pos[a], pos[b]));
	return bad;
}

/** The flat sheet with corners nudged until no flap is symmetric, a sliver, or its neighbor's mirror image, kept flat. */
export function lopsided(sheet: Point[], flaps: number[][], faceUp: number[], seed: number, level: number): Point[] {
	const rnd = random(seed * 613 + level * 71);
	const pairs = flaps.flatMap((f, i) =>
		flaps.slice(i + 1).flatMap((g) => {
			const ab = shared(f, g);
			return ab.length === 2 ? [[...ab, f.find((v) => !ab.includes(v))!, g.find((v) => !ab.includes(v))!]] : [];
		}),
	);
	const reach = (0.25 * flaps.reduce((t, f) => t + dist(sheet[f[0]], sheet[f[1]]), 0)) / flaps.length;
	let pos = sheet;
	let bad = sameness(pos, flaps, pairs);
	for (let round = 0; round < 3000 && bad > 0; round++) {
		const v = Math.floor(rnd() * pos.length);
		const [r, a] = [reach * rnd(), 2 * Math.PI * rnd()];
		const next = pos.map((p, w) => (w === v ? { x: p.x + r * Math.cos(a), y: p.y + r * Math.sin(a) } : p));
		const worse = sameness(next, flaps, pairs);
		if (worse < bad && flat(flaps, faceUp, next)) [pos, bad] = [next, worse];
	}
	return fit(pos);
}
