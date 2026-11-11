<script lang="ts" module>
	export type PaperColors = { front: string; back: string; cast: string; castStep: string };
</script>

<script lang="ts">
	import { trianglePath, turn } from './geometry';
	import type { Point } from './unfold';

	let {
		flaps,
		faceUp,
		pos,
		colors,
		depth = false,
		creaseWidth,
		id
	}: {
		/** Vertex triples, bottom to top. */
		flaps: number[][];
		faceUp: number[];
		pos: Point[];
		/** Plain values or var()s; an exported image needs plain values. */
		colors: PaperColors;
		/** Deepens the lower flaps a little, as stacked sheets look. */
		depth?: boolean;
		/** Crease width in board units, scaling with the sheet; one screen pixel without. */
		creaseWidth?: number;
		/** Unique per sheet on the page, for its clip paths. */
		id: string;
	} = $props();

	/** A blur-free shadow: the casting shape drawn faintly eight times, each a little further off. */
	const CAST_STEPS = Array.from({ length: 8 }, (_, i) => ({ step: i, x: 0.017 * ((i + 1) / 8), y: 0.036 * ((i + 1) / 8) }));

	const flapPaths = $derived(pos.length ? flaps.map((f) => trianglePath(f.map((v) => pos[v]))) : []);

	/** Per flap, every flap above it as one path: the shape of the shadow cast onto it. */
	const above = $derived.by(() => {
		const out: string[] = new Array(flapPaths.length).fill('');
		for (let k = flapPaths.length - 2; k >= 0; k--) out[k] = out[k + 1] + flapPaths[k + 1];
		return out;
	});

	/** How far a crease's highlight sits from its line, toward the light at the top left. */
	const HIGHLIGHT = 0.007;

	/** Each fold between two flaps, drawn on the higher one: a dark line, and a light one beside it. */
	const folds = $derived.by(() => {
		const owners = new Map<string, number[]>();
		flaps.forEach((f, k) => {
			for (const [a, b] of [[f[0], f[1]], [f[1], f[2]], [f[2], f[0]]]) {
				const key = a < b ? `${a},${b}` : `${b},${a}`;
				owners.set(key, [...(owners.get(key) ?? []), k]);
			}
		});
		const on: { a: number; b: number }[][] = flaps.map(() => []);
		for (const [key, ks] of owners) {
			if (ks.length !== 2) continue;
			const [a, b] = key.split(',').map(Number);
			on[Math.max(...ks)].push({ a, b });
		}
		return on;
	});

	/** A fold's ends in svg coordinates, and its highlight's, shifted toward the top left. */
	function crease({ a, b }: { a: number; b: number }) {
		const [p, q] = [pos[a], pos[b]];
		const len = Math.hypot(q.x - p.x, q.y - p.y) || 1;
		let [nx, ny] = [(q.y - p.y) / len, (q.x - p.x) / len];
		if (nx + ny > 0) [nx, ny] = [-nx, -ny];
		const [dx, dy] = [nx * HIGHLIGHT, ny * HIGHLIGHT];
		return { x1: p.x, y1: -p.y, x2: q.x, y2: -q.y, hx1: p.x + dx, hy1: -p.y + dy, hx2: q.x + dx, hy2: -q.y + dy };
	}

	/** The front while a flap runs the same way round as on the flat sheet, else the back. */
	function fill(k: number): string {
		const color = Math.sign(turn(flaps[k].map((v) => pos[v]))) === faceUp[k] ? colors.front : colors.back;
		if (!depth) return color;
		const deep = flaps.length > 1 ? 1 - k / (flaps.length - 1) : 0;
		return `color-mix(in oklab, ${color} ${100 - Math.round(deep * 14)}%, black)`;
	}
</script>

{#if pos.length}
	<defs>
		{#each flapPaths as d, k (k)}
			<clipPath id="{id}-flap-{k}"><path {d} /></clipPath>
			{#if above[k]}<path id="{id}-above-{k}" d={above[k]} />{/if}
		{/each}
	</defs>
	{#each flapPaths as d, k (k)}
		<path {d} style:fill={fill(k)} />
		{#if above[k]}
			<g clip-path="url(#{id}-flap-{k})">
				{#each CAST_STEPS as s (s.step)}
					<use href="#{id}-above-{k}" transform="translate({s.x} {s.y})" style:fill={colors.cast} style:fill-opacity={colors.castStep} />
				{/each}
			</g>
		{/if}
		{#each folds[k] as fold (`${fold.a},${fold.b}`)}
			{@const c = crease(fold)}
			<line
				x1={c.hx1}
				y1={c.hy1}
				x2={c.hx2}
				y2={c.hy2}
				stroke="#ffffff"
				stroke-opacity="0.45"
				stroke-width={creaseWidth ? creaseWidth * 0.85 : 1}
				vector-effect={creaseWidth ? undefined : 'non-scaling-stroke'}
			/>
			<line
				x1={c.x1}
				y1={c.y1}
				x2={c.x2}
				y2={c.y2}
				stroke="#000000"
				stroke-opacity="0.3"
				stroke-width={creaseWidth ?? 1.2}
				vector-effect={creaseWidth ? undefined : 'non-scaling-stroke'}
			/>
		{/each}
	{/each}
{/if}
