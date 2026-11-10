<!--
	Prototype: a mystery animal over five levels. Each level is the same picture at more corners;
	solving one unfolds it in color and earns one guess. Guess right, or finish all five, and the
	answer shows.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import Board from '#lib/Board.svelte';
	import type { Game } from '#lib/game.svelte.ts';
	import { loadCore } from '#lib/unfold.ts';
	import Thumb from '#lib/mystery/Thumb.svelte';
	import { LEVELS, guessable, markPlayed, pickShape, playLevel, resolveGuess, shapes } from '#lib/mystery/mystery.ts';

	type Guess = { level: number; name: string; correct: boolean };

	let shapeIndex = $state(0);
	let level = $state(1);
	let game = $state.raw<Game | undefined>(undefined);
	let colors = $state.raw<string[]>([]);
	let guesses = $state<Guess[]>([]);
	/** Moves on the levels already behind. */
	let pastMoves = $state(0);
	let gaveUp = $state(false);
	let text = $state('');
	let feedback = $state('');
	let showCursor = $state(false);
	let failed = $state(false);
	let input = $state<HTMLInputElement | undefined>(undefined);

	const shape = $derived(shapes[shapeIndex]);
	const solved = $derived(game?.solved ?? false);
	const gotItAt = $derived(guesses.find((g) => g.correct)?.level);
	const guessedThisLevel = $derived(guesses.some((g) => g.level === level));
	const canGuess = $derived(solved && gotItAt === undefined && !guessedThisLevel && !gaveUp);
	const revealed = $derived(gotItAt !== undefined || gaveUp || (level === LEVELS && solved && guessedThisLevel));
	const totalMoves = $derived(pastMoves + (game?.moves ?? 0));
	/** Pictures unfolded so far, this level's once solved. */
	const unfoldedLevels = $derived(shape.levels.slice(0, solved ? level : level - 1));

	onMount(() => {
		loadCore().then(
			() => start(hashShape() ?? pickShape()),
			() => (failed = true)
		);
		const onhash = () => {
			const i = hashShape();
			if (i !== undefined && i !== shapeIndex) start(i);
		};
		addEventListener('hashchange', onhash);
		return () => removeEventListener('hashchange', onhash);
	});

	/** #12 plays shape 12, to come back to one. */
	function hashShape(): number | undefined {
		const i = Number(location.hash.slice(1));
		return location.hash.length > 1 && Number.isInteger(i) && i >= 0 && i < shapes.length ? i : undefined;
	}

	function start(i: number): void {
		shapeIndex = i;
		guesses = [];
		pastMoves = 0;
		gaveUp = false;
		feedback = '';
		history.replaceState(history.state, '', `#${i}`);
		load(1);
	}

	function load(n: number): void {
		game?.free();
		level = n;
		text = '';
		({ game, colors } = playLevel(shapeIndex, n));
	}

	function nextLevel(): void {
		if (!game || level >= LEVELS) return;
		pastMoves += game.moves;
		feedback = '';
		load(level + 1);
	}

	function restartLevel(): void {
		load(level);
	}

	function another(): void {
		start(pickShape());
	}

	function guess(e: SubmitEvent): void {
		e.preventDefault();
		if (!canGuess || !text.trim()) return;
		const name = resolveGuess(text);
		if (!name) {
			feedback = `“${text.trim()}” isn't on the list. Try one of the suggestions.`;
			return;
		}
		const correct = name === shape.name;
		guesses = [...guesses, { level, name, correct }];
		text = '';
		feedback = correct ? '' : `Not ${article(name)} ${name}.`;
		if (correct || level === LEVELS) markPlayed(shapeIndex);
	}

	/** "an elephant", "a fox". */
	function article(name: string): string {
		return /^[aeiou]/.test(name) ? 'an' : 'a';
	}

	function giveUp(): void {
		gaveUp = true;
		markPlayed(shapeIndex);
	}

	// the guess box takes focus as soon as there's a guess to make
	$effect(() => {
		if (canGuess) input?.focus();
	});

	function onkeydown(e: KeyboardEvent): void {
		if (!game || e.metaKey || e.ctrlKey || e.altKey || e.target instanceof HTMLInputElement) return;
		const keys: Record<string, () => void> = {
			ArrowLeft: () => game!.step(1),
			ArrowRight: () => game!.step(-1),
			' ': () => (game!.solved ? nextLevel() : game!.select()),
			Enter: () => (game!.solved ? nextLevel() : game!.select()),
			Escape: () => game!.putBack()
		};
		const act = keys[e.key];
		if (!act) return;
		e.preventDefault();
		showCursor = true;
		act();
	}
</script>

<svelte:window {onkeydown} onpointerdown={() => (showCursor = false)} />

<svelte:head>
	<title>unfold · mystery</title>
</svelte:head>

<main>
	<header>
		<h1>unfold <span>mystery</span></h1>
		{#if game}
			<p class="meta">level {level} of {LEVELS} · {totalMoves} {totalMoves === 1 ? 'move' : 'moves'}</p>
		{/if}
	</header>

	<div class="board">
		{#if game}
			<Board {game} {showCursor} {colors} />
		{:else if failed}
			<p class="quiet">This browser couldn't load the puzzle.</p>
		{:else}
			<p class="quiet">folding…</p>
		{/if}
	</div>

	{#if game}
		<section class="panel" aria-live="polite">
			{#if !solved}
				<p>{game.crossings} {game.crossings === 1 ? 'crossing' : 'crossings'} · {game.moves} {game.moves === 1 ? 'move' : 'moves'}</p>
				<p class="quiet">Untangle it to unfold the next clue.</p>
			{:else if revealed}
				<p class="answer"><span class="emoji">{shape.emoji}</span> It's {article(shape.name)} <strong>{shape.name}</strong>!</p>
				<p class="quiet">
					{#if gotItAt !== undefined}
						You got it at level {gotItAt} of {LEVELS}.
					{:else if gaveUp}
						You gave up at level {level} of {LEVELS}.
					{:else}
						So close. Next one!
					{/if}
				</p>
			{:else if canGuess}
				<form onsubmit={guess}>
					<input
						bind:this={input}
						bind:value={text}
						list="animals"
						placeholder="What animal is it?"
						autocomplete="off"
						autocapitalize="none"
						enterkeyhint="go"
						aria-label="Your guess"
					/>
					<button type="submit" class="primary">Guess</button>
				</form>
				<datalist id="animals">
					{#each guessable as name (name)}<option value={name}></option>{/each}
				</datalist>
			{/if}

			{#if feedback}<p class="feedback">{feedback}</p>{/if}

			{#if guesses.length}
				<p class="guesses">
					{#each guesses as g (g.level)}
						<span class:correct={g.correct}>{g.level}: {g.name}</span>
					{/each}
				</p>
			{/if}

			<nav>
				{#if solved && level < LEVELS}
					<button class:primary={!canGuess} onclick={nextLevel}>
						{canGuess ? 'skip, next level ›' : revealed ? 'keep unfolding ›' : 'next level ›'}
					</button>
				{/if}
				{#if !solved}
					<button onclick={restartLevel} disabled={game.moves === 0 && !game.carrying}>restart level</button>
				{/if}
				{#if solved && level === LEVELS && !revealed && !canGuess}
					<button onclick={giveUp}>show me</button>
				{/if}
				{#if revealed || (solved && level === LEVELS)}
					<button class="primary" onclick={another}>another mystery</button>
				{:else if !revealed && level > 1}
					<button class="quiet-button" onclick={giveUp}>give up</button>
				{/if}
			</nav>
		</section>

		{#if unfoldedLevels.length}
			<ol class="clues" aria-label="Clues so far">
				{#each unfoldedLevels as lv, i (i)}
					<li><Thumb level={lv} /><span>{i + 1}</span></li>
				{/each}
			</ol>
		{/if}
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
	h1 span {
		font-weight: 400;
		color: var(--muted);
	}
	.meta {
		margin: 0.25rem 0 0;
		color: var(--muted);
		font-size: 0.9rem;
		font-variant-numeric: tabular-nums;
	}
	.board {
		width: min(100%, 56svh);
		aspect-ratio: 1;
		display: grid;
		place-items: center;
	}
	.panel {
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		text-align: center;
		min-height: 7rem;
	}
	.panel p {
		margin: 0;
	}
	.quiet {
		color: var(--muted);
		font-size: 0.9rem;
	}
	.answer {
		font-size: 1.3rem;
	}
	.emoji {
		font-size: 1.8rem;
		vertical-align: middle;
	}
	.feedback {
		color: var(--cross);
	}
	form {
		display: flex;
		gap: 0.5rem;
		width: min(100%, 22rem);
	}
	input {
		flex: 1;
		min-width: 0;
		font: inherit;
		font-size: 1rem;
		padding: 0.5rem 0.9rem;
		border-radius: 999px;
		border: 1px solid var(--line);
		background: var(--button);
		color: var(--text);
	}
	.guesses {
		display: flex;
		gap: 0.4rem;
		flex-wrap: wrap;
		justify-content: center;
		font-size: 0.85rem;
	}
	.guesses span {
		padding: 0.15rem 0.6rem;
		border-radius: 999px;
		background: var(--button);
		border: 1px solid var(--line);
		text-decoration: line-through;
		color: var(--muted);
	}
	.guesses span.correct {
		text-decoration: none;
		color: var(--on-accent);
		background: var(--accent);
		border-color: var(--accent);
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
	.quiet-button {
		background: none;
		border-color: transparent;
		color: var(--muted);
	}
	.clues {
		list-style: none;
		margin: 0.5rem 0 0;
		padding: 0;
		display: flex;
		gap: 0.5rem;
		justify-content: center;
	}
	.clues li {
		width: 3.5rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.2rem;
		color: var(--muted);
		font-size: 0.75rem;
	}
	.clues li :global(svg) {
		width: 3.5rem;
		height: 3.5rem;
		background: var(--button);
		border: 1px solid var(--line);
		border-radius: 0.6rem;
		padding: 0.3rem;
		box-sizing: border-box;
	}
</style>
