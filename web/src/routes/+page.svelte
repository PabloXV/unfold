<script lang="ts">
	import { onMount, tick, untrack } from 'svelte';
	import { replaceState } from '$app/navigation';
	import Board from '#lib/Board.svelte';
	import Card from '#lib/Card.svelte';
	import Icon from '#lib/Icon.svelte';
	import { Game } from '#lib/game.svelte.ts';
	import { dailySeed, formatDay, loadCore, paperColors, parseCode, today, type Point } from '#lib/unfold.ts';
	import { BUDGET, finished, frontier, grid, loadRun, overs, saveRun, spent, streak, type Run } from '#lib/run.ts';
	import { forget, readJson, writeJson } from '#lib/storage.ts';
	import { copyText, score, shareCard, shareLink, shareText, toPng, type Result } from '#lib/card.ts';
	import { accentFor } from '#lib/accent.ts';
	import { swanUrl } from '#lib/favicon.ts';
	import { CARD_BAR, DARK, LIGHT } from '#lib/theme.ts';
	import { LOCALES, locale, setLocale, t, type Locale } from '#lib/i18n/index.svelte.ts';

	const RESTART_TIME = 2000;
	const NOTICE_TIME = 1500;
	const CARD_DELAY = 1500;
	const RESHAPED_KEY = 'unfold:reshaped';
	const BACKDROPS = { light: [LIGHT.bg, LIGHT.spot], dark: [DARK.bg, DARK.spot] };

	let game = $state.raw<Game | undefined>(undefined);
	/** Set when playing a day's seed. */
	let day = $state<number | undefined>(undefined);
	/** The day's run, while playing a day's seed. */
	let run = $state.raw<Run | undefined>(undefined);
	/** The run's last level, which the card shows. */
	let cardShape = $state.raw<{ flaps: number[][]; faceUp: number[]; unfolded: Point[] } | undefined>(undefined);
	let paper = $state.raw<string[]>([]);
	let dark = $state(false);
	let cardOpen = $state(false);
	let helpOpen = $state(false);
	/** Whether the card's shape differs from how it unfolded. */
	let reshaped = $state(false);
	/** The card's corners now, and as the paper first unfolded. */
	let corners = $state.raw<Point[] | undefined>(undefined);
	let unfoldedCorners: Point[] | undefined;
	/** The card's flaps bottom to top. */
	let order = $state.raw<number[]>([]);
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

	const daily = $derived(game !== undefined && day !== undefined && run !== undefined && game.seed === dailySeed(day));
	/** Whether this is the level the run is on, whose moves count against the budget. */
	const atFrontier = $derived(daily && !finished(run!) && game!.level === frontier(run!));
	const result = $derived.by((): Result | undefined => {
		if (!daily || !run || !finished(run)) return;
		return { date: formatDay(day!, locale()), level: run.solved.length, overs: overs(run), grid: grid(run), streak: streak(day!) };
	});
	/** Whether there's more than one level to move between. */
	const levels = $derived(game !== undefined && (!daily || frontier(run!) > 1));
	const streakDays = $derived(daily && run ? streak(day!) : 0);
	/** Extra moves left in the run, counting the level being played; below zero on its last level. */
	const spare = $derived(atFrontier ? BUDGET - spent(run!) - Math.max(0, game!.moves - game!.par) : 0);

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
		const g = game;
		if (!g) return;
		const [moves, solved] = [g.moves, g.solved];
		untrack(() => progress(g, moves, solved));
	});

	$effect(() => {
		const style = document.documentElement.style;
		const [front, back] = paper;
		if (!front) return;
		style.setProperty('--paper-front', front);
		style.setProperty('--paper-back', back);
		document.querySelector('link[rel="icon"]')?.setAttribute('href', swanUrl(front, back));
		const setAccent = (name: string, accent: ReturnType<typeof accentFor>) => {
			if (accent) {
				style.setProperty(`--${name}`, accent.accent);
				style.setProperty(`--on-${name}`, accent.onAccent);
			} else {
				style.removeProperty(`--${name}`);
				style.removeProperty(`--on-${name}`);
			}
		};
		setAccent('accent', accentFor(paper, dark ? BACKDROPS.dark : BACKDROPS.light));
		setAccent('card-accent', accentFor(paper, [CARD_BAR, ...BACKDROPS.light]));
	});

	$effect(() => {
		if (!cardOpen || !cardView) return;
		void [corners, order, locale()];
		tick().then(() => {
			const svg = cardView?.element();
			if (svg) png = toPng(svg);
		});
	});

	/** A hash like #ABC123/9 opens that puzzle; anything else, today's run where it was left. */
	function open(hash: string): void {
		const code = parseCode(decodeURIComponent(hash.slice(1)));
		const now = today();
		const [seed, asked] = code ?? [dailySeed(now), 0];
		day = seed === dailySeed(now) ? now : undefined;
		run = day === undefined ? undefined : loadRun(day);
		const level = run ? (asked >= 1 ? Math.min(asked, frontier(run)) : frontier(run)) : asked;
		for (const g of shelf.values()) g.free();
		shelf.clear();
		game?.free();
		game = new Game(seed, level, movesBefore(level));
		paper = paperColors(seed);
		closeCard();
		corners = cardShape = undefined;
		restartAsked = false;
		showUrl();
	}

	/** Moves already spent on a level: the run's count for the level it's on, else none. */
	function movesBefore(level: number): number {
		return run && !finished(run) && level === frontier(run) ? run.pending : 0;
	}

	/** Saves the run as its level is played, and ends it once a level is solved over budget. */
	function progress(g: Game, moves: number, solved: boolean): void {
		if (!run || day === undefined || g.seed !== dailySeed(day) || finished(run) || g.level !== frontier(run)) return;
		if (!solved) {
			if (moves !== run.pending) saveRun(day, (run = { ...run, pending: moves }));
			return;
		}
		saveRun(day, (run = { solved: [...run.solved, [moves, g.par]], pending: 0 }));
		if (!finished(run)) return;
		writeJson(solvedKey(), round(g.unfolded));
		cardShape = { flaps: g.flaps, faceUp: g.faceUp, unfolded: g.unfolded };
		loadCorners();
		setTimeout(() => {
			if (game === g) cardOpen = true;
		}, CARD_DELAY);
	}

	/** A neighbouring level: in today's run, only those reached; in a shared puzzle, any. */
	function neighbour(step: 1 | -1): number | undefined {
		if (!game) return;
		const level = game.level + step;
		if (level < 1) return;
		return !daily || level <= frontier(run!) ? level : undefined;
	}

	/** The run's last level as the card shows it, read from a fresh copy when it isn't on the board. */
	function loadCardShape(): void {
		if (cardShape || !run || !game) return;
		const g = new Game(game.seed, run.solved.length);
		const solved = readJson<Point[]>(solvedKey());
		cardShape = { flaps: g.flaps, faceUp: g.faceUp, unfolded: solved?.length === g.dots ? solved : g.sheet() };
		g.free();
	}

	/** The card's shape: as reshaped, else as the run's last level unfolded. */
	function loadCorners(): void {
		if (!cardShape) return;
		unfoldedCorners = cardShape.unfolded;
		const saved = readJson<Point[]>(shapeKey());
		reshaped = saved?.length === unfoldedCorners.length;
		corners = reshaped ? saved! : unfoldedCorners;
		const stack = readJson<number[]>(orderKey());
		order = stack?.length === cardShape.flaps.length ? stack : flatOrder();
	}

	function openCard(): void {
		loadCardShape();
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

	const flatOrder = () => (cardShape ? cardShape.flaps.map((_, k) => k) : []);
	const round = (pts: Point[]) => pts.map((p) => ({ x: +p.x.toFixed(4), y: +p.y.toFixed(4) }));

	function resetShape(): void {
		if (unfoldedCorners) corners = unfoldedCorners;
		order = flatOrder();
		reshaped = false;
		forget(shapeKey());
		forget(orderKey());
	}

	async function share(): Promise<void> {
		if (!png || !result) return;
		const outcome = await shareCard(await png, shareText(result, here()));
		if (outcome === 'copied') flash(t().cardCopied);
		else if (outcome === 'downloaded') flash(t().cardSaved);
	}

	async function shareResultLink(): Promise<void> {
		if (!result) return;
		const outcome = await shareLink(shareText(result, here()));
		if (outcome === 'copied') flash(t().linkCopied);
		else if (outcome === 'failed') flash(shareText(result, here()));
	}

	/** Today's run keeps its plain address, so a link never gives a level away; other puzzles link their code. */
	function copyLink(): void {
		if (game) copy(daily ? here() : `${here()}#${game.code}`, t().linkCopied);
	}

	/** Copies text, or shows it when the clipboard is blocked. */
	async function copy(text: string, done: string): Promise<void> {
		flash((await copyText(text)) ? done : text);
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
		game = shelf.get(level) ?? new Game(game.seed, level, movesBefore(level));
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
		const before = atFrontier ? game.moves : 0;
		game.free();
		game = new Game(seed, level, before);
	}

	function flash(text: string): void {
		notice = text;
		clearTimeout(noticeTimer);
		noticeTimer = setTimeout(() => (notice = ''), NOTICE_TIME);
	}

	function onkeydown(e: KeyboardEvent): void {
		if (e.metaKey || e.ctrlKey || e.altKey) return;
		if (cardOpen || helpOpen) {
			if (e.key === 'Escape') cardOpen = helpOpen = false;
			return;
		}
		if (!game) return;
		if (e.key !== 'r') restartAsked = false;
		const g = game;
		const select = () => (!g.solved ? g.select() : neighbour(1) !== undefined ? jump(1) : result && openCard());
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

<main>
	<header>
		<h1>unfold</h1>
		{#if game}
			<p class="meta">
				{#if daily}{formatDay(day!, locale())}{:else}<span class="code">{game.code}</span>{/if}
				{#if streakDays}<span class="streak" aria-label={t().streak(streakDays)}>· <Icon name="fire" />{streakDays}</span>{/if}
			</p>
		{/if}
	</header>

	<div class="board">
		{#if game}
			<Board {game} {showCursor} />
		{:else if failed}
			<p class="loading">{t().failed}</p>
		{:else}
			<p class="loading">{t().loading}</p>
		{/if}
	</div>

	{#if game}
		<p class="level">{t().level(game.level)} · {t().solvableIn(game.par)}</p>
		<p class="status" aria-live="polite">
			{#if restartAsked}
				<strong>{t().restartAgain}</strong>
			{:else if notice}
				{notice}
			{:else if game.solved}
				<strong>{t().solved(score(game.moves, game.par))}</strong>
			{:else}
				{t().moves(game.moves)}{#if atFrontier}{` · ${spare < 0 ? t().lastLevel : t().toSpare(spare)}`}{/if}
			{/if}
		</p>

		<nav>
			{#if levels}
				<button onclick={() => jump(-1)} disabled={neighbour(-1) === undefined} aria-label={t().previousLevel}>‹</button>
			{/if}
			<button class="icon" onclick={restart} aria-label={game.solved ? t().replay : t().restart} title={game.solved ? t().replay : t().restart}><Icon name="reset" /></button>
			{#if result}
				<button class="icon primary" onclick={openCard} aria-label={t().yourCard} title={t().yourCard}><Icon name="share" /></button>
			{:else}
				<button class="icon" onclick={copyLink} aria-label={t().copyLink} title={t().copyLink}
					><Icon name="link" /></button
				>
			{/if}
			<button class="icon" onclick={() => (helpOpen = true)} aria-label={t().howToPlay} title={t().howToPlay}><Icon name="help" /></button>
			{#if levels}
				<button class:primary={game.solved && neighbour(1) !== undefined} onclick={() => jump(1)} disabled={neighbour(1) === undefined} aria-label={t().nextLevel}
					>{game.solved && neighbour(1) !== undefined ? `${t().nextLevel} ›` : '›'}</button
				>
			{/if}
		</nav>

	{/if}
</main>

{#if helpOpen}
	<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && (helpOpen = false)}>
		<div class="pop help-pop" role="dialog" aria-modal="true" aria-label={t().howToPlay}>
			<div class="sheet help-sheet">
				<div class="languages" role="radiogroup" aria-label={t().language}>
					{#each Object.entries(LOCALES) as [code, m] (code)}
						<button role="radio" aria-checked={locale() === code} class:on={locale() === code} onclick={() => setLocale(code as Locale)}>{m.name}</button>
					{/each}
				</div>
				<h2>{t().howToPlay}</h2>
				{#each t().help as paragraph, i (i)}
					<p>{paragraph}</p>
				{/each}
				<dl class="keys">
					<dt>← →</dt>
					<dd>{t().keys.move}</dd>
					<dt>space</dt>
					<dd>{t().keys.select}</dd>
					<dt>esc</dt>
					<dd>{t().keys.putBack}</dd>
					<dt>r</dt>
					<dd>{t().keys.restart}</dd>
					<dt>c</dt>
					<dd>{t().keys.copy}</dd>
				</dl>
			</div>
			<button class="icon close" onclick={() => (helpOpen = false)} aria-label={t().close}><Icon name="close" /></button>
		</div>
	</div>
{/if}

{#if cardOpen && result && corners && cardShape}
	<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && closeCard()}>
		<div class="pop card-pop" role="dialog" aria-modal="true" aria-label={t().yourCard}>
			<div class="frame">
				<Card
					bind:this={cardView}
					{result}
					flaps={cardShape.flaps}
					faceUp={cardShape.faceUp}
					{corners}
					{order}
					host={location.host}
					hint={!readJson<boolean>(RESHAPED_KEY)}
					onchange={saveShape}
				/>
				<div class="bar" role="toolbar" aria-label={t().cardTools} style:background={CARD_BAR}>
					<button class="icon" onclick={resetShape} disabled={!reshaped} aria-label={t().resetShape} title={t().resetShape}>
						<Icon name="reset" />
					</button>
					<button class="icon share" onclick={share} aria-label={t().shareCard} title={t().shareCard}>
						<Icon name="share" />
					</button>
					<button class="icon" onclick={shareResultLink} aria-label={t().shareLink} title={t().shareLink}>
						<Icon name="link" />
					</button>
				</div>
			</div>
			<button class="icon close" onclick={closeCard} aria-label={t().close}><Icon name="close" /></button>
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
		text-align: center;
	}
	button.icon {
		display: grid;
		place-items: center;
		width: 2.75rem;
		height: 2.75rem;
		min-width: 0;
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
	.level {
		margin: 0;
		color: var(--muted);
		font-size: 0.9rem;
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
	.pop {
		position: relative;
		width: min(100%, 28rem);
		animation: pop-in 0.35s cubic-bezier(0.2, 1.3, 0.4, 1);
	}
	.pop button.close {
		position: absolute;
		top: -0.9rem;
		right: -0.9rem;
		width: 2.25rem;
		height: 2.25rem;
		border: 1px solid var(--line);
		background: var(--button);
		color: var(--text);
		box-shadow: 0 0.15rem 0.5rem rgb(0 0 0 / 0.25);
	}
	.sheet {
		max-height: calc(100svh - 2rem);
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		border-radius: 1.5rem;
		background: var(--bg);
		box-shadow: 0 1rem 3rem rgb(0 0 0 / 0.3);
	}
	.languages {
		display: flex;
		align-self: flex-start;
		gap: 0.25rem;
		margin-bottom: 1rem;
		padding: 0.2rem;
		border-radius: 999px;
		background: var(--line);
	}
	.languages button {
		min-width: 0;
		padding: 0.25rem 0.8rem;
		border: none;
		background: transparent;
		color: var(--muted);
		font-size: 0.85rem;
	}
	.languages button.on {
		background: var(--button);
		color: var(--text);
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
	.card-pop {
		width: min(100%, 28rem, calc(100svh - 8rem));
	}
	.card-pop .notice {
		position: absolute;
		top: calc(100% + 0.6rem);
		left: 0;
		right: 0;
		text-align: center;
		color: #f4f0e8;
	}
	.streak {
		white-space: nowrap;
	}
	.streak :global(svg) {
		display: inline-block;
		vertical-align: -0.12em;
		margin-right: 0.1em;
	}
	.frame {
		width: 100%;
		border-radius: 1rem;
		overflow: hidden;
		box-shadow: 0 0.5rem 2rem rgb(0 0 0 / 0.35);
	}
	.bar {
		display: flex;
		justify-content: space-around;
		padding: 0.35rem 0.5rem;
	}
	.bar button {
		border: none;
		background: transparent;
		color: #3a352f;
	}
	.bar button.share {
		background: var(--card-accent, #3a352f);
		color: var(--on-card-accent, #f4f0e8);
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
