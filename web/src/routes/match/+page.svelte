<script lang="ts">
	import { onMount } from 'svelte';
	import Board from '#lib/Board.svelte';
	import Icon from '#lib/Icon.svelte';
	import Paper from '#lib/Paper.svelte';
	import { fit } from '#lib/geometry.ts';
	import { Match } from '#lib/match.svelte.ts';
	import { dailySeed, loadCore, paperColors, today } from '#lib/unfold.ts';

	/** The levels to try: a sheet by seed and size, and how many folds its target takes. */
	const LEVELS: [seed: number, level: number, folds: number][] = [
		[0x2023, 4, 2],
		[0x3f12, 4, 2],
		[0x5e1b, 6, 3],
		[0x9c06, 7, 3],
		[0x168, 8, 4]
	];
	const PAPER = { front: 'var(--paper-front)', back: 'var(--paper-back)', cast: 'var(--cast)', castStep: 'var(--cast-step)' };

	let match = $state.raw<Match | undefined>(undefined);
	let n = $state(0);

	const shown = $derived(match ? fit(match.target.pos).map((p) => ({ x: p.x * 0.9, y: p.y * 0.9 })) : []);

	onMount(() => {
		const onhash = () => load(Math.min(LEVELS.length, Math.max(1, Number(location.hash.slice(1)) || 1)));
		loadCore().then(() => {
			const [front, back] = paperColors(dailySeed(today()));
			document.documentElement.style.setProperty('--paper-front', front);
			document.documentElement.style.setProperty('--paper-back', back);
			onhash();
		});
		if (import.meta.env.DEV) Object.defineProperty(window, '__unfold', { get: () => match, configurable: true });
		addEventListener('hashchange', onhash);
		return () => removeEventListener('hashchange', onhash);
	});

	function load(at: number): void {
		n = at;
		match = new Match(...LEVELS[at - 1]);
		history.replaceState(null, '', `#${at}`);
	}

	const plural = (k: number) => `${k} ${k === 1 ? 'fold' : 'folds'}`;
</script>

<main>
	<h1>unfold <span>match</span></h1>
	{#if match}
		<p class="level">level {n} of {LEVELS.length} · target in {plural(match.folds)}</p>
		<figure class="target" aria-label="The shape to match">
			<svg viewBox="-1.05 -1.05 2.1 2.1">
				<g class="paper">
					<Paper flaps={match.target.stack.map((k) => match!.flaps[k])} faceUp={match.target.stack.map((k) => match!.faceUp[k])} pos={shown} colors={PAPER} depth id="target" />
				</g>
			</svg>
			<figcaption>match this</figcaption>
		</figure>
		<div class="board">
			{#key match}
				<Board sheet={match} />
			{/key}
		</div>
		<p class="status">{match.solved ? `Matched in ${plural(match.moves)}` : plural(match.moves)}</p>
		<nav>
			<button onclick={() => load(n - 1)} disabled={n <= 1} aria-label="Previous level">‹</button>
			<button class="icon" onclick={() => load(n)} aria-label="Restart"><Icon name="reset" /></button>
			<button onclick={() => load(n + 1)} disabled={n >= LEVELS.length} aria-label="Next level">›</button>
		</nav>
		<p class="how">Fold the flat paper until it looks like the target, same flaps on top and same sides up. Tap a crease to fold it; drag to turn the paper over.</p>
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
		gap: 0.6rem;
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
	.target {
		margin: 0;
		width: min(42%, 22svh);
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.2rem;
	}
	.target svg {
		display: block;
		width: 100%;
		aspect-ratio: 1;
		overflow: visible;
	}
	.target .paper {
		filter: drop-shadow(0.012px 0.024px 0.024px var(--shadow));
	}
	figcaption {
		color: var(--muted);
		font-size: 0.8rem;
	}
	.status {
		margin: 0;
	}
	.board {
		width: min(100%, 46svh);
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
