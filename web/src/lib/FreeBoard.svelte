<script lang="ts">
	import { onMount } from 'svelte';
	import { REACH, buried, rest, squeeze, width } from './free';
	import { nearestTo, svgPoint } from './geometry';
	import { t } from './i18n/index.svelte';
	import Paper from './Paper.svelte';
	import type { Sheet } from './sheet.svelte';
	import { overlaps, restack } from './stacking';
	import type { Point } from './unfold';

	let { sheet }: { sheet: Sheet } = $props();

	const PAPER = { front: 'var(--paper-front)', back: 'var(--paper-back)', cast: 'var(--cast)', castStep: 'var(--cast-step)' };
	/** How near a press must be to grab a corner, and how far it must go to start moving it. */
	const GRAB = 0.16;
	const NUDGE = 0.02;
	/** How long a let-go turn or a pushed-off corner takes to settle, in ms. */
	const FALL = 220;
	/** How close two corners may rest, and how thin a flap may get, kept below the least of each on any flat sheet. */
	const SPACE = 0.1;
	const THIN = 0.08;
	/** How far out the paper's widest side may reach before the view shrinks, and how quickly the view follows, in s. */
	const FIT = 1;
	const FOLLOW = 0.12;

	let svg = $state<SVGSVGElement | undefined>(undefined);
	let drag = $state<{ v: number; from: Point; was: Point; started: boolean } | undefined>(undefined);
	let turning = $state<{ from: Point; base: Point[]; under: number[]; dir: Point; t: number; falling?: boolean } | undefined>(undefined);
	let settling: (() => void) | undefined;
	let raf = 0;
	let view = $state.raw(framing());
	let finger: Point = { x: 0, y: 0 };
	let moving = new Set<number>();
	let overlapping = new Set<string>();

	const toScreen = (p: Point) => ({ x: (p.x - view.x) * view.s, y: (p.y - view.y) * view.s });
	const toWorld = (p: Point) => ({ x: p.x / view.s + view.x, y: p.y / view.s + view.y });
	const shown = $derived(sheet.pos.map(toScreen));
	const hidden = $derived(!sheet.solved && !turning ? buried(sheet.stack, sheet.flaps, sheet.pos) : []);

	onMount(() => {
		let last = 0;
		let loop = requestAnimationFrame(function frame(now) {
			reframe(last ? Math.min((now - last) / 1000, 0.1) : 0);
			last = now;
			loop = requestAnimationFrame(frame);
		});
		return () => {
			cancelAnimationFrame(loop);
			cancelAnimationFrame(raf);
			settling?.();
		};
	});

	/** The view that centers the paper's extremes, shrunk only when its widest side would pass FIT. */
	function framing(): { x: number; y: number; s: number } {
		const [xs, ys] = [sheet.pos.map((p) => p.x), sheet.pos.map((p) => p.y)];
		const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
		return { x: (x0 + x1) / 2, y: (y0 + y1) / 2, s: Math.min(1, (2 * FIT) / Math.max(x1 - x0, y1 - y0)) };
	}

	/** Eases the view toward the framing: held still mid-turn, and not zooming mid-drag, where it would run away. */
	function reframe(dt: number): void {
		if (turning) return;
		const want = framing();
		if (drag?.started) want.s = view.s;
		const k = 1 - Math.exp(-dt / FOLLOW);
		const next = { x: view.x + (want.x - view.x) * k, y: view.y + (want.y - view.y) * k, s: view.s + (want.s - view.s) * k };
		if (Math.abs(next.x - view.x) + Math.abs(next.y - view.y) + Math.abs(next.s - view.s) < 1e-6) return;
		view = next;
		if (drag?.started) follow();
	}

	/** Puts the dragged corner under the finger and restacks what it now covers. */
	function follow(): void {
		const p = toWorld(finger);
		sheet.pos = sheet.pos.map((q, v) => (v === drag!.v ? p : q));
		const now = overlaps(sheet.flaps, sheet.pos);
		sheet.stack = restack(sheet.stack, moving, overlapping, now);
		overlapping = now;
	}

	function at(e: PointerEvent): Point {
		const p = svgPoint(svg!, e);
		const k = Math.min(1, REACH / Math.hypot(p.x, p.y));
		return { x: p.x * k, y: p.y * k };
	}

	function onpointerdown(e: PointerEvent): void {
		if (sheet.solved || e.button > 0 || turning) return;
		settling?.();
		if (sheet.solved) return;
		e.preventDefault();
		svg!.setPointerCapture(e.pointerId);
		const v = nearestTo(shown, svgPoint(svg!, e), GRAB);
		if (v === undefined) {
			turning = { from: svgPoint(svg!, e), base: sheet.pos, under: sheet.stack, dir: { x: 1, y: 0 }, t: 0 };
			return;
		}
		drag = { v, from: shown[v], was: sheet.pos[v], started: false };
	}

	/** Notes which flaps move with the corner; they keep their layer and slide out from under whatever covers them. */
	function grab(v: number): void {
		moving = new Set(sheet.flaps.flatMap((f, k) => (f.includes(v) ? [k] : [])));
		overlapping = overlaps(sheet.flaps, sheet.pos);
	}

	/** Eases a dropped corner to where it settles, then checks for flat; a new press finishes it at once. */
	function settle(v: number, to: Point): void {
		const from = sheet.pos[v];
		const place = (k: number) => (sheet.pos = sheet.pos.map((q, w) => (w === v ? { x: from.x + (to.x - from.x) * k, y: from.y + (to.y - from.y) * k } : q)));
		const begin = performance.now();
		const finish = () => {
			cancelAnimationFrame(raf);
			settling = undefined;
			place(1);
			sheet.settled();
		};
		const tick = (now: number) => {
			const k = Math.min(1, (now - begin) / FALL);
			if (k === 1) return finish();
			place(1 - (1 - k) ** 2);
			raf = requestAnimationFrame(tick);
		};
		settling = finish;
		raf = requestAnimationFrame(tick);
	}

	/** Shows the paper turned t of the way over, flipping the stack once it passes edge-on. */
	function show(t: number): void {
		const { base, under, dir } = turning!;
		turning!.t = t;
		sheet.pos = squeeze(base, dir, t);
		sheet.stack = t > 0.5 ? [...under].reverse() : under;
	}

	function onpointermove(e: PointerEvent): void {
		if (turning) {
			if (turning.falling) return;
			const p = svgPoint(svg!, e);
			const [dx, dy] = [p.x - turning.from.x, p.y - turning.from.y];
			const len = Math.hypot(dx, dy);
			if (len < 1e-6) return;
			turning.dir = { x: dx / len, y: dy / len };
			show(Math.min(1, len / (width(turning.base, turning.dir) * view.s)));
			return;
		}
		if (!drag) return;
		finger = at(e);
		if (!drag.started) {
			if (Math.hypot(finger.x - drag.from.x, finger.y - drag.from.y) < NUDGE) return;
			drag.started = true;
			grab(drag.v);
		}
		follow();
	}

	/** Lets go of a turn: finished once past edge-on, else eased back flat. */
	function letGo(): void {
		if (turning!.falling) return;
		turning!.falling = true;
		const from = turning!.t;
		const to = from > 0.5 ? 1 : 0;
		const begin = performance.now();
		const tick = (now: number) => {
			const k = Math.min(1, (now - begin) / FALL);
			show(to + (from - to) * (1 - k) * (1 - k));
			if (k < 1) requestAnimationFrame(tick);
			else turning = undefined;
		};
		requestAnimationFrame(tick);
	}

	function onpointerup(): void {
		if (turning) return letGo();
		if (!drag) return;
		const { v, was, started } = drag;
		drag = undefined;
		if (!started) return;
		sheet.moves++;
		settle(v, rest(sheet.pos, sheet.flaps, v, was, SPACE / view.s, THIN / view.s));
	}
</script>

<div class="board">
	<svg
		bind:this={svg}
		viewBox="-1.25 -1.25 2.5 2.5"
		class:dragging={drag || turning}
		role="application"
		aria-label={t().board(sheet.moves)}
		{onpointerdown}
		{onpointermove}
		{onpointerup}
		onpointercancel={onpointerup}
	>
		<g class="paper">
			<Paper flaps={sheet.stack.map((k) => sheet.flaps[k])} faceUp={sheet.stack.map((k) => sheet.faceUp[k])} pos={shown} colors={PAPER} depth={!sheet.solved} id="board" />
		</g>
		{#each hidden as v (v)}
			<circle class="hidden" cx={shown[v].x} cy={-shown[v].y} r="0.045" />
		{/each}
	</svg>
</div>

<style>
	.board {
		width: 100%;
		aspect-ratio: 1;
		container-type: inline-size;
	}
	svg {
		display: block;
		width: 100%;
		height: 100%;
		overflow: visible;
		touch-action: none;
		user-select: none;
		-webkit-user-select: none;
	}
	.paper {
		filter: drop-shadow(0.012px 0.024px 0.024px var(--shadow));
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
</style>
