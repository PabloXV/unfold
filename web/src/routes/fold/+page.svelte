<script lang="ts">
	import { onMount } from 'svelte';
	import FreeBoard from '#lib/FreeBoard.svelte';
	import Icon from '#lib/Icon.svelte';
	import { Sheet } from '#lib/sheet.svelte.ts';
	import { dailySeed, loadCore, paperColors, today } from '#lib/unfold.ts';

	let sheet = $state.raw<Sheet | undefined>(undefined);
	let seed = 0;

	onMount(() => {
		const onhash = () => load(Number(location.hash.slice(1)) || 3);
		loadCore().then(() => {
			seed = dailySeed(today());
			const [front, back] = paperColors(seed);
			document.documentElement.style.setProperty('--paper-front', front);
			document.documentElement.style.setProperty('--paper-back', back);
			onhash();
		});
		if (import.meta.env.DEV) Object.defineProperty(window, '__unfold', { get: () => sheet, configurable: true });
		addEventListener('hashchange', onhash);
		return () => removeEventListener('hashchange', onhash);
	});

	function load(level: number): void {
		if (sheet?.level === level) return;
		sheet = new Sheet(seed, level, 0, true);
		history.replaceState(null, '', `#${level}`);
	}

	const plural = (n: number) => `${n} ${n === 1 ? 'move' : 'moves'}`;
</script>

<main>
	<h1>unfold <span>fold</span></h1>
	{#if sheet}
		<p class="level">level {sheet.level} · solvable in {sheet.par}</p>
		<div class="board">
			{#key sheet}
				<FreeBoard {sheet} />
			{/key}
		</div>
		<p class="status">{sheet.solved ? `Flat in ${plural(sheet.moves)}` : plural(sheet.moves)}</p>
		<nav>
			<button onclick={() => load(sheet!.level - 1)} disabled={sheet.level <= 1} aria-label="Previous level">‹</button>
			<button class="icon" onclick={() => (sheet = new Sheet(seed, sheet!.level, 0, true))} aria-label="Restart"><Icon name="reset" /></button>
			<button onclick={() => load(sheet!.level + 1)} aria-label="Next level">›</button>
		</nav>
		<p class="how">Drag a corner to fold the paper along a crease, everything on its side coming along. Fold it until it lies flat. Drag off the paper to turn it over.</p>
	{/if}
</main>

<style>
	main {
		max-width: 34rem;
		margin: 0 auto;
		padding: 1.25rem 1rem 2rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
	}
	h1 {
		margin: 0;
		font-size: 2.2rem;
	}
	h1 span {
		color: var(--muted);
		font-weight: 400;
		font-size: 1.2rem;
	}
	.level,
	.how {
		margin: 0;
		color: var(--muted);
		font-size: 0.9rem;
		text-align: center;
	}
	.status {
		margin: 0;
	}
	.board {
		width: min(100%, 62svh);
		background: radial-gradient(closest-side, var(--spot), transparent);
	}
	nav {
		display: flex;
		gap: 0.5rem;
	}
	button {
		font: inherit;
		color: var(--text);
		background: var(--button);
		border: 1px solid var(--line);
		border-radius: 999px;
		min-width: 2.75rem;
		height: 2.75rem;
		cursor: pointer;
	}
	button:disabled {
		opacity: 0.4;
		cursor: default;
	}
	button.icon {
		display: grid;
		place-items: center;
		padding: 0;
	}
</style>
