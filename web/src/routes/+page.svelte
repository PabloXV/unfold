<script lang="ts">
	import { onMount } from 'svelte';
	import Board from '#lib/Board.svelte';
	import { Game } from '#lib/game.svelte.ts';
	import { dailySeed, formatDay, loadCore, parseCode, today } from '#lib/unfold.ts';
	import { firstUnsolved } from '#lib/scores.ts';

	const RESTART_TIME = 2000;
	const NOTICE_TIME = 1500;

	let game = $state.raw<Game | undefined>(undefined);
	/** Set when playing a day's seed. */
	let day = $state<number | undefined>(undefined);
	/** Only once the keyboard is used, so touch and mouse players never see a cursor. */
	let showCursor = $state(false);
	let restartAsked = $state(false);
	let notice = $state('');
	let failed = $state(false);
	/** Levels left with [ and ], to come back to as they were. */
	const shelf = new Map<number, Game>();
	let restartTimer: ReturnType<typeof setTimeout> | undefined;
	let noticeTimer: ReturnType<typeof setTimeout> | undefined;

	onMount(() => {
		loadCore().then(
			() => open(location.hash),
			() => (failed = true)
		);
		const onhash = () => {
			if (location.hash.slice(1) !== game?.code) open(location.hash);
		};
		addEventListener('hashchange', onhash);
		return () => removeEventListener('hashchange', onhash);
	});

	/** A hash like #ABC123/9 opens that puzzle; anything else, today's daily. */
	function open(hash: string): void {
		const code = parseCode(decodeURIComponent(hash.slice(1)));
		const now = today();
		let seed: number, level: number;
		if (code) {
			[seed, level] = [code[0], code[1]];
			// a pasted code from today's daily gets its date too
			day = seed === dailySeed(now) ? now : undefined;
		} else {
			[seed, day] = [dailySeed(now), now];
			level = firstUnsolved(seed);
		}
		for (const g of shelf.values()) g.free();
		shelf.clear();
		game?.free();
		game = new Game(seed, level);
		showUrl();
	}

	/** The daily keeps a clean address; any other seed shows its code, to bookmark or share. */
	function showUrl(): void {
		if (!game) return;
		const url = day === undefined ? `#${game.code}` : location.pathname;
		history.replaceState(history.state, '', url);
	}

	function jump(step: 1 | -1): void {
		if (!game || game.level + step < 1) return;
		const level = game.level + step;
		shelf.set(game.level, game);
		game = shelf.get(level) ?? new Game(game.seed, level);
		shelf.delete(level);
		restartAsked = false;
		showUrl();
	}

	/** Asks first when there's progress to lose. */
	function restart(): void {
		if (!game) return;
		if (game.inProgress && !restartAsked) {
			restartAsked = true;
			clearTimeout(restartTimer);
			restartTimer = setTimeout(() => (restartAsked = false), RESTART_TIME);
			return;
		}
		restartAsked = false;
		const { seed, level } = game;
		game.free();
		game = new Game(seed, level);
	}

	function flash(text: string): void {
		notice = text;
		clearTimeout(noticeTimer);
		noticeTimer = setTimeout(() => (notice = ''), NOTICE_TIME);
	}

	async function copyLink(): Promise<void> {
		if (!game) return;
		const url = `${location.origin}${location.pathname}#${game.code}`;
		try {
			await navigator.clipboard.writeText(url);
			flash('link copied');
		} catch {
			flash(url);
		}
	}

	function onkeydown(e: KeyboardEvent): void {
		if (!game || e.metaKey || e.ctrlKey || e.altKey) return;
		if (e.key !== 'r') restartAsked = false;
		const keys: Record<string, () => void> = {
			ArrowLeft: () => game!.step(1),
			h: () => game!.step(1),
			ArrowRight: () => game!.step(-1),
			l: () => game!.step(-1),
			' ': () => (game!.solved ? jump(1) : game!.select()),
			Enter: () => (game!.solved ? jump(1) : game!.select()),
			Escape: () => game!.putBack(),
			'[': () => jump(-1),
			']': () => jump(1),
			r: restart,
			c: copyLink
		};
		const act = keys[e.key];
		if (!act) return;
		e.preventDefault();
		if (['ArrowLeft', 'ArrowRight', 'h', 'l', ' ', 'Enter', 'Escape'].includes(e.key)) showCursor = true;
		act();
	}
</script>

<svelte:window {onkeydown} onpointerdown={() => (showCursor = false)} />

<svelte:head>
	<title>unfold</title>
	<meta name="description" content="A daily origami puzzle: move the dots around the circle until no creases cross, and watch the paper unfold." />
</svelte:head>

<main>
	<header>
		<h1>unfold</h1>
		{#if game}
			<p class="meta">
				{[day !== undefined && formatDay(day), `level ${game.level}`, game.best !== undefined && `best ${game.best}`]
					.filter(Boolean)
					.join(' · ')}
				<span class="code">{game.code}</span>
			</p>
		{/if}
	</header>

	<div class="board">
		{#if game}
			<Board {game} {showCursor} />
		{:else if failed}
			<p class="loading">This browser couldn't load the puzzle.</p>
		{:else}
			<p class="loading">folding…</p>
		{/if}
	</div>

	{#if game}
		<p class="status" aria-live="polite">
			{#if restartAsked}
				<strong>Press restart again to start over</strong>
			{:else if notice}
				{notice}
			{:else if game.solved}
				<strong>Solved in {game.moves} {game.moves === 1 ? 'move' : 'moves'}!{game.newBest ? ' New best!' : ''}</strong>
			{:else}
				{game.crossings} {game.crossings === 1 ? 'crossing' : 'crossings'} · {game.moves}
				{game.moves === 1 ? 'move' : 'moves'}
			{/if}
		</p>

		<nav>
			<button onclick={() => jump(-1)} disabled={game.level <= 1} aria-label="Previous level">‹</button>
			<button onclick={restart}>{game.solved ? 'replay' : 'restart'}</button>
			<button onclick={copyLink}>copy link</button>
			<button class:primary={game.solved} onclick={() => jump(1)} aria-label="Next level">
				{game.solved ? 'next level ›' : '›'}
			</button>
		</nav>

		<p class="hint">
			Drag dots around the circle until no creases cross.<br />
			<span class="keys">Keys: ← → move · space pick up / set down · esc put back · [ ] levels · r restart · c copy</span>
		</p>
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
	header {
		text-align: center;
	}
	h1 {
		margin: 0;
		font-size: 2rem;
		font-weight: 600;
		letter-spacing: 0.04em;
	}
	.meta {
		margin: 0.25rem 0 0;
		color: var(--muted);
		font-size: 0.9rem;
	}
	.code {
		font-family: ui-monospace, monospace;
	}
	.board {
		width: min(100%, 62svh);
		aspect-ratio: 1;
		display: grid;
		place-items: center;
	}
	.loading {
		color: var(--muted);
	}
	.status {
		margin: 0;
		min-height: 1.5em;
		font-variant-numeric: tabular-nums;
	}
	.status strong {
		color: var(--accent);
	}
	nav {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		justify-content: center;
	}
	button {
		font: inherit;
		color: var(--text);
		background: var(--button);
		border: 1px solid var(--line);
		border-radius: 999px;
		padding: 0.45rem 1rem;
		min-width: 2.75rem;
		cursor: pointer;
	}
	button:disabled {
		opacity: 0.4;
		cursor: default;
	}
	button.primary {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--on-accent);
	}
	.hint {
		margin: 0.5rem 0 0;
		text-align: center;
		color: var(--muted);
		font-size: 0.85rem;
		line-height: 1.5;
	}
	@media (hover: none) {
		.keys {
			display: none;
		}
	}
</style>
