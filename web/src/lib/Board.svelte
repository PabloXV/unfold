<!--
	The paper on its circle. Flaps are drawn bottom to top as filled triangles, each followed by
	the shadow cast on it from above, so higher paper hides what's under it without any further
	work. The paper has two colored sides, and a flap turned over shows its back. Dots glide
	around the circle when they move, lift when hovered or carried, and the whole sheet unfolds
	into its shape once solved.
-->
<script lang="ts">
	import type { Game } from './game.svelte';
	import Paper from './Paper.svelte';
	import type { Point } from './unfold';

	let { game, showCursor }: { game: Game; showCursor: boolean } = $props();

	const PAPER = { front: 'var(--paper-front)', back: 'var(--paper-back)', cast: 'var(--cast)', castStep: 'var(--cast-step)' };

	/** How far a hovered or carried dot stands out, as a fraction of the radius. */
	const LIFT = 0.06;
	/** Time constant of a glide, in seconds: dots are about there after three of these. */
	const GLIDE = 0.04;
	const UNFOLD_DELAY = 150;
	const UNFOLD_TIME = 1200;
	/** A press this far from the center, in radii, grabs the dot nearest it in angle, so on a
	 * phone the whole band around the ring is the target, not just the dot. */
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

	// drawn angle and radius per vertex, eased toward where each belongs
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
				// under the finger, not behind it
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

	// a new level starts in place, with nothing gliding in
	$effect.pre(() => {
		if (shown === game) return;
		shown = game;
		angle = game.slots.map(slotAngle);
		radius = angle.map(() => 1);
		hovered = drag = undefined;
		unfold = 0;
		place();
	});

	// anything that moves a dot starts the animation, which stops itself once all is still
	$effect(() => {
		void [game, game.slots, game.cursor, game.solvedAt, hovered, drag, showCursor];
		kick();
	});

	$effect(() => () => cancelAnimationFrame(raf));

	// --- input

	function boardPoint(e: PointerEvent): Point {
		const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(svg.getScreenCTM()!.inverse());
		return { x: p.x, y: -p.y };
	}

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
		const p = boardPoint(e);
		const v = nearest(p);
		if (v === undefined) return;
		e.preventDefault();
		(e.currentTarget as Element).setPointerCapture(e.pointerId);
		if (game.carrying) game.putBack();
		game.pickUp(v);
		drag = { v, angle: Math.atan2(p.y, p.x) };
	}

	function onpointermove(e: PointerEvent): void {
		const p = boardPoint(e);
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

	// --- drawing

	/** Where two segments meet, if they do. */
	function meet(a: Point, b: Point, c: Point, d: Point): Point | undefined {
		const r = { x: b.x - a.x, y: b.y - a.y };
		const s = { x: d.x - c.x, y: d.y - c.y };
		const denom = r.x * s.y - r.y * s.x;
		if (denom === 0) return;
		const t = ((c.x - a.x) * s.y - (c.y - a.y) * s.x) / denom;
		const u = ((c.x - a.x) * r.y - (c.y - a.y) * r.x) / denom;
		if (t < 0 || t > 1 || u < 0 || u > 1) return;
		return { x: a.x + r.x * t, y: a.y + r.y * t };
	}

	/** Strictly inside a triangle, so points on its edges don't count. */
	function inside([a, b, c]: Point[], p: Point): boolean {
		const side = (u: Point, v: Point) => (v.x - u.x) * (p.y - u.y) - (v.y - u.y) * (p.x - u.x);
		const [d1, d2, d3] = [side(a, b), side(b, c), side(c, a)];
		const eps = 1e-9;
		return (d1 > eps && d2 > eps && d3 > eps) || (d1 < -eps && d2 < -eps && d3 < -eps);
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

	/**
	 * How a dot shows: idle dots ripple outward to say they can be grabbed; once one is held the
	 * others go still and it holds a pulsing ring; a hovered dot, or the keyboard's, holds a
	 * steady one.
	 */
	function dotState(v: number): 'ripple' | 'held' | 'focus' | 'still' {
		if (game.carried === v) return 'held';
		if (game.carrying || drag) return 'still';
		if (hovered === v || (showCursor && game.slots[v] === game.cursor)) return 'focus';
		return 'ripple';
	}
</script>

<div
	class="board"
	class:solved={game.solved}
	class:dragging={drag !== undefined}
	role="application"
	aria-label="Puzzle board: {game.crossings} crossings, {game.moves} moves"
	{onpointerdown}
	{onpointermove}
	{onpointerup}
	{onpointercancel}
	onpointerleave={() => (hovered = undefined)}
>
	<!-- the paper, with its shadow on the table drawn by CSS, which Safari does quickly -->
	<svg class="paper" viewBox="-1.25 -1.25 2.5 2.5" aria-hidden="true">
		{#if pos.length === game.dots}
			<Paper flaps={game.flaps} faceUp={game.faceUp} {pos} colors={PAPER} depth={!game.solved} id="board" />
		{/if}
	</svg>

	<!--
		Everything that animates, on its own layer above the paper, so the paper and its shadows
		are only redrawn when the paper moves.
	-->
	<svg bind:this={svg} class="dots" viewBox="-1.25 -1.25 2.5 2.5" aria-hidden="true">
		{#each marks as m, i (i)}
			<g transform="translate({m.x} {-m.y})"><circle class="ripple cross" r="0.08" /></g>
		{/each}
		{#if pos.length === game.dots && !game.solved}
			{#each pos as p, v (v)}
				{@const state = dotState(v)}
				<g transform="translate({p.x} {-p.y})">
					{#if state === 'ripple'}
						<circle class="ripple" r="0.11" />
					{:else if state === 'held'}
						<circle class="held" r="0.085" />
					{:else if state === 'focus'}
						<circle class="focus" r="0.085" />
					{/if}
				</g>
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
		/* lets the table shadow scale with the board */
		container-type: inline-size;
	}
	.board svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
		/* each layer kept apart by the browser, so the always-animating rings don't make it
		   repaint the paper under them */
		will-change: transform;
	}
	.paper {
		filter: drop-shadow(0.8cqw 1.6cqw 1.6cqw var(--shadow));
	}
	.dragging {
		cursor: grabbing;
	}
	/* rings keep the same line width and dashes at any size, scaled from their own center */
	.ripple,
	.held,
	.focus {
		fill: none;
		vector-effect: non-scaling-stroke;
		stroke-dasharray: 3 3;
		transform-box: fill-box;
		transform-origin: center;
	}
	.ripple {
		stroke: var(--dot);
		stroke-width: 2px;
		animation: ripple 1.5s ease-out infinite;
	}
	.ripple.cross {
		stroke: var(--cross);
		stroke-width: 2.5px;
	}
	.held {
		stroke: var(--carry);
		stroke-width: 2.5px;
		animation: held 1.2s ease-in-out infinite;
	}
	.focus {
		stroke: var(--accent);
		stroke-width: 2px;
	}
	/* bright for most of the way out, fading at the end */
	@keyframes ripple {
		0% {
			transform: scale(0.2);
			opacity: 1;
		}
		55% {
			opacity: 0.85;
		}
		100% {
			transform: scale(1);
			opacity: 0;
		}
	}
	@keyframes held {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.35;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.ripple,
		.held {
			animation: none;
		}
		.ripple {
			transform: scale(0.55);
			opacity: 0.7;
		}
	}
</style>
