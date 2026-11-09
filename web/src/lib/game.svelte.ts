// One attempt at a level, as reactive state for the page. The rules all live in the wasm
// Attempt; this mirrors its state into runes after every change so Svelte can redraw, and adds
// the cursor, which only keyboard play needs.
import { Attempt, type Edge, type Point } from './unfold';
import { bestMoves, record } from './scores';

export class Game {
	readonly seed: number;
	readonly level: number;
	readonly code: string;
	readonly dots: number;
	/** Vertex triples, stacked bottom to top. */
	readonly flaps: [number, number, number][];
	readonly edges: Edge[];
	readonly priorBest: number | undefined;

	slots = $state<number[]>([]);
	moves = $state(0);
	solved = $state(false);
	carried = $state<number | undefined>(undefined);
	crossingPairs = $state<[number, number][]>([]);
	/** The slot keyboard play is on. Follows the carried dot. */
	cursor = $state(0);
	/** When the solve happened, by performance.now(), to time the unfold from. */
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
		const f = this.#attempt.flaps();
		this.flaps = Array.from({ length: f.length / 3 }, (_, i) => [f[3 * i], f[3 * i + 1], f[3 * i + 2]]);
		const e = this.#attempt.edges();
		this.edges = Array.from({ length: e.length / 4 }, (_, i) => ({
			a: e[4 * i],
			b: e[4 * i + 1],
			topFlap: e[4 * i + 2],
			shared: e[4 * i + 3] === 1
		}));
		this.priorBest = bestMoves(seed, level);
		this.#sync();
	}

	get crossings(): number {
		return this.crossingPairs.length;
	}

	get carrying(): boolean {
		return this.carried !== undefined;
	}

	/** Whether restarting would lose anything. */
	get inProgress(): boolean {
		return !this.solved && (this.moves > 0 || this.carrying);
	}

	get best(): number | undefined {
		const all = [this.priorBest, this.solved ? this.moves : undefined].filter((n) => n !== undefined);
		return all.length ? Math.min(...all) : undefined;
	}

	get newBest(): boolean {
		return this.solved && this.priorBest !== undefined && this.moves < this.priorBest;
	}

	vertexAt(slot: number): number {
		return this.slots.indexOf(slot);
	}

	pickUp(v: number): void {
		this.#attempt.pickUp(v);
		this.#sync();
		if (this.carried !== undefined) this.cursor = this.slots[this.carried];
	}

	/** +1 counterclockwise, -1 clockwise. A carried dot comes along. */
	step(dir: 1 | -1): void {
		if (this.carrying) this.#attempt.step(dir);
		this.cursor = (this.cursor + dir + this.dots) % this.dots;
		this.#sync();
	}

	/** Carries the carried dot to a slot, the short way round, one slot at a time. */
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
			this.unfolded = pairs(this.#attempt.unfoldTarget());
			this.solvedAt = performance.now();
			record(this.seed, this.level, this.moves);
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

	/** Releases the wasm side. The game can't be used after. */
	free(): void {
		this.#attempt.free();
	}

	#sync(): void {
		const a = this.#attempt;
		this.slots = Array.from(a.slots());
		this.moves = a.moves;
		this.solved = a.solved;
		this.carried = a.carried;
		const c = a.crossingPairs();
		this.crossingPairs = Array.from({ length: c.length / 2 }, (_, i) => [c[2 * i], c[2 * i + 1]]);
	}
}

function pairs(xy: Float64Array): Point[] {
	return Array.from({ length: xy.length / 2 }, (_, i) => ({ x: xy[2 * i], y: xy[2 * i + 1] }));
}
