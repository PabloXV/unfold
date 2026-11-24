import { type Crease, random, swinger, swung } from './fold';
import { inside, turn } from './geometry';
import { Sheet } from './sheet.svelte';
import type { Point } from './unfold';

/** How often making a target turns the paper over before a fold. */
const TURN = 0.35;
/** How finely the match check samples the paper, across its widest side. */
const GRID = 40;

/** A folded shape: where each corner lies and the flaps bottom to top. */
export type Shape = { pos: Point[]; stack: number[] };

/** The paper turned over: mirrored, its stack upside down. */
const turned = ({ pos, stack }: Shape): Shape => ({ pos: pos.map((p) => ({ x: -p.x, y: p.y })), stack: [...stack].reverse() });

/** The flat sheet folded along up to `k` different creases by the taps a player can make, turned over now and then. */
function target(sheet: Point[], flaps: number[][], all: Crease[], k: number, seed: number): { shape: Shape; folds: number } {
	const rnd = random(seed);
	let shape: Shape = { pos: sheet, stack: flaps.map((_, i) => i) };
	const used = new Set<Crease>();
	while (used.size < k) {
		if (rnd() < TURN) shape = turned(shape);
		const options = all.flatMap((c) => {
			const swing = used.has(c) ? undefined : swinger(c, all, shape.pos, flaps, shape.stack);
			return swing ? [{ c, swing }] : [];
		});
		if (!options.length) break;
		const { c, swing } = options[Math.floor(rnd() * options.length)];
		const [pos, stack] = swung(shape.pos, flaps, shape.stack, c, swing);
		shape = { pos, stack };
		used.add(c);
	}
	return { shape, folds: used.size };
}

/** The map taking three points onto three others, as x' = a x + b y + c, y' = d x + e y + f. */
function affine([p, q, r]: Point[], [P, Q, R]: Point[]): (v: Point) => Point {
	const det = turn([p, q, r]);
	const solve = (u: number, v: number, w: number) => {
		const a = ((v - u) * (r.y - p.y) - (w - u) * (q.y - p.y)) / det;
		const b = ((w - u) * (q.x - p.x) - (v - u) * (r.x - p.x)) / det;
		return [a, b, u - a * p.x - b * p.y];
	};
	const [a, b, c] = solve(P.x, Q.x, R.x);
	const [d, e, f] = solve(P.y, Q.y, R.y);
	return (v) => ({ x: a * v.x + b * v.y + c, y: d * v.x + e * v.y + f });
}

/** What shows at a point: the top flap there and which side of it faces up, or nothing. */
function seen(shape: Shape, flaps: number[][], faceUp: number[], p: Point): string {
	const k = shape.stack.findLast((f) => inside(flaps[f].map((v) => shape.pos[v]), p));
	return k === undefined ? '' : `${k}${Math.sign(turn(flaps[k].map((v) => shape.pos[v]))) * faceUp[k]}`;
}

/** Whether two shapes of one sheet look the same, however one is moved or turned: the same flap on top, same side up, everywhere. */
export function same(a: Shape, b: Shape, flaps: number[][], faceUp: number[]): boolean {
	const onto = affine(flaps[0].map((v) => b.pos[v]), flaps[0].map((v) => a.pos[v]));
	const moved = { pos: b.pos.map(onto), stack: b.stack };
	if (moved.pos.some((p, v) => Math.hypot(p.x - a.pos[v].x, p.y - a.pos[v].y) > 1e-6)) return false;
	const [xs, ys] = [a.pos.map((p) => p.x), a.pos.map((p) => p.y)];
	const [x0, y0] = [Math.min(...xs), Math.min(...ys)];
	const step = Math.max(Math.max(...xs) - x0, Math.max(...ys) - y0) / GRID;
	for (let i = 0.537; i < GRID; i++) {
		for (let j = 0.461; j < GRID; j++) {
			const p = { x: x0 + i * step, y: y0 + j * step };
			if (seen(a, flaps, faceUp, p) !== seen(moved, flaps, faceUp, p)) return false;
		}
	}
	return true;
}

/** A level of the matching game: the flat sheet to fold until it looks like the target. */
export class Match extends Sheet {
	readonly target: Shape;
	/** The fewest folds that make the target. */
	readonly folds: number;

	constructor(seed: number, level: number, folds: number) {
		super(seed, level);
		const made = target(this.upright, this.flaps, this.creases, folds, seed * 31 + level);
		[this.target, this.folds] = [made.shape, made.folds];
		this.pos = this.upright;
		this.stack = this.flaps.map((_, k) => k);
	}

	settled(): void {
		this.solved = same({ pos: this.pos, stack: this.stack }, this.target, this.flaps, this.faceUp);
	}
}
