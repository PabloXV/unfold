<script lang="ts">
	import Paper from './Paper.svelte';
	import { FONT, SIZE, type Result } from './card';
	import { nearestTo, svgPoint, trianglePath } from './geometry';
	import { overlaps, restack } from './stacking';
	import type { Point } from './unfold';

	let {
		result,
		flaps,
		faceUp,
		corners,
		host,
		editing = false,
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
		/** Lets the corners be dragged anywhere. */
		editing?: boolean;
		onchange?: (corners: Point[], order: number[]) => void;
	} = $props();

	/** The paper's middle and scale on the card, fixed so a dragged corner doesn't rescale it. */
	const CX = SIZE / 2;
	const CY = 500;
	const SCALE = 280;
	/** How far from the middle a corner can be dragged, and how near a press must be to grab one. */
	const REACH = 1.15;
	const GRAB = 0.22;
	const INK = '#2a2723';
	const MUTED = '#7a736a';

	const css = getComputedStyle(document.documentElement);
	const PAPER = {
		front: css.getPropertyValue('--paper-front').trim() || '#d9473f',
		back: css.getPropertyValue('--paper-back').trim() || '#f2c14e',
		cast: 'rgb(60 45 25)',
		castStep: '0.032'
	};

	let svg = $state<SVGSVGElement | undefined>(undefined);
	let pos = $state.raw<Point[]>([]);
	let stack = $state.raw<number[]>([]);
	let dragging = $state<number | undefined>(undefined);
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

	const silhouette = $derived(flaps.map((f) => trianglePath(f.map((v) => pos[v]))).join(''));

	function cardPoint(e: PointerEvent): Point {
		const p = svgPoint(svg!, e);
		return { x: (p.x - CX) / SCALE, y: (p.y + CY) / SCALE };
	}

	function onpointerdown(e: PointerEvent): void {
		if (!editing || e.button > 0) return;
		const v = nearestTo(pos, cardPoint(e), GRAB);
		if (v === undefined) return;
		dragging = v;
		moving = new Set(flaps.flatMap((f, k) => (f.includes(v) ? [k] : [])));
		overlapping = overlaps(flaps, pos);
		moved = false;
		e.preventDefault();
		svg!.setPointerCapture(e.pointerId);
	}

	function onpointermove(e: PointerEvent): void {
		if (dragging === undefined) return;
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
	class:editing
	viewBox="0 0 {SIZE} {SIZE}"
	role="img"
	aria-label="Share card: {result.date}, {result.moves} moves, par {result.par}"
	{onpointerdown}
	{onpointermove}
	{onpointerup}
	onpointercancel={onpointerup}
>
	<defs>
		<radialGradient id="card-spot" cx={CX} cy={CY} r={SIZE * 0.62} gradientUnits="userSpaceOnUse">
			<stop offset="0" stop-color="#fffdf8" />
			<stop offset="1" stop-color="#ebe4d6" />
		</radialGradient>
		<filter id="card-shadow" x="-25%" y="-25%" width="150%" height="150%">
			<feGaussianBlur in="SourceAlpha" stdDeviation="0.02" />
			<feOffset dx="0.02" dy="0.04" result="blur" />
			<feFlood flood-color="rgb(60, 45, 25)" flood-opacity="0.26" />
			<feComposite in2="blur" operator="in" />
		</filter>
	</defs>

	<rect width={SIZE} height={SIZE} fill="url(#card-spot)" />
	<text x="80" y="140" fill={INK} font-family={FONT} font-size="64" font-weight="700">unfold</text>
	<text x={SIZE - 80} y="140" fill={MUTED} font-family={FONT} font-size="40" text-anchor="end">{result.date}</text>

	<g transform="translate({CX} {CY}) scale({SCALE})">
		<g filter="url(#card-shadow)"><path d={silhouette} fill="#000" /></g>
		<Paper flaps={stack.map((k) => flaps[k])} faceUp={stack.map((k) => faceUp[k])} {pos} colors={PAPER} creaseWidth={0.009} id="card" />
		{#if editing}
			{#each pos as p, v (v)}
				<circle class="handle no-export" class:held={dragging === v} cx={p.x} cy={-p.y} r="0.06" />
			{/each}
		{/if}
	</g>

	<text x={CX} y="960" fill={INK} font-family={FONT} font-size="48" font-weight="700" text-anchor="middle">
		{result.moves} moves · par {result.par}
	</text>
	<text x={CX} y="1024" fill={MUTED} font-family={FONT} font-size="30" text-anchor="middle">{host}</text>
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
	.editing {
		cursor: crosshair;
	}
	.handle {
		fill: none;
		stroke: var(--dot);
		stroke-width: 2px;
		stroke-dasharray: 4 3;
		vector-effect: non-scaling-stroke;
	}
	.handle.held {
		stroke: var(--carry);
		stroke-width: 3px;
	}
</style>
