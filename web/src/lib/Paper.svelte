<!--
	A sheet of two-sided paper folded into flaps, for inside an <svg>, in board units (y up, the
	solved shape about a unit across). Flaps are drawn bottom to top, each followed by the shadow
	everything above it casts on it, so higher paper hides what's under it with no further work.
	Shared by the board and the share card, so both look the same.
-->
<script lang="ts">
	import { turn } from './game.svelte';
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
		/** Per flap, which way round its corners run on the flat sheet, as in Game. */
		faceUp: number[];
		/** Where each vertex is now. */
		pos: Point[];
		/** CSS colors: plain values or var()s, both work, except in an exported image. */
		colors: { front: string; back: string; cast: string; castStep: string };
		/** Deepen the lower flaps a little, as stacked sheets look. */
		depth?: boolean;
		/** Unique per sheet on the page, for its clip paths. */
		id: string;
	} = $props();

	/**
	 * A shadow without blur filters, which Safari draws slowly: the casting shape eight times over,
	 * each very faint and a little further off, layers into a smooth fade. Each copy is one path, so
	 * overlapping casters never shade darker than one.
	 */
	const CAST_STEPS = Array.from({ length: 8 }, (_, i) => ({ step: i, x: 0.017 * ((i + 1) / 8), y: 0.036 * ((i + 1) / 8) }));

	/**
	 * Each flap as a closed path, always wound the same way round, so that many of them filled as
	 * one path cover their union instead of cancelling out where they overlap.
	 */
	const flapPaths = $derived(
		pos.length === 0
			? []
			: flaps.map((f) => {
					let [a, b, c] = f.map((v) => ({ x: pos[v].x, y: -pos[v].y }));
					if ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x) < 0) [b, c] = [c, b];
					return `M${a.x} ${a.y}L${b.x} ${b.y}L${c.x} ${c.y}Z`;
				})
	);

	/** Per flap, every flap above it as one path: the shape of the shadow cast onto it. */
	const above = $derived.by(() => {
		const out: string[] = new Array(flapPaths.length).fill('');
		for (let k = flapPaths.length - 2; k >= 0; k--) out[k] = out[k + 1] + flapPaths[k + 1];
		return out;
	});

	/**
	 * The paper's front while a flap's corners run the same way round as on the flat sheet, its
	 * back once one corner has crossed the opposite edge and turned it over.
	 */
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
			<!-- defined once and reused by every step, so moving dots rewrites one shape per flap -->
			{#if above[k]}<path id="{id}-above-{k}" d={above[k]} />{/if}
		{/each}
	</defs>
	{#each flapPaths as d, k (k)}
		<path class="flap" {d} style:fill={fill(k)} />
		<!-- the shadow everything above this flap casts on it, only on it -->
		{#if above[k]}
			<g clip-path="url(#{id}-flap-{k})">
				{#each CAST_STEPS as s (s.step)}
					<use href="#{id}-above-{k}" transform="translate({s.x} {s.y})" style:fill={colors.cast} style:fill-opacity={colors.castStep} />
				{/each}
			</g>
		{/if}
	{/each}
{/if}

<style>
	.flap {
		/* quick, so a flap turning over reads as a flip, not a fade */
		transition: fill 0.2s;
	}
</style>
