<script lang="ts">
	import { untrack } from 'svelte';
	import { FIRE } from './Icon.svelte';
	import Paper from './Paper.svelte';
	import { FONT, SIZE, score, type Result } from './card';
	import { nearestTo, svgPoint, trianglePath } from './geometry';
	import { overlaps, restack } from './stacking';
	import { CAST, DEFAULT_PAPER, LIGHT } from './theme';
	import type { Point } from './unfold';

	let {
		result,
		flaps,
		faceUp,
		corners,
		host,
		hint = false,
		order,
		onchange
	}: {
		result: Result;
		flaps: number[][];
		faceUp: number[];
		/** Where each corner is, in board units. */
		corners: Point[];
		/** Flap indices bottom to top. */
		order: number[];
		host: string;
		/** Nudges a corner once, to show the paper can be reshaped. */
		hint?: boolean;
		onchange?: (corners: Point[], order: number[]) => void;
	} = $props();

	/** The paper's middle and scale on the card, fixed so a dragged corner doesn't rescale it. */
	const CX = SIZE / 2;
	const CY = 500;
	const SCALE = 280;
	/** How far from the middle a corner can be dragged, and how near a press must be to grab one. */
	const REACH = 1.15;
	const GRAB = 0.22;
	/** The result line's size, and Courier Prime's advance as a fraction of it, being monospaced. */
	const LINE = 48;
	const ADVANCE = 0.6;
	const FLAME = 46;
	const HINT_DELAY = 700;
	const HINT_LENGTH = 1100;
	const HINT_REACH = 0.18;

	const css = getComputedStyle(document.documentElement);
	const PAPER = {
		front: css.getPropertyValue('--paper-front').trim() || DEFAULT_PAPER.front,
		back: css.getPropertyValue('--paper-back').trim() || DEFAULT_PAPER.back,
		cast: CAST,
		castStep: '0.032'
	};

	let svg = $state<SVGSVGElement | undefined>(undefined);
	let pos = $state.raw<Point[]>([]);
	let stack = $state.raw<number[]>([]);
	let dragging = $state<number | undefined>(undefined);
	let hovered = $state<number | undefined>(undefined);
	/** The flaps on the dragged corner, which flaps overlapped at the last step, and whether it moved. */
	let moving = new Set<number>();
	let overlapping = new Set<string>();
	let moved = false;

	$effect.pre(() => {
		pos = corners;
	});

	$effect.pre(() => {
		stack = order;
	});

	/** Pulls the corner furthest up and right toward the middle and lets it settle back. */
	$effect(() => {
		if (!hint || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
		let frame = 0;
		const start = untrack(() => corners);
		const timer = setTimeout(() => {
			const v = start.reduce((best, p, i) => (p.x + p.y > start[best].x + start[best].y ? i : best), 0);
			const home = start[v];
			const begun = performance.now();
			const step = (now: number) => {
				if (dragging !== undefined) return;
				const t = Math.min(1, (now - begun) / HINT_LENGTH);
				const pull = HINT_REACH * Math.sin(Math.PI * t) ** 2;
				pos = pos.map((q, i) => (i === v ? { x: home.x * (1 - pull), y: home.y * (1 - pull) } : q));
				if (t < 1) frame = requestAnimationFrame(step);
			};
			frame = requestAnimationFrame(step);
		}, HINT_DELAY);
		return () => {
			clearTimeout(timer);
			cancelAnimationFrame(frame);
		};
	});

	/** The result line, then the streak's flame and count, laid out as one centered row. */
	const line = $derived.by(() => {
		const text = result.streak ? `${score(result.moves, result.par)} · ` : score(result.moves, result.par);
		const count = result.streak ? String(result.streak) : '';
		const width = (text.length + count.length) * LINE * ADVANCE + (count ? FLAME : 0);
		const x = CX - width / 2;
		const flameX = x + text.length * LINE * ADVANCE;
		return { text, count, x, flameX, countX: flameX + FLAME };
	});

	const silhouette = $derived(flaps.map((f) => trianglePath(f.map((v) => pos[v]))).join(''));

	function cardPoint(e: PointerEvent): Point {
		const p = svgPoint(svg!, e);
		return { x: (p.x - CX) / SCALE, y: (p.y + CY) / SCALE };
	}

	function onpointerdown(e: PointerEvent): void {
		if (e.button > 0) return;
		const v = nearestTo(pos, cardPoint(e), GRAB);
		if (v === undefined) return;
		pos = corners;
		dragging = v;
		moving = new Set(flaps.flatMap((f, k) => (f.includes(v) ? [k] : [])));
		overlapping = overlaps(flaps, pos);
		moved = false;
		e.preventDefault();
		svg!.setPointerCapture(e.pointerId);
	}

	function onpointermove(e: PointerEvent): void {
		if (dragging === undefined) {
			hovered = nearestTo(pos, cardPoint(e), GRAB);
			return;
		}
		const p = cardPoint(e);
		const k = Math.min(1, REACH / Math.hypot(p.x, p.y));
		pos = pos.map((q, v) => (v === dragging ? { x: p.x * k, y: p.y * k } : q));
		const now = overlaps(flaps, pos);
		stack = restack(stack, moving, overlapping, now);
		overlapping = now;
		moved = true;
	}

	function onpointerup(): void {
		if (dragging === undefined) return;
		dragging = undefined;
		if (moved) onchange?.(pos, stack);
	}

	export function element(): SVGSVGElement | undefined {
		return svg;
	}
</script>

<svg
	bind:this={svg}
	class="card"
	class:grab={hovered !== undefined}
	class:grabbing={dragging !== undefined}
	viewBox="0 0 {SIZE} {SIZE}"
	role="img"
	aria-label="Share card: {result.date}, {score(result.moves, result.par)}{result.streak ? `, ${result.streak} day streak` : ''}"
	{onpointerdown}
	{onpointermove}
	{onpointerup}
	onpointercancel={onpointerup}
	onpointerleave={() => (hovered = undefined)}
>
	<defs>
		<radialGradient id="card-spot" cx={CX} cy={CY} r={SIZE * 0.62} gradientUnits="userSpaceOnUse">
			<stop offset="0" stop-color={LIGHT.spot} />
			<stop offset="1" stop-color={LIGHT.bg} />
		</radialGradient>
		<filter id="card-shadow" x="-25%" y="-25%" width="150%" height="150%">
			<feGaussianBlur in="SourceAlpha" stdDeviation="0.02" />
			<feOffset dx="0.02" dy="0.04" result="blur" />
			<feFlood flood-color={CAST} flood-opacity="0.26" />
			<feComposite in2="blur" operator="in" />
		</filter>
	</defs>

	<rect width={SIZE} height={SIZE} fill="url(#card-spot)" />
	<text x="80" y="140" fill={LIGHT.text} font-family={FONT} font-size="64" font-weight="700">unfold</text>
	<text x={SIZE - 80} y="140" fill={LIGHT.muted} font-family={FONT} font-size="40" text-anchor="end">{result.date}</text>

	<g transform="translate({CX} {CY}) scale({SCALE})">
		<g filter="url(#card-shadow)"><path d={silhouette} fill="#000" /></g>
		<Paper flaps={stack.map((k) => flaps[k])} faceUp={stack.map((k) => faceUp[k])} {pos} colors={PAPER} creaseWidth={0.009} id="card" />
	</g>

	<g fill={LIGHT.text} font-family={FONT} font-size={LINE} font-weight="700">
		<text x={line.x} y="960">{line.text}</text>
		{#if line.count}
			<path d={FIRE} transform="translate({line.flameX} {960 - FLAME * 0.86}) scale({FLAME / 24})" />
			<text x={line.countX} y="960">{line.count}</text>
		{/if}
	</g>
	<text x={CX} y="1024" fill={LIGHT.muted} font-family={FONT} font-size="30" text-anchor="middle">{host}</text>
</svg>

<style>
	.card {
		display: block;
		width: 100%;
		height: auto;
		touch-action: none;
		user-select: none;
		-webkit-user-select: none;
	}
	.grab {
		cursor: grab;
	}
	.grabbing {
		cursor: grabbing;
	}
</style>
