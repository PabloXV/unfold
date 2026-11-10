<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import Board from '#lib/Board.svelte';
	import { Game } from '#lib/game.svelte.ts';
	import { Attempt, dailyLevels, dailySeed, formatDay, loadCore, parseCode, today, type Point } from '#lib/unfold.ts';
	import { bestMoves, firstUnsolved } from '#lib/scores.ts';
	import { renderCard, shareCard, shareText, squares, type Result } from '#lib/card.ts';

	const RESTART_TIME = 2000;
	const NOTICE_TIME = 1500;

	let game = $state.raw<Game | undefined>(undefined);
	/** Set when playing a day's seed. */
	let day = $state<number | undefined>(undefined);
	/** The day's levels, in order, and the par of each; unset outside the daily. */
	let ladder = $state.raw<number[] | undefined>(undefined);
	let pars = $state.raw<number[]>([]);
	/** Bumped whenever a score is recorded, so the day's result is worked out again. */
	let scored = $state(0);
	/** Dragging the finished shape's corners before sharing. */
	let reshaping = $state(false);
	/** Where the finished shape's corners were before any reshaping, to put them back. */
	let unreshaped: Point[] | undefined;
	let card = $state<{ blob: Blob; url: string } | undefined>(undefined);
	let board = $state<Board | undefined>(undefined);

	const rung = $derived(ladder && game ? ladder.indexOf(game.level) : -1);
	const onLastRung = $derived(ladder !== undefined && rung === ladder.length - 1);
	/** The day's moves per level, once every level is solved. */
	const result = $derived.by((): Result | undefined => {
		void scored;
		if (!ladder || !game || day === undefined) return;
		const moves = ladder.map((level) => bestMoves(game!.seed, level));
		if (moves.some((m) => m === undefined)) return;
		return { date: formatDay(day), moves: moves as number[], pars };
	});
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
		// the game in play, for automated checks; left out of production builds
		if (import.meta.env.DEV) Object.defineProperty(window, '__unfold', { get: () => game, configurable: true });
		const onhash = () => {
			if (location.hash.slice(1) !== game?.code) open(location.hash);
		};
		addEventListener('hashchange', onhash);
		return () => removeEventListener('hashchange', onhash);
	});

	/**
	 * A hash like #ABC123/9 opens that puzzle, with levels on from there; anything else, today's
	 * daily, its five levels in a row, starting at the first not yet solved.
	 */
	function open(hash: string): void {
		const code = parseCode(decodeURIComponent(hash.slice(1)));
		const now = today();
		let seed: number, level: number;
		if (code) {
			[seed, level] = [code[0], code[1]];
			// a pasted code from today's daily gets its date too
			day = seed === dailySeed(now) ? now : undefined;
			ladder = undefined;
		} else {
			[seed, day] = [dailySeed(now), now];
			ladder = Array.from(dailyLevels());
			pars = ladder.map((l) => {
				const a = new Attempt(seed, l);
				const par = a.par;
				a.free();
				return par;
			});
			level = ladder.find((l) => bestMoves(seed, l) === undefined) ?? ladder[ladder.length - 1];
		}
		for (const g of shelf.values()) g.free();
		shelf.clear();
		game?.free();
		game = Game.ofSeed(seed, level);
		reshaping = false;
		showUrl();
	}

	/** In the daily, the level `step` along its ladder; elsewhere, the next or previous level. */
	function neighbour(step: 1 | -1): number | undefined {
		if (!game) return;
		if (!ladder) return game.level + step >= 1 ? game.level + step : undefined;
		return ladder[rung + step];
	}

	const shapeKey = () => `unfold:shape:${day}`;

	/** The day's finished shape: as it lies on the board if just solved, else as last saved. */
	function finishedShape(): { flaps: number[][]; faceUp: number[]; corners: Point[] } | undefined {
		if (!game || !onLastRung) return;
		let corners: Point[] | undefined = game.solved ? game.unfolded : undefined;
		if (!corners) {
			try {
				corners = JSON.parse(localStorage.getItem(shapeKey()) ?? 'null') ?? undefined;
			} catch {
				// the flat sheet will do
			}
		}
		if (corners?.length !== game.dots) corners = game.sheet();
		return { flaps: game.flaps, faceUp: game.faceUp, corners };
	}

	async function drawCard(): Promise<void> {
		const shape = finishedShape();
		if (!result || !shape) return;
		const blob = await renderCard(result, shape, location.host);
		if (card) URL.revokeObjectURL(card.url);
		card = { blob, url: URL.createObjectURL(blob) };
	}

	// the card follows the result: drawn once the day is done
	$effect(() => {
		if (result && onLastRung) drawCard();
	});

	// a solve records a score, which may finish the day
	$effect(() => {
		if (game?.solved) untrack(() => scored++);
	});

	function toggleReshape(): void {
		if (!game?.solved) return;
		if (!reshaping && !unreshaped) unreshaped = game.unfolded.map((p) => ({ ...p }));
		reshaping = !reshaping;
		if (!reshaping) drawCard();
	}

	function resetShape(): void {
		if (!game || !unreshaped) return;
		unreshaped.forEach((p, v) => (game!.unfolded[v] = { ...p }));
		board?.refresh();
		saveShape(game.unfolded);
	}

	function saveShape(corners: Point[]): void {
		try {
			localStorage.setItem(shapeKey(), JSON.stringify(corners.map((p) => ({ x: +p.x.toFixed(4), y: +p.y.toFixed(4) }))));
		} catch {
			// the card still has it for now
		}
		drawCard();
	}

	async function share(): Promise<void> {
		if (!card || !result) return;
		const outcome = await shareCard(card.blob, shareText(result, location.origin + location.pathname));
		if (outcome === 'copied') flash('card copied');
		else if (outcome === 'downloaded') flash('card saved');
	}

	async function copyText(): Promise<void> {
		if (!result) return;
		try {
			await navigator.clipboard.writeText(shareText(result, location.origin + location.pathname));
			flash('result copied');
		} catch {
			flash(squares(result));
		}
	}

	/** The daily keeps a clean address; any other seed shows its code, to bookmark or share. */
	function showUrl(): void {
		if (!game) return;
		const url = day === undefined ? `#${game.code}` : location.pathname;
		history.replaceState(history.state, '', url);
	}

	function jump(step: 1 | -1): void {
		const level = neighbour(step);
		if (!game || level === undefined) return;
		reshaping = false;
		unreshaped = undefined;
		shelf.set(game.level, game);
		game = shelf.get(level) ?? Game.ofSeed(game.seed, level);
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
		reshaping = false;
		unreshaped = undefined;
		const { seed, level } = game;
		game.free();
		game = Game.ofSeed(seed, level);
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
				{[
					day !== undefined && formatDay(day),
					ladder ? `level ${rung + 1} of ${ladder.length}` : `level ${game.level}`,
					`par ${game.par}`,
					game.best !== undefined && `best ${game.best}`
				]
					.filter(Boolean)
					.join(' · ')}
				{#if !ladder}<span class="code">{game.code}</span>{/if}
			</p>
		{/if}
	</header>

	<div class="board">
		{#if game}
			<Board bind:this={board} {game} {showCursor} reshape={reshaping} onreshape={saveShape} />
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
			{:else if reshaping}
				Drag the corners to reshape it. Flip a flap to show its other side.
			{:else if game.solved}
				<strong>
					Solved in {game.moves} {game.moves === 1 ? 'move' : 'moves'}{game.moves <= game.par ? ', par!' : ` (par ${game.par})`}{game.newBest
						? ' New best!'
						: ''}
				</strong>
			{:else}
				{game.crossings} {game.crossings === 1 ? 'crossing' : 'crossings'} · {game.moves}
				{game.moves === 1 ? 'move' : 'moves'}
			{/if}
		</p>

		{#if result && onLastRung}
			<section class="done" aria-label="Today's result">
				<p class="total">
					Today: <strong>{result.moves.reduce((a, b) => a + b, 0)} moves</strong>, par {result.pars.reduce((a, b) => a + b, 0)}
					<span class="squares">{squares(result)}</span>
				</p>
				{#if card}<img class="card" src={card.url} alt="Your share card: today's finished shape and score" />{/if}
				<nav>
					<button class="primary" onclick={share} disabled={!card}>share</button>
					<button onclick={copyText}>copy text</button>
					{#if game.solved}
						<button class:primary={reshaping} onclick={toggleReshape}>{reshaping ? 'done reshaping' : 'reshape'}</button>
						{#if reshaping}<button onclick={resetShape}>reset shape</button>{/if}
					{/if}
				</nav>
			</section>
		{/if}

		<nav>
			<button onclick={() => jump(-1)} disabled={neighbour(-1) === undefined} aria-label="Previous level">‹</button>
			<button onclick={restart}>{game.solved ? 'replay' : 'restart'}</button>
			<button onclick={copyLink}>copy link</button>
			<button class:primary={game.solved && !onLastRung} onclick={() => jump(1)} disabled={neighbour(1) === undefined} aria-label="Next level">
				{game.solved && !onLastRung ? 'next level ›' : '›'}
			</button>
		</nav>

		<p class="hint">
			Drag dots around the circle until no creases cross.<br />
			<span class="keys">Keys: ← → move · space pick up / set down · esc put back · [ ] levels · r restart · c copy</span>
		</p>
		<p class="hint"><a href="mystery">Try the mystery prototype →</a></p>
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
	.done {
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		padding: 1rem 0;
		border-top: 1px solid var(--line);
		border-bottom: 1px solid var(--line);
	}
	.total {
		margin: 0;
		text-align: center;
	}
	.squares {
		display: block;
		font-size: 1.4rem;
		letter-spacing: 0.1em;
		margin-top: 0.25rem;
	}
	.card {
		width: min(100%, 18rem);
		border-radius: 0.75rem;
		box-shadow: 0 0.5rem 1.5rem rgb(60 45 25 / 0.18);
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
