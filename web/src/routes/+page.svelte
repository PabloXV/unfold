<script lang="ts">
	import { onMount, tick, untrack } from 'svelte';
	import { replaceState } from '$app/navigation';
	import Board from '#lib/Board.svelte';
	import Card from '#lib/Card.svelte';
	import { Game } from '#lib/game.svelte.ts';
	import { Attempt, dailyLevels, dailySeed, formatDay, loadCore, parseCode, today, type Point } from '#lib/unfold.ts';
	import { bestMoves } from '#lib/scores.ts';
	import { forget, readJson, writeJson } from '#lib/storage.ts';
	import { shareCard, shareText, squares, toPng, total, type Result } from '#lib/card.ts';

	const RESTART_TIME = 2000;
	const NOTICE_TIME = 1500;
	const CARD_DELAY = 1500;

	let game = $state.raw<Game | undefined>(undefined);
	/** Set when playing a day's seed. */
	let day = $state<number | undefined>(undefined);
	/** The day's levels in order; unset outside the daily. */
	let ladder = $state.raw<number[] | undefined>(undefined);
	let pars = $state.raw<number[]>([]);
	/** Bumped on every solve, so the day's result is worked out again. */
	let scored = $state(0);
	let cardOpen = $state(false);
	let editing = $state(false);
	/** The card's shape as reshaped, and as it first unfolded. */
	let corners = $state.raw<Point[] | undefined>(undefined);
	let unfoldedCorners: Point[] | undefined;
	let cardView = $state<Card | undefined>(undefined);
	/** The card as a PNG, made ahead so sharing opens the share sheet right on the tap. */
	let png: Promise<Blob> | undefined;
	/** Shown only once the keyboard is used. */
	let showCursor = $state(false);
	let restartAsked = $state(false);
	let notice = $state('');
	let failed = $state(false);
	/** Levels left with [ and ], kept as they were. */
	const shelf = new Map<number, Game>();
	let restartTimer: ReturnType<typeof setTimeout> | undefined;
	let noticeTimer: ReturnType<typeof setTimeout> | undefined;

	const rung = $derived(ladder && game ? ladder.indexOf(game.level) : -1);
	const onLastRung = $derived(ladder !== undefined && rung === ladder.length - 1);
	const result = $derived.by((): Result | undefined => {
		void scored;
		if (!ladder || !game || day === undefined) return;
		const moves = ladder.map((level) => bestMoves(game!.seed, level));
		if (moves.some((m) => m === undefined)) return;
		return { date: formatDay(day), moves: moves as number[], pars };
	});

	const here = () => location.origin + location.pathname;
	const shapeKey = () => `unfold:shape:${day}`;

	onMount(() => {
		loadCore().then(
			() => open(location.hash),
			() => (failed = true)
		);
		if (import.meta.env.DEV) Object.defineProperty(window, '__unfold', { get: () => game, configurable: true });
		const onhash = () => {
			if (location.hash.slice(1) !== game?.code) open(location.hash);
		};
		addEventListener('hashchange', onhash);
		return () => removeEventListener('hashchange', onhash);
	});

	$effect(() => {
		if (game?.solved) untrack(() => scored++);
	});

	$effect(() => {
		if (!result || !onLastRung || !game?.solved) return;
		untrack(loadCorners);
		const timer = setTimeout(() => (cardOpen = true), CARD_DELAY);
		return () => clearTimeout(timer);
	});

	/** A hash like #ABC123/9 opens that puzzle; anything else, today's daily at its first unsolved level. */
	function open(hash: string): void {
		const code = parseCode(decodeURIComponent(hash.slice(1)));
		const now = today();
		let seed: number, level: number;
		if (code) {
			[seed, level] = [code[0], code[1]];
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
		game = new Game(seed, level);
		closeCard();
		corners = undefined;
		showUrl();
	}

	/** The next or previous level, along the ladder in the daily. */
	function neighbour(step: 1 | -1): number | undefined {
		if (!game) return;
		if (!ladder) return game.level + step >= 1 ? game.level + step : undefined;
		return ladder[rung + step];
	}

	/** The card's shape: as saved, else as just unfolded, else the flat sheet. */
	function loadCorners(): void {
		if (!game || !onLastRung) return;
		unfoldedCorners = game.solved ? game.unfolded.map((p) => ({ ...p })) : game.sheet();
		const saved = readJson<Point[]>(shapeKey());
		corners = saved?.length === game.dots ? saved : unfoldedCorners;
	}

	function openCard(): void {
		if (!corners) loadCorners();
		cardOpen = true;
	}

	function closeCard(): void {
		cardOpen = editing = false;
	}

	function saveCorners(next: Point[]): void {
		corners = next;
		writeJson(
			shapeKey(),
			next.map((p) => ({ x: +p.x.toFixed(4), y: +p.y.toFixed(4) }))
		);
	}

	function resetShape(): void {
		if (unfoldedCorners) corners = unfoldedCorners;
		forget(shapeKey());
	}

	$effect(() => {
		if (!cardOpen || !cardView) return;
		void corners;
		tick().then(() => {
			const svg = cardView?.element();
			if (svg) png = toPng(svg);
		});
	});

	async function share(): Promise<void> {
		if (!png || !result) return;
		editing = false;
		const outcome = await shareCard(await png, shareText(result, here()));
		if (outcome === 'copied') flash('card copied');
		else if (outcome === 'downloaded') flash('card saved');
	}

	/** Copies text, or shows it when the clipboard is blocked. */
	async function copy(text: string, done: string): Promise<void> {
		try {
			await navigator.clipboard.writeText(text);
			flash(done);
		} catch {
			flash(text);
		}
	}

	/** The daily keeps a clean address; any other seed shows its code. */
	function showUrl(): void {
		if (!game) return;
		replaceState(day === undefined ? `#${game.code}` : location.pathname, {});
	}

	function jump(step: 1 | -1): void {
		const level = neighbour(step);
		if (!game || level === undefined) return;
		closeCard();
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
		closeCard();
		const { seed, level } = game;
		game.free();
		game = new Game(seed, level);
	}

	function flash(text: string): void {
		notice = text;
		clearTimeout(noticeTimer);
		noticeTimer = setTimeout(() => (notice = ''), NOTICE_TIME);
	}

	function onkeydown(e: KeyboardEvent): void {
		if (!game || e.metaKey || e.ctrlKey || e.altKey) return;
		if (cardOpen) {
			if (e.key === 'Escape') closeCard();
			return;
		}
		if (e.key !== 'r') restartAsked = false;
		const g = game;
		const select = () => (g.solved ? jump(1) : g.select());
		const keys: Record<string, () => void> = {
			ArrowLeft: () => g.step(1),
			h: () => g.step(1),
			ArrowRight: () => g.step(-1),
			l: () => g.step(-1),
			' ': select,
			Enter: select,
			Escape: () => g.putBack(),
			'[': () => jump(-1),
			']': () => jump(1),
			r: restart,
			c: () => copy(`${here()}#${g.code}`, 'link copied')
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
			<p class="total">
				Today: <strong>{total(result.moves)} moves</strong>, par {total(result.pars)}
				<span class="squares">{squares(result)}</span>
			</p>
			<button class="primary" onclick={openCard}>your card</button>
		{/if}

		<nav>
			<button onclick={() => jump(-1)} disabled={neighbour(-1) === undefined} aria-label="Previous level">‹</button>
			<button onclick={restart}>{game.solved ? 'replay' : 'restart'}</button>
			<button onclick={() => game && copy(`${here()}#${game.code}`, 'link copied')}>copy link</button>
			<button class:primary={game.solved && !onLastRung} onclick={() => jump(1)} disabled={neighbour(1) === undefined} aria-label="Next level">
				{game.solved && !onLastRung ? 'next level ›' : '›'}
			</button>
		</nav>

		<p class="hint">
			Drag dots around the circle until no creases cross.<br />
			<span class="keys">Keys: ← → move · space pick up / set down · esc put back · [ ] levels · r restart · c copy</span>
		</p>
	{/if}
</main>

{#if cardOpen && result && corners && game}
	<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && closeCard()}>
		<div class="sheet" role="dialog" aria-modal="true" aria-label="Your card">
			<button class="close" onclick={closeCard} aria-label="Close">×</button>
			<Card
				bind:this={cardView}
				{result}
				flaps={game.flaps}
				faceUp={game.faceUp}
				{corners}
				host={location.host}
				{editing}
				onchange={saveCorners}
			/>
			<p class="hint">
				{#if editing}
					Drag the corners to reshape it. Pull a flap over to show the paper's other side.
				{:else}
					Share it, or reshape it first with the pencil.
				{/if}
			</p>
			<nav>
				<button class="icon" class:primary={editing} onclick={() => (editing = !editing)} aria-pressed={editing} aria-label="Reshape">
					<svg viewBox="0 0 24 24" aria-hidden="true"
						><path d="M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16v4Z" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" /></svg
					>
				</button>
				{#if editing}
					<button onclick={resetShape}>reset</button>
				{/if}
				<button class="primary" onclick={share}>share</button>
				<button onclick={() => result && copy(shareText(result, here()), 'result copied')}>copy text</button>
			</nav>
			{#if notice}<p class="hint">{notice}</p>{/if}
		</div>
	</div>
{/if}

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
	.overlay {
		position: fixed;
		inset: 0;
		z-index: 10;
		display: grid;
		place-items: center;
		padding: 1rem;
		background: rgb(20 15 10 / 0.45);
		backdrop-filter: blur(3px);
		-webkit-backdrop-filter: blur(3px);
		animation: fade-in 0.25s ease-out;
	}
	.sheet {
		position: relative;
		width: min(100%, 28rem);
		max-height: 100%;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		padding: 2.75rem 1rem 1rem;
		border-radius: 1.5rem;
		background: var(--bg);
		box-shadow: 0 1rem 3rem rgb(0 0 0 / 0.3);
		animation: pop-in 0.35s cubic-bezier(0.2, 1.3, 0.4, 1);
	}
	.close {
		position: absolute;
		top: 0.4rem;
		right: 0.4rem;
		z-index: 1;
		min-width: 0;
		width: 2.25rem;
		height: 2.25rem;
		padding: 0;
		font-size: 1.4rem;
		line-height: 1;
		border-color: transparent;
		background: transparent;
		color: var(--muted);
	}
	button.icon {
		display: grid;
		place-items: center;
		padding: 0.45rem 0.8rem;
	}
	button.icon svg {
		width: 1.25rem;
		height: 1.25rem;
	}
	@keyframes fade-in {
		from {
			opacity: 0;
		}
	}
	@keyframes pop-in {
		from {
			opacity: 0;
			transform: translateY(1.5rem) scale(0.96);
		}
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
