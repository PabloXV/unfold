<script lang="ts">
	import Paper from './Paper.svelte';
	import { SIZE, squares, total, type Result } from './card';
	import { nearestTo, svgPoint, trianglePath } from './geometry';
	import type { Point } from './unfold';

	let {
		result,
		flaps,
		faceUp,
		corners,
		host,
		editing = false,
		onchange
	}: {
		result: Result;
		flaps: number[][];
		faceUp: number[];
		/** Where each corner is, in board units. */
		corners: Point[];
		host: string;
		/** Lets the corners be dragged anywhere. */
		editing?: boolean;
		onchange?: (corners: Point[]) => void;
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
	const FONT = 'ui-rounded, "SF Pro Rounded", system-ui, -apple-system, "Segoe UI", sans-serif';
	const SQUARE_COLORS: Record<string, string> = { '🟩': '#2f7d6d', '🟨': '#d9a23a', '⬜': '#d9d2c5' };
	const SQ = 56;
	const GAP = 16;

	const css = getComputedStyle(document.documentElement);
	const PAPER = {
		front: css.getPropertyValue('--paper-front').trim() || '#d9473f',
		back: css.getPropertyValue('--paper-back').trim() || '#f2c14e',
		cast: 'rgb(60 45 25)',
		castStep: '0.032'
	};

	let svg = $state<SVGSVGElement | undefined>(undefined);
	let pos = $state.raw<Point[]>([]);
	let dragging = $state<number | undefined>(undefined);

	$effect.pre(() => {
		pos = corners;
	});

	const marks = $derived([...squares(result)]);
	const silhouette = $derived(flaps.map((f) => trianglePath(f.map((v) => pos[v]))).join(''));

	function cardPoint(e: PointerEvent): Point {
		const p = svgPoint(svg!, e);
		return { x: (p.x - CX) / SCALE, y: (p.y + CY) / SCALE };
	}

	function onpointerdown(e: PointerEvent): void {
		if (!editing || e.button > 0) return;
		dragging = nearestTo(pos, cardPoint(e), GRAB);
		if (dragging === undefined) return;
		e.preventDefault();
		svg!.setPointerCapture(e.pointerId);
	}

	function onpointermove(e: PointerEvent): void {
		if (dragging === undefined) return;
		const p = cardPoint(e);
		const k = Math.min(1, REACH / Math.hypot(p.x, p.y));
		pos = pos.map((q, v) => (v === dragging ? { x: p.x * k, y: p.y * k } : q));
	}

	function onpointerup(): void {
		if (dragging === undefined) return;
		dragging = undefined;
		onchange?.(pos);
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
	aria-label="Share card: {result.date}, {total(result.moves)} moves, par {total(result.pars)}"
	{onpointerdown}
	{onpointermove}
	{onpointerup}
	onpointercancel={onpointerup}
>
	<defs>
		<filter id="card-shadow" x="-25%" y="-25%" width="150%" height="150%">
			<feGaussianBlur in="SourceAlpha" stdDeviation="0.02" />
			<feOffset dx="0.02" dy="0.04" result="blur" />
			<feFlood flood-color="rgb(60, 45, 25)" flood-opacity="0.26" />
			<feComposite in2="blur" operator="in" />
		</filter>
	</defs>

	<rect width={SIZE} height={SIZE} fill="#f4f0e8" />
	<text x="80" y="140" fill={INK} font-family={FONT} font-size="64" font-weight="600">unfold</text>
	<text x={SIZE - 80} y="140" fill={MUTED} font-family={FONT} font-size="40" text-anchor="end">{result.date}</text>

	<g transform="translate({CX} {CY}) scale({SCALE})">
		<g filter="url(#card-shadow)"><path d={silhouette} fill="#000" /></g>
		<Paper {flaps} {faceUp} {pos} colors={PAPER} id="card" />
		{#if editing}
			{#each pos as p, v (v)}
				<circle class="handle no-export" class:held={dragging === v} cx={p.x} cy={-p.y} r="0.06" />
			{/each}
		{/if}
	</g>

	{#each marks as m, i (i)}
		<rect x={CX - (marks.length * SQ + (marks.length - 1) * GAP) / 2 + i * (SQ + GAP)} y="860" width={SQ} height={SQ} rx="12" fill={SQUARE_COLORS[m]} />
	{/each}
	<text x={CX} y="976" fill={INK} font-family={FONT} font-size="44" font-weight="600" text-anchor="middle">
		{total(result.moves)} moves · par {total(result.pars)}
	</text>
	<text x={CX} y="1024" fill={MUTED} font-family={FONT} font-size="30" text-anchor="middle">{host}</text>
</svg>

<style>
	.card {
		display: block;
		width: 100%;
		height: auto;
		border-radius: 1rem;
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
