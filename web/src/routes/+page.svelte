<script lang="ts">
	import { onMount, tick, untrack } from 'svelte';
	import { replaceState } from '$app/navigation';
	import Board from '#lib/Board.svelte';
	import Card from '#lib/Card.svelte';
	import Icon from '#lib/Icon.svelte';
	import { Game } from '#lib/game.svelte.ts';
	import { dailyLevel, dailySeed, formatDay, loadCore, paperColors, parseCode, today, type Point } from '#lib/unfold.ts';
	import { bestMoves } from '#lib/scores.ts';
	import { forget, readJson, writeJson } from '#lib/storage.ts';
	import { shareCard, shareLink, shareText, toPng, type Result } from '#lib/card.ts';
	import { accentFor } from '#lib/accent.ts';
	import { swanUrl } from '#lib/favicon.ts';

	const RESTART_TIME = 2000;
	const NOTICE_TIME = 1500;
	const CARD_DELAY = 1500;
	const RESHAPED_KEY = 'unfold:reshaped';
	const BACKDROPS = { light: ['#ebe4d6', '#fffdf8'], dark: ['#3d3833', '#6a645c'] };

	let game = $state.raw<Game | undefined>(undefined);
	/** Set when playing a day's seed. */
	let day = $state<number | undefined>(undefined);
	/** Bumped on every solve, so the day's result is worked out again. */
	let scored = $state(0);
	let paper = $state.raw<string[]>([]);
	let dark = $state(false);
	let cardOpen = $state(false);
	let helpOpen = $state(false);
	/** The card's shape as reshaped, and as it first unfolded. */
	let reshaped = $state(false);
	let corners = $state.raw<Point[] | undefined>(undefined);
	/** The card's flaps bottom to top; grabbing a corner lifts its flaps to the top. */
	let order = $state.raw<number[]>([]);
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

	const daily = $derived(game !== undefined && day !== undefined && game.seed === dailySeed(day) && game.level === dailyLevel());
	const result = $derived.by((): Result | undefined => {
		void scored;
		if (!daily || !game || day === undefined) return;
		const moves = bestMoves(game.seed, game.level);
		return moves === undefined ? undefined : { date: formatDay(day), moves, par: game.par };
	});

	const here = () => location.origin + location.pathname;
	const shapeKey = () => `unfold:shape:${day}`;
	const solvedKey = () => `unfold:solved:${day}`;
	const orderKey = () => `unfold:order:${day}`;

	onMount(() => {
		loadCore().then(
			() => open(location.hash),
			() => (failed = true)
		);
		if (import.meta.env.DEV) Object.defineProperty(window, '__unfold', { get: () => game, configurable: true });
		const scheme = matchMedia('(prefers-color-scheme: dark)');
		const onscheme = () => (dark = scheme.matches);
		onscheme();
		scheme.addEventListener('change', onscheme);
		const onhash = () => {
			if (location.hash.slice(1) !== game?.code) open(location.hash);
		};
		addEventListener('hashchange', onhash);
		return () => {
			removeEventListener('hashchange', onhash);
			scheme.removeEventListener('change', onscheme);
		};
	});

	$effect(() => {
		if (game?.solved) untrack(() => scored++);
	});

	$effect(() => {
		if (!result || !game?.solved) return;
		untrack(loadCorners);
		const timer = setTimeout(() => (cardOpen = true), CARD_DELAY);
		return () => clearTimeout(timer);
	});

	$effect(() => {
		const style = document.documentElement.style;
		const [front, back] = paper;
		if (!front) return;
		style.setProperty('--paper-front', front);
		style.setProperty('--paper-back', back);
		document.querySelector('link[rel="icon"]')?.setAttribute('href', swanUrl(front, back));
		const accent = accentFor(paper, dark ? BACKDROPS.dark : BACKDROPS.light);
		if (accent) {
			style.setProperty('--accent', accent.accent);
			style.setProperty('--on-accent', accent.onAccent);
		} else {
			style.removeProperty('--accent');
			style.removeProperty('--on-accent');
		}
	});

	$effect(() => {
		if (!cardOpen || !cardView) return;
		void [corners, order];
		tick().then(() => {
			const svg = cardView?.element();
			if (svg) png = toPng(svg);
		});
	});

	/** A hash like #ABC123/9 opens that puzzle; anything else, today's puzzle. */
	function open(hash: string): void {
		const code = parseCode(decodeURIComponent(hash.slice(1)));
		const now = today();
		const [seed, level] = code ?? [dailySeed(now), dailyLevel()];
		day = seed === dailySeed(now) ? now : undefined;
		for (const g of shelf.values()) g.free();
		shelf.clear();
		game?.free();
		game = new Game(seed, level);
		paper = paperColors(seed);
		closeCard();
		corners = undefined;
		showUrl();
	}

	/** The next or previous level of a shared puzzle; the daily stands alone. */
	function neighbour(step: 1 | -1): number | undefined {
		if (!game || daily) return;
		return game.level + step >= 1 ? game.level + step : undefined;
	}

	/** The card's shape: as reshaped, else as solved, on whichever side the player solved it. */
	function loadCorners(): void {
		if (!game || !daily) return;
		const fits = (p: Point[] | undefined) => p?.length === game!.dots;
		if (game.solved) writeJson(solvedKey(), round(game.unfolded));
		const solved = readJson<Point[]>(solvedKey());
		unfoldedCorners = fits(solved) ? solved : game.sheet();
		const saved = readJson<Point[]>(shapeKey());
		reshaped = fits(saved);
		corners = reshaped ? saved! : unfoldedCorners;
		const stack = readJson<number[]>(orderKey());
		order = stack?.length === game.flaps.length ? stack : game.flaps.map((_, k) => k);
	}

	function openCard(): void {
		if (!corners) loadCorners();
		cardOpen = true;
	}

	function closeCard(): void {
		cardOpen = false;
	}

	function saveShape(next: Point[], stack: number[]): void {
		corners = next;
		order = stack;
		reshaped = true;
		writeJson(RESHAPED_KEY, true);
		writeJson(shapeKey(), round(next));
		writeJson(orderKey(), stack);
	}

	const round = (pts: Point[]) => pts.map((p) => ({ x: +p.x.toFixed(4), y: +p.y.toFixed(4) }));

	function resetShape(): void {
		if (unfoldedCorners) corners = unfoldedCorners;
		order = game ? game.flaps.map((_, k) => k) : [];
		reshaped = false;
		forget(shapeKey());
		forget(orderKey());
	}

	async function share(): Promise<void> {
		if (!png || !result) return;
		const outcome = await shareCard(await png, shareText(result, here()));
		if (outcome === 'copied') flash('card copied');
		else if (outcome === 'downloaded') flash('card saved');
	}

	async function shareResultLink(): Promise<void> {
		if (!result) return;
		const outcome = await shareLink(shareText(result, here()));
		if (outcome === 'copied') flash('link copied');
		else if (outcome === 'failed') flash(shareText(result, here()));
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

	/** The daily keeps a clean address; any other puzzle shows its code. */
	function showUrl(): void {
		if (!game) return;
		replaceState(daily ? location.pathname : `#${game.code}`, {});
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
		if (cardOpen || helpOpen) {
			if (e.key === 'Escape') cardOpen = helpOpen = false;
			return;
		}
		if (e.key !== 'r') restartAsked = false;
		const g = game;
		const select = () => (!g.solved ? g.select() : result ? openCard() : jump(1));
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
			'?': () => (helpOpen = true),
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
		<button class="icon help" onclick={() => (helpOpen = true)} aria-label="How to play" title="How to play"><Icon name="help" /></button>
		<h1>unfold</h1>
		{#if game}
			<p class="meta">
				{[daily && day !== undefined ? formatDay(day) : `level ${game.level}`, `fewest possible ${game.par}`]
					.filter(Boolean)
					.join(' · ')}
				{#if !daily}<span class="code">{game.code}</span>{/if}
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
					Solved in {game.moves} {game.moves === 1 ? 'move' : 'moves'}{game.moves <= game.par ? ', the fewest possible!' : ` (fewest possible ${game.par})`}
				</strong>
			{:else}
				{game.moves} {game.moves === 1 ? 'move' : 'moves'}
			{/if}
		</p>

		<nav>
			{#if !daily}
				<button onclick={() => jump(-1)} disabled={neighbour(-1) === undefined} aria-label="Previous level">‹</button>
			{/if}
			<button class="icon" onclick={restart} aria-label={game.solved ? 'Replay' : 'Restart'} title={game.solved ? 'Replay' : 'Restart'}><Icon name="reset" /></button>
			{#if result}
				<button class="icon primary" onclick={openCard} aria-label="Your card" title="Your card"><Icon name="share" /></button>
			{:else}
				<button class="icon" onclick={() => game && copy(`${here()}#${game.code}`, 'link copied')} aria-label="Copy link" title="Copy link"
					><Icon name="link" /></button
				>
			{/if}
			{#if !daily}
				<button class:primary={game.solved} onclick={() => jump(1)} aria-label="Next level">{game.solved ? 'next level ›' : '›'}</button>
			{/if}
		</nav>

	{/if}
</main>

{#if helpOpen}
	<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && (helpOpen = false)}>
		<div class="sheet help-sheet" role="dialog" aria-modal="true" aria-label="How to play">
			<button class="close" onclick={() => (helpOpen = false)} aria-label="Close"><Icon name="close" /></button>
			<h2>How to play</h2>
			<p>The paper is folded up around a circle. Move its dots until no creases cross, and it unfolds.</p>
			<p>Drag a dot along the circle; the others shift over to make room. Setting a dot down somewhere new is a move.</p>
			<p>Fewest possible is the least number of moves it can be solved in. There's a new puzzle every day.</p>
			<p>When it's solved, share your card. Drag its corners to fold the paper into a shape of your own first.</p>
			<dl class="keys">
				<dt>← →</dt>
				<dd>move around the circle</dd>
				<dt>space</dt>
				<dd>pick up, set down</dd>
				<dt>esc</dt>
				<dd>put back</dd>
				<dt>r</dt>
				<dd>restart</dd>
				<dt>c</dt>
				<dd>copy link</dd>
			</dl>
		</div>
	</div>
{/if}

{#if cardOpen && result && corners && game}
	<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && closeCard()}>
		<div class="sheet" role="dialog" aria-modal="true" aria-label="Your card">
			<button class="close" onclick={closeCard} aria-label="Close"><Icon name="close" /></button>
			<div class="frame">
				<Card
					bind:this={cardView}
					{result}
					flaps={game.flaps}
					faceUp={game.faceUp}
					{corners}
					{order}
					host={location.host}
					hint={!readJson<boolean>(RESHAPED_KEY)}
					onchange={saveShape}
				/>
				<div class="bar" role="toolbar" aria-label="Card">
					<button onclick={resetShape} disabled={!reshaped} aria-label="Reset shape" title="Reset shape">
						<Icon name="reset" />
					</button>
					<button onclick={share} aria-label="Share card" title="Share card">
						<Icon name="share" />
					</button>
					<button onclick={shareResultLink} aria-label="Share link" title="Share link">
						<Icon name="link" />
					</button>
				</div>
			</div>
			<p class="notice" aria-live="polite">{notice}</p>
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
		position: relative;
		width: 100%;
		text-align: center;
	}
	.help {
		position: absolute;
		top: 0;
		right: 0;
		border-color: transparent;
		background: transparent;
		color: var(--muted);
	}
	button.icon {
		display: grid;
		place-items: center;
		width: 2.75rem;
		height: 2.75rem;
		padding: 0;
	}
	h1 {
		margin: 0;
		font-size: 2.2rem;
		font-weight: 700;
	}
	.meta {
		margin: 0.25rem 0 0;
		color: var(--muted);
		font-size: 0.9rem;
	}
	.board {
		position: relative;
		width: min(100%, 62svh);
		aspect-ratio: 1;
		display: grid;
		place-items: center;
	}
	.board::before {
		content: '';
		position: absolute;
		inset: -45%;
		z-index: -1;
		pointer-events: none;
		background: radial-gradient(closest-side, var(--spot), transparent);
	}
	.loading {
		color: var(--muted);
	}
	.status {
		margin: 0;
		min-height: 1.5em;
		text-align: center;
		font-variant-numeric: tabular-nums;
	}
	.status strong {
		color: var(--accent);
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
		gap: 0.5rem;
		padding: 2.75rem 1rem 0.75rem;
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
		display: grid;
		place-items: center;
		border-color: transparent;
		background: transparent;
		color: var(--muted);
	}
	.help-sheet {
		align-items: stretch;
		gap: 0;
		padding: 1.5rem 1.5rem 1.25rem;
		line-height: 1.5;
	}
	.help-sheet h2 {
		margin: 0 0 0.75rem;
		font-size: 1.3rem;
	}
	.help-sheet p {
		margin: 0 0 0.75rem;
	}
	.keys {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.25rem 1rem;
		margin: 0.5rem 0 0;
		padding-top: 0.75rem;
		border-top: 1px solid var(--line);
		color: var(--muted);
		font-size: 0.9rem;
	}
	.keys dt {
		font-weight: 700;
		color: var(--text);
	}
	.keys dd {
		margin: 0;
	}
	.frame {
		width: 100%;
		border-radius: 1rem;
		overflow: hidden;
		box-shadow: 0 0.25rem 1rem rgb(40 30 20 / 0.18);
	}
	.bar {
		display: flex;
		justify-content: space-around;
		padding: 0.35rem 0.5rem;
		background: #e4ddd0;
	}
	.bar button {
		display: grid;
		place-items: center;
		min-width: 0;
		width: 2.75rem;
		height: 2.75rem;
		padding: 0;
		border: none;
		background: transparent;
		color: #3a352f;
	}
	.bar button:disabled {
		opacity: 0.3;
	}
	.notice {
		margin: 0;
		min-height: 1.3em;
		color: var(--muted);
		font-size: 0.85rem;
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
	@media (hover: none), (pointer: coarse) {
		.keys {
			display: none;
		}
	}
</style>
