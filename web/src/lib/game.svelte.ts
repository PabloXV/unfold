import { points, turn } from './geometry';
import { record } from './scores';
import { Attempt, type Point } from './unfold';

/** One attempt at a level as reactive state, mirroring the wasm Attempt after every change. */
export class Game {
	readonly seed: number;
	readonly level: number;
	readonly code: string;
	readonly dots: number;
	/** Vertex triples, stacked bottom to top. */
	readonly flaps: number[][];
	readonly par: number;
	/** Per flap, the sign of its area on the flat sheet; drawn the other way round, it shows its back. */
	readonly faceUp: number[];

	slots = $state<number[]>([]);
	moves = $state(0);
	solved = $state(false);
	carried = $state<number | undefined>(undefined);
	crossings = $state(0);
	/** The slot keyboard play is on, which follows the carried dot. */
	cursor = $state(0);
	/** When the solve happened, by performance.now(). */
	solvedAt = $state<number | undefined>(undefined);
	/** Where each vertex unfolds to once solved. */
	unfolded: Point[] = [];

	#attempt: Attempt;

	constructor(seed: number, level: number) {
		this.#attempt = new Attempt(seed, level);
		this.seed = seed;
		this.level = level;
		this.code = this.#attempt.code;
		this.dots = this.#attempt.dots;
		this.flaps = groups(this.#attempt.flaps(), 3);
		this.par = this.#attempt.par;
		const sheet = points(this.#attempt.shape());
		this.faceUp = this.flaps.map((f) => Math.sign(turn(f.map((v) => sheet[v]))));
		this.#sync();
	}

	get carrying(): boolean {
		return this.carried !== undefined;
	}

	/** Whether restarting would lose anything. */
	get inProgress(): boolean {
		return !this.solved && (this.moves > 0 || this.carrying);
	}

	vertexAt(slot: number): number {
		return this.slots.indexOf(slot);
	}

	pickUp(v: number): void {
		this.#attempt.pickUp(v);
		this.#sync();
		if (this.carried !== undefined) this.cursor = this.slots[this.carried];
	}

	/** +1 counterclockwise, -1 clockwise; a carried dot comes along. */
	step(dir: 1 | -1): void {
		if (this.carrying) this.#attempt.step(dir);
		this.cursor = (this.cursor + dir + this.dots) % this.dots;
		this.#sync();
	}

	/** Carries the carried dot to a slot the short way round, one slot at a time. */
	carryTo(slot: number): void {
		if (this.carried === undefined) return;
		for (let guard = 0; guard < this.dots; guard++) {
			const at = this.#attempt.slots()[this.carried];
			if (at === slot) break;
			const ahead = (slot - at + this.dots) % this.dots;
			this.#attempt.step(ahead <= this.dots / 2 ? 1 : -1);
		}
		this.cursor = slot;
		this.#sync();
	}

	setDown(): void {
		const counted = this.#attempt.setDown();
		this.#sync();
		if (counted && this.solved) {
			this.unfolded = points(this.#attempt.unfoldTarget());
			this.solvedAt = performance.now();
			record(this.code, this.moves);
		}
	}

	putBack(): void {
		const v = this.#attempt.putBack();
		this.#sync();
		if (v !== undefined) this.cursor = this.slots[v];
	}

	/** Picks up the dot under the cursor, or sets the carried one down. */
	select(): void {
		if (this.carrying) this.setDown();
		else this.pickUp(this.vertexAt(this.cursor));
	}

	/** The flat sheet fitted into the unit circle, the way it was drawn. */
	sheet(): Point[] {
		return points(this.#attempt.upright());
	}

	/** Releases the wasm side; the game can't be used after. */
	free(): void {
		this.#attempt.free();
	}

	#sync(): void {
		const a = this.#attempt;
		this.slots = Array.from(a.slots());
		this.moves = a.moves;
		this.solved = a.solved;
		this.carried = a.carried;
		this.crossings = a.crossings;
	}
}

function groups(flat: ArrayLike<number>, size: number): number[][] {
	return Array.from({ length: flat.length / size }, (_, i) => Array.from({ length: size }, (_, j) => flat[size * i + j]));
}
