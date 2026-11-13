/** How long flaps stay lit, in ms. */
const LIT = 700;

/** Flaps lit up for a moment, to show what keeps a crease from folding; each call lights them afresh. */
export class Blink {
	lit = $state<Set<number> | undefined>(undefined);
	#timer = 0;

	show(flaps: number[]): void {
		this.stop();
		this.lit = undefined;
		requestAnimationFrame(() => {
			this.lit = new Set(flaps);
			this.#timer = window.setTimeout(() => (this.lit = undefined), LIT);
		});
	}

	/** The lit flaps by their place in `stack`, as Paper takes them. */
	places(stack: number[]): Set<number> | undefined {
		return this.lit && new Set(stack.flatMap((k, i) => (this.lit!.has(k) ? [i] : [])));
	}

	stop(): void {
		clearTimeout(this.#timer);
	}
}
