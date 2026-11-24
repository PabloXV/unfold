import { type Crease, creases, lopsided } from './fold';
import { points, turn } from './geometry';
import { type Shape, same, target } from './match';
import { Puzzle, type Point } from './unfold';

/** One go at a level: the flat sheet, folded a crease at a time until it looks like the target. */
export class Sheet {
	readonly seed: number;
	readonly level: number;
	readonly code: string;
	readonly dots: number;
	/** Vertex triples, in the order the stack indexes them. */
	readonly flaps: number[][];
	/** The fewest folds that make the target. */
	readonly par: number;
	/** Per flap, the sign of its area on the flat sheet; drawn the other way round, it shows its back. */
	readonly faceUp: number[];
	/** The flat sheet, its flaps made lopsided, fitted into the unit circle. */
	readonly upright: Point[];
	readonly creases: Crease[];
	/** The folded shape to match. */
	readonly target: Shape;

	pos = $state.raw<Point[]>([]);
	/** Flaps bottom to top. */
	stack = $state.raw<number[]>([]);
	moves = $state(0);
	solved = $state(false);

	/** Moves made before this go began, which still count. */
	#before: number;

	constructor(seed: number, level: number, before = 0) {
		const a = new Puzzle(seed, level);
		this.seed = seed;
		this.level = level;
		this.code = a.code;
		this.dots = a.dots;
		this.flaps = groups(a.flaps(), 3);
		const shape = points(a.shape());
		this.faceUp = this.flaps.map((f) => Math.sign(turn(f.map((v) => shape[v]))));
		this.creases = creases(this.flaps);
		this.upright = lopsided(points(a.upright()), this.flaps, this.faceUp, this.creases, seed, level);
		a.free();
		const made = target(this.upright, this.flaps, this.creases, Math.max(1, Math.floor(level / 2)), seed * 31 + level);
		[this.target, this.par] = [made.shape, made.folds];
		this.pos = this.upright;
		this.stack = this.flaps.map((_, k) => k);
		this.#before = before;
		this.moves = before;
	}

	/** Whether restarting would lose anything. */
	get inProgress(): boolean {
		return !this.solved && this.moves > this.#before;
	}

	/** Checks, once a fold or turn has settled, whether the paper now looks like the target. */
	settled(): void {
		this.solved = same({ pos: this.pos, stack: this.stack }, this.target, this.flaps, this.faceUp);
	}
}

function groups(flat: ArrayLike<number>, size: number): number[][] {
	return Array.from({ length: flat.length / size }, (_, i) => Array.from({ length: size }, (_, j) => flat[size * i + j]));
}
