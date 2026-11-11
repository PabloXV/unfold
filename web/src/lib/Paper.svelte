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
	{/each}
{/if}
