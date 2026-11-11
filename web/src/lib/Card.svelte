<!--
	The share card: the day's finished shape on a postcard with the score. It's also where the
	shape is reshaped before sharing: with `editing`, its corners can be dragged anywhere, and a
	flap pulled over shows the paper's other side. Drawn as one <svg> with its own colors, so it
	exports to a PNG looking exactly as it does here, shadows and all.
-->
<script lang="ts">
	import Paper from './Paper.svelte';
	import { SIZE, squares, total, type Result } from './card';
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
		/** Where each corner is, in board units; reshaping replaces it. */
		corners: Point[];
		host: string;
		editing?: boolean;
		/** The corners after each reshaping drag. */
		onchange?: (corners: Point[]) => void;
	} = $props();

	let svg = $state<SVGSVGElement | undefined>(undefined);

	/** The paper's middle and size on the card, fixed so a dragged corner doesn't rescale it. */
	const CX = SIZE / 2;
	const CY = 500;
	const SCALE = 280;
	/** How far from the middle a corner can be dragged, in board units, keeping it on the card. */
	const REACH = 1.15;
	/** How near a press has to be to a corner to drag it, in board units. */
	const GRAB = 0.22;

	// colors written out, since the exported image can't see the page's CSS
	const css = typeof document === 'undefined' ? undefined : getComputedStyle(document.documentElement);
	const read = (name: string, fallback: string) => css?.getPropertyValue(name).trim() || fallback;
	const PAPER = {
		front: read('--paper-front', '#d9473f'),
		back: read('--paper-back', '#f2c14e'),
		cast: 'rgb(60 45 25)',
		castStep: '0.032'
	};
	const INK = '#2a2723';
	const MUTED = '#7a736a';
	const FONT = 'ui-rounded, "SF Pro Rounded", system-ui, -apple-system, "Segoe UI", sans-serif';
	const SQUARE_COLORS: Record<string, string> = { '🟩': '#2f7d6d', '🟨': '#d9a23a', '⬜': '#d9d2c5' };

	// where the corners are drawn: the prop, then each drag, until the prop changes again
	let pos = $state.raw<Point[]>([]);
	$effect.pre(() => {
		pos = corners;
	});
	let dragging = $state<number | undefined>(undefined);

	const marks = $derived([...squares(result)]);
	const SQ = 56;
	const GAP = 16;

	/** The whole sheet as one path, for the shadow it casts on the card. */
	const silhouette = $derived(
		flaps
			.map((f) => {
				const [a, b, c] = f.map((v) => pos[v]);
				return `M${a.x} ${-a.y}L${b.x} ${-b.y}L${c.x} ${-c.y}Z`;
			})
			.join('')
	);

	function cardPoint(e: PointerEvent): Point {
		const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(svg!.getScreenCTM()!.inverse());
		return { x: (p.x - CX) / SCALE, y: -(p.y - CY) / SCALE };
	}

	function onpointerdown(e: PointerEvent): void {
		if (!editing || e.button > 0) return;
		const p = cardPoint(e);
		let best: number | undefined;
		let bestDist = GRAB;
		pos.forEach((q, v) => {
			const d = Math.hypot(q.x - p.x, q.y - p.y);
			if (d < bestDist) [best, bestDist] = [v, d];
		});
		if (best === undefined) return;
		e.preventDefault();
		svg!.setPointerCapture(e.pointerId);
		dragging = best;
	}

	function onpointermove(e: PointerEvent): void {
		if (dragging === undefined) return;
		const p = cardPoint(e);
		const r = Math.hypot(p.x, p.y);
		const k = r > REACH ? REACH / r : 1;
		pos = pos.map((q, v) => (v === dragging ? { x: p.x * k, y: p.y * k } : q));
	}

	function onpointerup(): void {
		if (dragging === undefined) return;
		dragging = undefined;
		onchange?.(pos);
	}

	/** The card's <svg>, for exporting. */
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
		<!-- the sheet's shadow on the card: drawn once, so a blur is fine here -->
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
