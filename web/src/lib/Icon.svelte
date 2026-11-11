<script lang="ts">
	let { name }: { name: 'pencil' | 'reset' | 'share' | 'link' | 'help' | 'close' } = $props();
</script>

<svg viewBox="0 0 24 24" aria-hidden="true">
	{#if name === 'pencil'}
		<path d="M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16v4Z" />
	{:else if name === 'reset'}
		<path d="M4.5 12a7.5 7.5 0 1 0 2.2-5.3M4 4v5h5" />
	{:else if name === 'share'}
		<circle cx="18" cy="5" r="2.5" /><circle cx="6" cy="12" r="2.5" /><circle cx="18" cy="19" r="2.5" /><path d="M8.2 10.8l7.6-4.6M8.2 13.2l7.6 4.6" />
	{:else if name === 'link'}
		<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1" />
	{:else if name === 'help'}
		<circle cx="12" cy="12" r="9" /><path d="M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6V14" /><circle cx="12" cy="17.2" r="0.6" />
	{:else}
		<path d="M6 6l12 12M18 6 6 18" />
	{/if}
</svg>

<style>
	svg {
		display: block;
		width: 1.35rem;
		height: 1.35rem;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.8;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
