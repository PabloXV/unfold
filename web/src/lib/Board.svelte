<script lang="ts">
	import { onMount } from 'svelte';
	import { Blink } from './blink.svelte';
	import { type Crease, loose, play, tap } from './fold';
	import { squeeze, svgPoint, width } from './geometry';
	import { t } from './i18n/index.svelte';
	import Paper from './Paper.svelte';
	import type { Sheet } from './sheet.svelte';
	import { readJson, writeJson } from './storage';
	import { today, type Point } from './unfold';

	let { sheet }: { sheet: Sheet } = $props();
	/** Each board's own, as an outgoing and incoming board share the page while they slide. */
	const uid = $props.id();

	const PAPER = { front: 'var(--paper-front)', back: 'var(--paper-back)', cast: 'var(--cast)', castStep: 'var(--cast-step)' };
	/** How far a press must move to turn the paper rather than tap it. */
	const NUDGE = 0.02;
	/** How long a let-go turn takes to settle, in ms. */
	const FALL = 220;
	/** How far out the paper's widest side may reach before the view shrinks, and how quickly the view follows, in s. */
	const FIT = 1;
	const FOLLOW = 0.12;
	/** How near a tap must land to a crease to fold it, and how long the fold takes, in ms. */
	const TAP = 0.12;
	const SWING = 320;
	/** Idle this long, in ms, and the paper wobbles to show it turns over; each wobble that day doubles the wait. */
	const IDLE = 5000;
	/** How long a wobble lasts, in ms, and how far over it lifts, as a share of a whole turn. */
	const WOBBLE = 900;
	const LIFT = 0.12;
	const WOBBLES = `unfold:wobbles:${today()}`;

	let svg = $state<SVGSVGElement | undefined>(undefined);
	let turning = $state<{ from: Point; base: Point[]; under: number[]; dir: Point; t: number; moved?: boolean; falling?: boolean } | undefined>(undefined);
	let folding = $state(false);
	const blink = new Blink();
	let settling: (() => void) | undefined;
	let wobbling: (() => void) | undefined;
	let idle = 0;
	let view = $state.raw(framing());

	const toScreen = (p: Point) => ({ x: (p.x - view.x) * view.s, y: (p.y - view.y) * view.s });
	const toWorld = (p: Point) => ({ x: p.x / view.s + view.x, y: p.y / view.s + view.y });
	const shown = $derived(sheet.pos.map(toScreen));
	const pulse = $derived(!sheet.solved && !turning && !folding ? loose(sheet.creases, sheet.pos, sheet.flaps, sheet.stack) : new Set<string>());

	onMount(() => {
		let last = 0;
		let loop = requestAnimationFrame(function frame(now) {
			reframe(last ? Math.min((now - last) / 1000, 0.1) : 0);
			last = now;
			loop = requestAnimationFrame(frame);
		});
		wait();
		return () => {
			cancelAnimationFrame(loop);
			clearTimeout(idle);
			blink.stop();
			wobbling?.();
			settling?.();
		};
	});

	/** Starts the idle clock: the first wobble of the day after IDLE, each later one after twice the last wait. */
	function wait(): void {
		clearTimeout(idle);
		if (sheet.solved) return;
		idle = window.setTimeout(wobble, IDLE * 2 ** (readJson<number>(WOBBLES) ?? 0));
	}

	/** Lifts the paper's edge a little, twice, as if about to turn it over; a press stops it at once. */
	function wobble(): void {
		if (sheet.solved || turning || folding) return wait();
		writeJson(WOBBLES, (readJson<number>(WOBBLES) ?? 0) + 1);
		const base = sheet.pos;
		const begin = performance.now();
		let raf = 0;
		const tick = (now: number) => {
			const k = Math.min(1, (now - begin) / WOBBLE);
			sheet.pos = squeeze(base, { x: 1, y: 0 }, LIFT * Math.abs(Math.sin(2 * Math.PI * k)));
			if (k < 1) raf = requestAnimationFrame(tick);
			else {
				stop();
				wait();
			}
		};
		const stop = () => {
			cancelAnimationFrame(raf);
			sheet.pos = base;
			wobbling = undefined;
		};
		wobbling = stop;
		raf = requestAnimationFrame(tick);
	}

	/** The view that centers the paper's extremes, shrunk only when its widest side would pass FIT. */
	function framing(): { x: number; y: number; s: number } {
		const [xs, ys] = [sheet.pos.map((p) => p.x), sheet.pos.map((p) => p.y)];
		const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
		return { x: (x0 + x1) / 2, y: (y0 + y1) / 2, s: Math.min(1, (2 * FIT) / Math.max(x1 - x0, y1 - y0)) };
	}

	/** Eases the view toward the framing, held still mid-turn, mid-fold and mid-wobble. */
	function reframe(dt: number): void {
		if (turning || folding || wobbling) return;
		const want = framing();
		const k = 1 - Math.exp(-dt / FOLLOW);
		const next = { x: view.x + (want.x - view.x) * k, y: view.y + (want.y - view.y) * k, s: view.s + (want.s - view.s) * k };
		if (Math.abs(next.x - view.x) + Math.abs(next.y - view.y) + Math.abs(next.s - view.s) < 1e-6) return;
		view = next;
	}

	function onpointerdown(e: PointerEvent): void {
		if (sheet.solved || e.button > 0 || turning) return;
		clearTimeout(idle);
		wobbling?.();
		settling?.();
		if (sheet.solved) return;
		e.preventDefault();
		svg!.setPointerCapture(e.pointerId);
		turning = { from: svgPoint(svg!, e), base: sheet.pos, under: sheet.stack, dir: { x: 1, y: 0 }, t: 0 };
	}

	/** Folds or unfolds a crease, its smaller side swinging up and over the rest, or its larger if the smaller can't. */
	function swing(c: Crease, moving: number[]): void {
		sheet.moves++;
		folding = true;
		settling = play(
			SWING,
			c,
			moving,
			sheet.pos,
			sheet.flaps,
			sheet.stack,
			(pos, stack) => ([sheet.pos, sheet.stack] = [pos, stack]),
			() => {
				settling = undefined;
				folding = false;
				sheet.settled();
				wait();
			}
		);
	}

	/** Shows the paper turned t of the way over, flipping the stack once it passes edge-on. */
	function show(t: number): void {
		const { base, under, dir } = turning!;
		turning!.t = t;
		sheet.pos = squeeze(base, dir, t);
		sheet.stack = t > 0.5 ? [...under].reverse() : under;
	}

	function onpointermove(e: PointerEvent): void {
		if (!turning || turning.falling) return;
		const p = svgPoint(svg!, e);
		const [dx, dy] = [p.x - turning.from.x, p.y - turning.from.y];
		const len = Math.hypot(dx, dy);
		if (!turning.moved && len < NUDGE) return;
		turning.moved = true;
		turning.dir = { x: dx / len, y: dy / len };
		show(Math.min(1, len / (width(turning.base, turning.dir) * view.s)));
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
			else {
				turning = undefined;
				wait();
			}
		};
		requestAnimationFrame(tick);
	}

	function onpointerup(): void {
		if (!turning) return;
		if (turning.moved) return letGo();
		const at = toWorld(turning.from);
		turning = undefined;
		wait();
		const did = tap(at, TAP / view.s, sheet.creases, sheet.pos, sheet.flaps, sheet.stack);
		if (did && 'inWay' in did) blink.show(did.inWay);
		else if (did) swing(did.c, did.moving);
	}
</script>

<div class="board">
	<svg
		bind:this={svg}
		viewBox="-1.25 -1.25 2.5 2.5"
		class:turning={turning?.moved}
		role="application"
		aria-label={t().board(sheet.moves)}
		{onpointerdown}
		{onpointermove}
		{onpointerup}
		onpointercancel={onpointerup}
	>
		<g class="paper">
			<Paper
				flaps={sheet.stack.map((k) => sheet.flaps[k])}
				faceUp={sheet.stack.map((k) => sheet.faceUp[k])}
				pos={shown}
				colors={PAPER}
				depth={!sheet.solved}
				{pulse}
				lit={blink.places(sheet.stack)}
				id="board-{uid}"
			/>
		</g>
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
	.turning {
		cursor: grabbing;
	}
</style>
