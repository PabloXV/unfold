<!-- A solved level's picture, small, the way up it was drawn. -->
<script lang="ts">
	import type { Level } from './mystery';

	let { level }: { level: Level } = $props();

	const xs = $derived(level.corners.filter((_, i) => i % 2 === 0));
	const ys = $derived(level.corners.filter((_, i) => i % 2 === 1));
	const box = $derived.by(() => {
		const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
		const size = Math.max(x1 - x0, y1 - y0) * 1.1;
		return `${(x0 + x1) / 2 - size / 2} ${-(y0 + y1) / 2 - size / 2} ${size} ${size}`;
	});
	const triangles = $derived(
		level.colors.map((color, i) => ({
			color,
			points: level.triangles
				.slice(3 * i, 3 * i + 3)
				.map((v) => `${xs[v]},${-ys[v]}`)
				.join(' ')
		}))
	);
</script>

<svg viewBox={box} aria-hidden="true">
	{#each triangles as t, i (i)}
		<polygon points={t.points} fill={t.color} stroke={t.color} stroke-width="0.6" stroke-linejoin="round" />
	{/each}
</svg>

<style>
	svg {
		display: block;
		width: 100%;
		height: 100%;
	}
</style>
