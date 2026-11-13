import { type Crease, creases, crumple } from './fold';
import { circle, flat } from './free';
import { points, turn } from './geometry';
import { Attempt, type Point } from './unfold';

/** One go at a level: the folded paper, its corners dragged anywhere until it lies flat. */
export class Sheet {
	readonly seed: number;
	readonly level: number;
	readonly code: string;
	readonly dots: number;
	/** Vertex triples, in the order the stack indexes them. */
	readonly flaps: number[][];
	readonly par: number;
	/** Per flap, the sign of its area on the flat sheet; drawn the other way round, it shows its back. */
	readonly faceUp: number[];
	/** The flat sheet fitted into the unit circle, the way it was drawn. */
	readonly upright: Point[];
	/** With folding, the creases the paper can fold along; without, corners drag anywhere. */
	readonly creases?: Crease[];

	pos = $state.raw<Point[]>([]);
	/** Flaps bottom to top. */
	stack = $state.raw<number[]>([]);
	moves = $state(0);
	solved = $state(false);

	/** Moves made before this go began, which still count. */
	#before: number;

	constructor(seed: number, level: number, before = 0, folding = false) {
		const a = new Attempt(seed, level);
		this.seed = seed;
		this.level = level;
		this.code = a.code;
		this.dots = a.dots;
		this.flaps = groups(a.flaps(), 3);
		let par = a.par;
		const shape = points(a.shape());
		this.faceUp = this.flaps.map((f) => Math.sign(turn(f.map((v) => shape[v]))));
		this.upright = points(a.upright());
		this.pos = circle(Array.from(a.slots()));
		a.free();
		this.stack = this.flaps.map((_, k) => k);
		if (folding) {
			this.creases = creases(this.flaps);
			const start = crumple(this.upright, this.flaps, this.creases, seed, level);
			[this.pos, this.stack, par] = [start.pos, start.stack, start.par];
		}
		this.par = par;
		this.#before = before;
		this.moves = before;
	}

	/** Whether restarting would lose anything. */
	get inProgress(): boolean {
		return !this.solved && this.moves > this.#before;
	}

	/** Checks, once a moved corner has settled, whether the paper now lies flat. */
	settled(): void {
		this.solved = flat(this.flaps, this.faceUp, this.pos);
	}
}

function groups(flat: ArrayLike<number>, size: number): number[][] {
	return Array.from({ length: flat.length / size }, (_, i) => Array.from({ length: size }, (_, j) => flat[size * i + j]));
}
