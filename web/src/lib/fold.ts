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

/** The flaps on corner `v`'s side of a crease, none if `v` is on the crease. */
export function side(c: Crease, flaps: number[][], v: number): number[] | undefined {
	if (v === c.a || v === c.b) return;
	return c.sides.find((s) => s.some((k) => flaps[k].includes(v)));
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

/** How far over a crease to fold so corner `v` comes closest to `to`, and how far it stays from it. */
export function aim(pos: Point[], c: Crease, v: number, to: Point): { s: number; miss: number } {
	const [a, b, p] = [pos[c.a], pos[c.b], pos[v]];
	const foot = bend(p, a, b, 0);
	const n = { x: p.x - foot.x, y: p.y - foot.y };
	const len2 = n.x * n.x + n.y * n.y;
	if (len2 < 1e-9) return { s: 1, miss: Infinity };
	const s = Math.max(-1, Math.min(1, ((to.x - foot.x) * n.x + (to.y - foot.y) * n.y) / len2));
	return { s, miss: Math.hypot(to.x - foot.x - n.x * s, to.y - foot.y - n.y * s) };
}

/** The flat sheet folded over `k` different creases. A fold only flips its crease, so folding a crease twice undoes it
 * and order never changes the shape: unfolding each of the `k` is the fewest moves. */
export function crumple(sheet: Point[], flaps: number[][], all: Crease[], seed: number, level: number) {
	const rnd = random(seed * 977 + level * 131);
	const picks = all.map((c) => ({ c, r: rnd() })).sort((x, y) => x.r - y.r).slice(0, Math.min(all.length, 1 + Math.floor(level / 2)));
	let pos = sheet;
	let stack = flaps.map((_, i) => i);
	for (const { c } of picks) {
		const moving = c.sides[rnd() < 0.5 ? 0 : 1];
		pos = fold(pos, flaps, c, moving, -1);
		stack = restack(stack, moving, -1);
	}
	return { pos, stack, par: picks.length };
}
