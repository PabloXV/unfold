<script lang="ts">
	import type { Game } from './game.svelte';
	import { inside, meet, svgPoint } from './geometry';
	import Paper from './Paper.svelte';
	import type { Point } from './unfold';

	let { game, showCursor }: { game: Game; showCursor: boolean } = $props();

	const PAPER = { front: 'var(--paper-front)', back: 'var(--paper-back)', cast: 'var(--cast)', castStep: 'var(--cast-step)' };
	/** How far a hovered or carried dot stands out, as a fraction of the radius. */
	const LIFT = 0.06;
	/** Time constant of a glide, in seconds. */
	const GLIDE = 0.04;
	const UNFOLD_DELAY = 150;
	const UNFOLD_TIME = 1200;
	/** A press between these radii grabs the dot nearest it in angle. */
	const GRAB_FROM = 0.55;
	const GRAB_TO = 1.3;
	const TAU = 2 * Math.PI;

	let svg: SVGSVGElement;
	let pos = $state.raw<Point[]>([]);
	/** Eased from 0 on the circle to 1 fully unfolded. */
	let unfold = $state(0);
	let hovered = $state<number | undefined>(undefined);
	/** The dot being dragged, and the angle of the pointer dragging it. */
	let drag = $state<{ v: number; angle: number } | undefined>(undefined);

	let angle: number[] = [];
	let radius: number[] = [];
	let shown: Game | undefined;
	let raf = 0;
	let last = 0;

	const ease = (t: number) => t * t * (3 - 2 * t);
	const wrap = (a: number) => ((((a + Math.PI) % TAU) + TAU) % TAU) - Math.PI;
	const slotAngle = (slot: number) => (TAU * slot) / game.dots;

	function lifted(v: number): boolean {
		if (game.solved) return false;
		if (drag) return drag.v === v;
		return hovered === v || (showCursor && game.slots[v] === game.cursor);
	}

	function place(): void {
		pos = angle.map((a, v) => {
			const on = { x: Math.cos(a) * radius[v], y: Math.sin(a) * radius[v] };
			const to = game.unfolded[v];
			return unfold > 0 && to ? { x: on.x + (to.x - on.x) * unfold, y: on.y + (to.y - on.y) * unfold } : on;
		});
	}

	function frame(now: number): void {
		const dt = last ? Math.min((now - last) / 1000, 0.1) : 1 / 60;
		last = now;
		const k = 1 - Math.exp(-dt / GLIDE);
		let moving = false;
		for (let v = 0; v < game.dots; v++) {
			const toAngle = drag?.v === v ? drag.angle : slotAngle(game.slots[v]);
			const toRadius = lifted(v) ? 1 + LIFT : 1;
			const da = wrap(toAngle - angle[v]);
			const dr = toRadius - radius[v];
			if (drag?.v === v) {
				angle[v] = toAngle;
				radius[v] += dr * k;
				moving ||= Math.abs(dr) > 1e-4;
			} else if (Math.abs(da) > 1e-4 || Math.abs(dr) > 1e-4) {
				angle[v] += da * k;
				radius[v] += dr * k;
				moving = true;
			} else {
				angle[v] = toAngle;
				radius[v] = toRadius;
			}
		}
		const t = game.solvedAt === undefined ? 0 : Math.min(1, Math.max(0, (now - game.solvedAt - UNFOLD_DELAY) / UNFOLD_TIME));
		unfold = ease(t);
		place();
		raf = moving || (game.solvedAt !== undefined && t < 1) ? requestAnimationFrame(frame) : 0;
	}

	function kick(): void {
		if (!raf) {
			last = 0;
			raf = requestAnimationFrame(frame);
		}
	}

	$effect.pre(() => {
		if (shown === game) return;
		shown = game;
		angle = game.slots.map(slotAngle);
		radius = angle.map(() => 1);
		hovered = drag = undefined;
		unfold = 0;
		place();
	});

	$effect(() => {
		void [game, game.slots, game.cursor, game.solvedAt, hovered, drag, showCursor];
		kick();
	});

	$effect(() => () => cancelAnimationFrame(raf));

	function nearest(p: Point): number | undefined {
		const r = Math.hypot(p.x, p.y);
		if (r < GRAB_FROM || r > GRAB_TO) return;
		const a = Math.atan2(p.y, p.x);
		let best: number | undefined;
		let bestOff = Infinity;
		pos.forEach((q, v) => {
			const off = Math.abs(wrap(Math.atan2(q.y, q.x) - a));
			if (off < bestOff) [best, bestOff] = [v, off];
		});
		return best;
	}

	function onpointerdown(e: PointerEvent): void {
		if (game.solved || drag || e.button > 0) return;
		const p = svgPoint(svg, e);
		const v = nearest(p);
		if (v === undefined) return;
		e.preventDefault();
		(e.currentTarget as Element).setPointerCapture(e.pointerId);
		if (game.carrying) game.putBack();
		game.pickUp(v);
		drag = { v, angle: Math.atan2(p.y, p.x) };
	}

	function onpointermove(e: PointerEvent): void {
		const p = svgPoint(svg, e);
		if (!drag) {
			if (e.pointerType === 'mouse') hovered = game.solved ? undefined : nearest(p);
			return;
		}
		const a = Math.atan2(p.y, p.x);
		drag = { v: drag.v, angle: a };
		const slot = Math.round((((a % TAU) + TAU) % TAU) / (TAU / game.dots)) % game.dots;
		if (slot !== game.slots[drag.v]) game.carryTo(slot);
	}

	function onpointerup(): void {
		if (!drag) return;
		drag = undefined;
		game.setDown();
	}

	function onpointercancel(): void {
		if (!drag) return;
		drag = undefined;
		game.putBack();
	}

	/** Each crossing where its creases are drawn now, unless a higher flap covers it. */
	const marks = $derived(
		pos.length !== game.dots
			? []
			: game.crossingPairs.flatMap(([i, j]) => {
					const [e, f] = [game.edges[i], game.edges[j]];
					const at = meet(pos[e.a], pos[e.b], pos[f.a], pos[f.b]);
					if (!at) return [];
					const covered = game.flaps.slice(Math.min(e.topFlap, f.topFlap) + 1).some((t) => inside(t.map((v) => pos[v]), at));
					return covered ? [] : [at];
				})
	);

	/** Rings only on the held dot and the keyboard's; touch and mouse show what they grab by moving it. */
	function ring(v: number): 'held' | 'focus' | undefined {
		if (game.carried === v) return 'held';
		if (!game.carrying && !drag && showCursor && game.slots[v] === game.cursor) return 'focus';
	}
</script>

<div
	class="board"
	class:dragging={drag !== undefined}
	role="application"
	aria-label="Puzzle board: {game.crossings} crossings, {game.moves} moves"
	{onpointerdown}
	{onpointermove}
	{onpointerup}
	{onpointercancel}
	onpointerleave={() => (hovered = undefined)}
>
	<svg class="paper" viewBox="-1.25 -1.25 2.5 2.5" aria-hidden="true">
		{#if pos.length === game.dots}
			<Paper flaps={game.flaps} faceUp={game.faceUp} {pos} colors={PAPER} depth={!game.solved} id="board" />
		{/if}
	</svg>
	<svg bind:this={svg} class="dots" viewBox="-1.25 -1.25 2.5 2.5" aria-hidden="true">
		{#each marks as m, i (i)}
			<circle class="cross" cx={m.x} cy={-m.y} r="0.045" />
		{/each}
		{#if pos.length === game.dots && !game.solved}
			{#each pos as p, v (v)}
				{@const kind = ring(v)}
				{#if kind}<circle class={kind} cx={p.x} cy={-p.y} r="0.085" />{/if}
			{/each}
		{/if}
	</svg>
</div>

<style>
	.board {
		position: relative;
		width: 100%;
		aspect-ratio: 1;
		touch-action: none;
		user-select: none;
		-webkit-user-select: none;
		container-type: inline-size;
	}
	.board svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
		will-change: transform;
	}
	.paper {
		filter: drop-shadow(0.8cqw 1.6cqw 1.6cqw var(--shadow));
	}
	.dragging {
		cursor: grabbing;
	}
	.cross,
	.held,
	.focus {
		fill: none;
		vector-effect: non-scaling-stroke;
		stroke-dasharray: 3 3;
	}
	.cross {
		stroke: var(--cross);
		stroke-width: 2.5px;
	}
	.held {
		stroke: var(--carry);
		stroke-width: 2.5px;
	}
	.focus {
		stroke: var(--accent);
		stroke-width: 2px;
	}
</style>
