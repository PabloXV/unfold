<script lang="ts">
	import { fit } from './geometry';
	import { t } from './i18n/index.svelte';
	import Paper from './Paper.svelte';
	import type { Sheet } from './sheet.svelte';

	let { sheet }: { sheet: Sheet } = $props();

	const PAPER = { front: 'var(--paper-front)', back: 'var(--paper-back)', cast: 'var(--cast)', castStep: 'var(--cast-step)' };
	const uid = $props.id();
	const pos = $derived(fit(sheet.target.pos));
</script>

<figure aria-label={t().target}>
	<svg viewBox="-1.08 -1.08 2.16 2.16">
		<g class="paper">
			<Paper flaps={sheet.target.stack.map((k) => sheet.flaps[k])} faceUp={sheet.target.stack.map((k) => sheet.faceUp[k])} {pos} colors={PAPER} depth id="target-{uid}" />
		</g>
	</svg>
</figure>

<style>
	figure {
		margin: 0;
		width: 38%;
	}
	svg {
		display: block;
		width: 100%;
		aspect-ratio: 1;
		overflow: visible;
	}
	.paper {
		filter: drop-shadow(0.012px 0.024px 0.024px var(--shadow));
	}
</style>
