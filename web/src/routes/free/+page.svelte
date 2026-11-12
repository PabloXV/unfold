<script lang="ts">
	import { onMount } from 'svelte';
	import Paper from '#lib/Paper.svelte';
	import Icon from '#lib/Icon.svelte';
	import { Game } from '#lib/game.svelte.ts';
	import { REACH, buried, flat, moved, scramble } from '#lib/free.ts';
	import { nearestTo, svgPoint } from '#lib/geometry.ts';
	import { overlaps, restack } from '#lib/stacking.ts';
	import { dailySeed, loadCore, paperColors, today, type Point } from '#lib/unfold.ts';

	const PAPER = { front: 'var(--paper-front)', back: 'var(--paper-back)', cast: 'var(--cast)', castStep: 'var(--cast-step)' };
	/** How near a press must be to grab a corner, and how far it must go to count as a move. */
	const GRAB = 0.16;
	const NUDGE = 0.02;

	let level = $state(3);
	let flaps = $state.raw<number[][]>([]);
	let faceUp = $state.raw<number[]>([]);
	let pos = $state.raw<Point[]>([]);
	let start: Point[] = [];
	let stack = $state.raw<number[]>([]);
	let par = $state(0);
	let moves = $state(0);
	let done = $state(false);
	let svg = $state<SVGSVGElement | undefined>(undefined);
	let drag = $state<{ v: number; from: Point } | undefined>(undefined);
	let lifting = new Set<number>();
	let overlapping = new Set<string>();
	let seed = 0;

	const hidden = $derived(pos.length && !done ? buried(stack, flaps, pos) : []);

	onMount(() => {
		const onhash = () => {
			const asked = Number(location.hash.slice(1)) || 3;
			if (asked !== level || !flaps.length) {
				level = asked;
				load();
			}
		};
		loadCore().then(() => {
			seed = dailySeed(today());
			const [front, back] = paperColors(seed);
			document.documentElement.style.setProperty('--paper-front', front);
			document.documentElement.style.setProperty('--paper-back', back);
			onhash();
		});
		addEventListener('hashchange', onhash);
		return () => removeEventListener('hashchange', onhash);
	});

	function load(): void {
		const g = new Game(seed, level);
		flaps = g.flaps;
		faceUp = g.faceUp;
		start = scramble(g.sheet(), g.flaps, g.faceUp, seed, level);
		par = moved(level, g.dots);
		g.free();
		restart();
		history.replaceState(null, '', `#${level}`);
	}

	function restart(): void {
		pos = start.map((p) => ({ ...p }));
		stack = flaps.map((_, k) => k);
		moves = 0;
		done = false;
	}

	function go(step: number): void {
		if (level + step < 1) return;
		level += step;
		load();
	}

	function at(e: PointerEvent): Point {
		const p = svgPoint(svg!, e);
		const k = Math.min(1, REACH / Math.hypot(p.x, p.y));
		return { x: p.x * k, y: p.y * k };
	}

	function onpointerdown(e: PointerEvent): void {
		if (done || e.button > 0) return;
		const v = nearestTo(pos, at(e), GRAB);
		if (v === undefined) return;
		e.preventDefault();
		svg!.setPointerCapture(e.pointerId);
		drag = { v, from: pos[v] };
		lifting = new Set(flaps.flatMap((f, k) => (f.includes(v) ? [k] : [])));
		overlapping = overlaps(flaps, pos);
		stack = [...stack.filter((k) => !lifting.has(k)), ...stack.filter((k) => lifting.has(k))];
	}

	function onpointermove(e: PointerEvent): void {
		if (!drag) return;
		const p = at(e);
		pos = pos.map((q, v) => (v === drag!.v ? p : q));
		const now = overlaps(flaps, pos);
		stack = restack(stack, lifting, overlapping, now);
		overlapping = now;
	}

	function onpointerup(): void {
		if (!drag) return;
		const { v, from } = drag;
		drag = undefined;
		if (Math.hypot(pos[v].x - from.x, pos[v].y - from.y) < NUDGE) return;
		moves++;
		done = flat(flaps, faceUp, pos);
	}
</script>

<main>
	<h1>unfold <span>free</span></h1>
	<p class="level">level {level} · solvable in {par}</p>
	<div class="board">
		<svg
			bind:this={svg}
			viewBox="-1.25 -1.25 2.5 2.5"
			class:dragging={drag}
			role="application"
			aria-label="Paper"
			{onpointerdown}
			{onpointermove}
			{onpointerup}
			onpointercancel={onpointerup}
		>
			{#if pos.length}
				<g class="paper">
					<Paper flaps={stack.map((k) => flaps[k])} faceUp={stack.map((k) => faceUp[k])} {pos} colors={PAPER} depth={!done} id="free" />
				</g>
				{#each hidden as v (v)}
					<circle class="hidden" cx={pos[v].x} cy={-pos[v].y} r="0.045" />
				{/each}
			{/if}
		</svg>
	</div>
	<p class="status">{done ? `Flat in ${moves} ${moves === 1 ? 'move' : 'moves'}` : `${moves} ${moves === 1 ? 'move' : 'moves'}`}</p>
	<nav>
		<button onclick={() => go(-1)} disabled={level <= 1} aria-label="Previous level">‹</button>
		<button class="icon" onclick={restart} aria-label="Restart"><Icon name="reset" /></button>
		<button onclick={() => go(1)} aria-label="Next level">›</button>
	</nav>
	<p class="how">Drag the corners until the paper lies flat: no flap over another, none flipped.</p>
</main>

<style>
	main {
		max-width: 34rem;
		margin: 0 auto;
		padding: 1.25rem 1rem 2rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
	}
	h1 {
		margin: 0;
		font-size: 2.2rem;
	}
	h1 span {
		color: var(--muted);
		font-weight: 400;
		font-size: 1.2rem;
	}
	.level,
	.how {
		margin: 0;
		color: var(--muted);
		font-size: 0.9rem;
		text-align: center;
	}
	.status {
		margin: 0;
	}
	.board {
		width: min(100%, 62svh);
		aspect-ratio: 1;
		background: radial-gradient(closest-side, var(--spot), transparent);
	}
	svg {
		display: block;
		width: 100%;
		height: 100%;
		overflow: visible;
		touch-action: none;
		user-select: none;
		-webkit-user-select: none;
		container-type: inline-size;
	}
	.paper {
		filter: drop-shadow(0.012px 0.024px 0.02px var(--shadow));
	}
	.dragging {
		cursor: grabbing;
	}
	.hidden {
		fill: none;
		stroke: var(--muted);
		stroke-width: 2px;
		stroke-dasharray: 3 3;
		vector-effect: non-scaling-stroke;
		opacity: 0.8;
	}
	nav {
		display: flex;
		gap: 0.5rem;
	}
	button {
		font: inherit;
		color: var(--text);
		background: var(--button);
		border: 1px solid var(--line);
		border-radius: 999px;
		min-width: 2.75rem;
		height: 2.75rem;
		cursor: pointer;
	}
	button:disabled {
		opacity: 0.4;
		cursor: default;
	}
	button.icon {
		display: grid;
		place-items: center;
		padding: 0;
	}
</style>
