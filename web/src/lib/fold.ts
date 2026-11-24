import { fit, inside, overlap, overlaps, turn } from './geometry';
import type { Point } from './unfold';

/** A crease: its corners, the two flaps hinged on it, and the flaps on each side of it, which it cuts the paper into. */
export type Crease = { a: number; b: number; hinge: [number, number]; sides: [number[], number[]] };

/** A seeded generator, so a seed and level always fold the same way. */
export function random(seed: number): () => number {
	let s = seed >>> 0;
	return () => {
		s = (s + 0x6d2b79f5) >>> 0;
		let t = Math.imul(s ^ (s >>> 15), 1 | s);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

/** Whether the paper lies flat: every flap the same side up, and no two overlapping. */
export function flat(flaps: number[][], faceUp: number[], pos: Point[]): boolean {
	const sides = flaps.map((f, k) => Math.sign(turn(f.map((v) => pos[v]))) * faceUp[k]);
	return sides.every((s) => s === sides[0]) && overlaps(flaps, pos).size === 0;
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
			if (ab.length === 2) out.push({ a: ab[0], b: ab[1], hinge: [i, j], sides: [reach(flaps, i, j), reach(flaps, j, i)] });
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

/** A fold as tapped: the flaps that swing, and whether they swing away, under the paper, rather than up over it. */
export type Swing = { moving: number[]; under: boolean };

/** The stack once the moving flaps pass over, or under: they land on top, or beneath, turned over, their own order reversed. */
function restack(stack: number[], moving: number[], s: number, under = false): number[] {
	if (s >= 0) return stack;
	if (under) return restack([...stack].reverse(), moving, s).reverse();
	return [...stack.filter((k) => !moving.includes(k)), ...stack.filter((k) => moving.includes(k)).reverse()];
}

/** The paper and its stack with a swing done `s` of the way: 1 not at all, -1 fully. */
export function swung(pos: Point[], flaps: number[][], stack: number[], c: Crease, { moving, under }: Swing, s = -1): [Point[], number[]] {
	return [fold(pos, flaps, c, moving, s), restack(stack, moving, s, under)];
}

/** How far a point lies from a segment. */
function away(p: Point, a: Point, b: Point): number {
	const [dx, dy] = [b.x - a.x, b.y - a.y];
	const k = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy || 1)));
	return Math.hypot(p.x - a.x - dx * k, p.y - a.y - dy * k);
}

/** What a tap at `p` does: folds a crease with `swing`, or lights `inWay`, what keeps it from folding; or nothing. */
export function tap(p: Point, reach: number, all: Crease[], pos: Point[], flaps: number[][], stack: number[]): { c: Crease; swing: Swing } | { inWay: number[] } | undefined {
	const c = tapped(pos, flaps, stack, all, p, reach);
	if (!c) return;
	const swing = swinger(c, all, pos, flaps, stack);
	return swing ? { c, swing } : { inWay: blocking(c, all, pos, flaps, stack) };
}

/** The crease a tap at `p` means: the nearest within `reach` on any flap from the top down to the first one the tap lands inside. */
function tapped(pos: Point[], flaps: number[][], stack: number[], all: Crease[], p: Point, reach: number): Crease | undefined {
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

/** The flaps off one side of a crease lying over any of its flaps, which keep it from swinging. */
function blockers(pos: Point[], flaps: number[][], stack: number[], moving: number[]): number[] {
	const level = new Map(stack.map((k, i) => [k, i]));
	const tri = (k: number) => flaps[k].map((w) => pos[w]);
	return stack.filter((k) => !moving.includes(k) && moving.some((m) => level.get(k)! > level.get(m)! && overlap(tri(m), tri(k))));
}

/** Whether a segment passes through a triangle's inside, not just along or past it. */
function crosses(t: Point[], a: Point, b: Point): boolean {
	const sign = Math.sign(turn(t));
	let [lo, hi] = [0, 1];
	for (let i = 0; i < 3; i++) {
		const [p, q] = [t[i], t[(i + 1) % 3]];
		const n = { x: -(q.y - p.y) * sign, y: (q.x - p.x) * sign };
		const [at, by] = [n.x * (a.x - p.x) + n.y * (a.y - p.y), n.x * (b.x - a.x) + n.y * (b.y - a.y)];
		if (Math.abs(by) < 1e-12) {
			if (at <= 1e-9) return false;
		} else if (by > 0) lo = Math.max(lo, (1e-9 - at) / by);
		else hi = Math.min(hi, (1e-9 - at) / by);
	}
	return hi - lo > 1e-6;
}

/** Flaps cutting through the paper: lying across a crease, between the two flaps hinged on it, as no paper can. */
function through(pos: Point[], flaps: number[][], stack: number[], all: Crease[]): number[] {
	const level = new Map(stack.map((k, i) => [k, i]));
	const out = new Set<number>();
	for (const { a, b, hinge } of all) {
		const [lo, hi] = hinge.map((k) => level.get(k)!).sort((x, y) => x - y);
		for (const k of stack.slice(lo + 1, hi)) if (crosses(flaps[k].map((w) => pos[w]), pos[a], pos[b])) out.add(k);
	}
	return [...out];
}

/** What keeps one side of a crease from swinging: flaps lying on it, or else flaps the fold would leave cutting through. */
function inWay(c: Crease, all: Crease[], pos: Point[], flaps: number[][], stack: number[], moving: number[]): number[] {
	const on = blockers(pos, flaps, stack, moving);
	return on.length ? on : through(fold(pos, flaps, c, moving, -1), flaps, restack(stack, moving, -1), all);
}

const smaller = (c: Crease) => (c.sides[0].length <= c.sides[1].length ? c.sides[0] : c.sides[1]);

/** What keeps a crease from folding toward the viewer, the way that can be seen. */
function blocking(c: Crease, all: Crease[], pos: Point[], flaps: number[][], stack: number[]): number[] {
	return inWay(c, all, pos, flaps, stack, smaller(c));
}

/** How a tap folds a crease: its smaller side swings up over the paper if nothing is in the way, else under it, else not at all. */
export function swinger(c: Crease, all: Crease[], pos: Point[], flaps: number[][], stack: number[]): Swing | undefined {
	const moving = smaller(c);
	if (!inWay(c, all, pos, flaps, stack, moving).length) return { moving, under: false };
	if (!inWay(c, all, pos, flaps, [...stack].reverse(), moving).length) return { moving, under: true };
}

/** The creases that can fold now, as "low,high" corners. */
export function loose(all: Crease[], pos: Point[], flaps: number[][], stack: number[]): Set<string> {
	return new Set(all.filter((c) => swinger(c, all, pos, flaps, stack)).map((c) => (c.a < c.b ? `${c.a},${c.b}` : `${c.b},${c.a}`)));
}

/** Plays a fold over `ms`, handing `show` the paper folded that far each frame; returns a way to finish it at once. */
export function play(ms: number, c: Crease, swing: Swing, pos: Point[], flaps: number[][], stack: number[], show: (pos: Point[], stack: number[]) => void, done: () => void): () => void {
	const begin = performance.now();
	let raf = 0;
	const at = (k: number) => show(...swung(pos, flaps, stack, c, swing, Math.cos(Math.PI * k)));
	const finish = () => {
		cancelAnimationFrame(raf);
		at(1);
		done();
	};
	const tick = (now: number) => {
		const k = Math.min(1, (now - begin) / ms);
		if (k === 1) return finish();
		at(k);
		raf = requestAnimationFrame(tick);
	};
	raf = requestAnimationFrame(tick);
	return finish;
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
export function lopsided(sheet: Point[], flaps: number[][], faceUp: number[], all: Crease[], seed: number, level: number): Point[] {
	const rnd = random(seed * 613 + level * 71);
	const pairs = all.map(({ a, b, hinge }) => [a, b, ...hinge.map((k) => flaps[k].find((v) => v !== a && v !== b)!)]);
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
