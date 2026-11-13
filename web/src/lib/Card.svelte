<script lang="ts">
	import { FIRE } from './Icon.svelte';
	import { t } from './i18n/index.svelte';
	import Paper from './Paper.svelte';
	import { FONT, SIZE, runLine, type Result } from './card';
	import { blocking, creases, loose, play, swinger, tapped } from './fold';
	import { fit } from './free';
	import { svgPoint, trianglePath } from './geometry';
	import { CAST, DEFAULT_PAPER, LIGHT } from './theme';
	import type { Point } from './unfold';

	let {
		result,
		flaps,
		faceUp,
		corners,
		order,
		onchange
	}: {
		result: Result;
		flaps: number[][];
		faceUp: number[];
		/** Where each corner is, fitted into the unit circle. */
		corners: Point[];
		/** Flap indices bottom to top. */
		order: number[];
		onchange?: (corners: Point[], order: number[]) => void;
	} = $props();

	/** The paper's middle and scale on the card. */
	const CX = SIZE / 2;
	const CY = 500;
	const SCALE = 280;
	/** How near a tap must land to a crease to fold it, how long the fold takes, and how long blocking flaps light up, in ms. */
	const TAP = 0.2;
	const SWING = 320;
	const BLINK = 700;
	const LINE = 48;
	const FLAME = 46;
	/** The grid of levels above the result line: square size, the gap between, and its baseline. */
	const SQUARE = 26;
	const SQUARE_GAP = 10;
	const GRID_Y = 872;

	const css = getComputedStyle(document.documentElement);
	const PAPER = {
		front: css.getPropertyValue('--paper-front').trim() || DEFAULT_PAPER.front,
		back: css.getPropertyValue('--paper-back').trim() || DEFAULT_PAPER.back,
		cast: CAST,
		castStep: '0.032'
	};

	let svg = $state<SVGSVGElement | undefined>(undefined);
	let pos = $state.raw<Point[]>([]);
	let stack = $state.raw<number[]>([]);
	let folding = $state(false);
	let lit = $state<Set<number> | undefined>(undefined);
	let unlight = 0;
	let settling: (() => void) | undefined;

	const all = $derived(creases(flaps));
	const pulse = $derived(folding || !pos.length ? new Set<string>() : loose(all, pos, flaps, stack));

	$effect.pre(() => {
		pos = corners;
	});

	$effect.pre(() => {
		stack = order;
	});

	$effect(() => () => {
		clearTimeout(unlight);
		settling?.();
	});

	let scoreText = $state<SVGTextElement | undefined>(undefined);
	let countText = $state<SVGTextElement | undefined>(undefined);
	/** The drawn widths of the score and the streak count, measured since any language's glyphs may appear. */
	let widths = $state({ score: 0, count: 0 });

	const texts = $derived({
		text: result.streak ? `${runLine(result)} · ` : runLine(result),
		count: result.streak ? String(result.streak) : ''
	});

	$effect(() => {
		void texts;
		widths = { score: scoreText?.getComputedTextLength() ?? 0, count: countText?.getComputedTextLength() ?? 0 };
	});

	/** The result line, then the streak's flame and count, laid out as one centered row. */
	const line = $derived.by(() => {
		const x = CX - (widths.score + (texts.count ? FLAME + widths.count : 0)) / 2;
		return { ...texts, x, flameX: x + widths.score, countX: x + widths.score + FLAME };
	});

	/** Each level's square, with how it's drawn: filled, half filled, or open. */
	const squares = $derived.by(() => {
		const width = result.overs.length * (SQUARE + SQUARE_GAP) - SQUARE_GAP;
		return result.overs.map((over, i) => ({
			x: CX - width / 2 + i * (SQUARE + SQUARE_GAP),
			kind: over === 0 ? 'full' : over <= 2 ? 'half' : 'open'
		}));
	});

	const silhouette = $derived(flaps.map((f) => trianglePath(f.map((v) => pos[v]))).join(''));

	function cardPoint(e: PointerEvent): Point {
		const p = svgPoint(svg!, e);
		return { x: (p.x - CX) / SCALE, y: (p.y + CY) / SCALE };
	}

	/** Folds the crease tapped, keeping the paper fitted on the card, or lights what keeps it from folding. */
	function onpointerup(e: PointerEvent): void {
		if (e.button > 0) return;
		settling?.();
		const c = tapped(pos, flaps, stack, all, cardPoint(e), TAP);
		if (!c) return;
		const moving = swinger(c, all, pos, flaps, stack);
		if (!moving) return light(blocking(c, all, pos, flaps, stack));
		folding = true;
		settling = play(
			SWING,
			c,
			moving,
			pos,
			flaps,
			stack,
			(p, s) => ([pos, stack] = [fit(p), s]),
			() => {
				settling = undefined;
				folding = false;
				onchange?.(pos, stack);
			}
		);
	}

	/** Lights up the flaps keeping a tapped crease from folding, afresh on every tap. */
	function light(which: number[]): void {
		clearTimeout(unlight);
		lit = undefined;
		requestAnimationFrame(() => {
			lit = new Set(which);
			unlight = window.setTimeout(() => (lit = undefined), BLINK);
		});
	}

	export function element(): SVGSVGElement | undefined {
		return svg;
	}
</script>

<svg
	bind:this={svg}
	class="card"
	viewBox="0 0 {SIZE} {SIZE}"
	role="img"
	aria-label={t().cardLabel(result.date, `${runLine(result)} ${result.grid}`) + (result.streak ? `, ${t().streak(result.streak)}` : '')}
	{onpointerup}
>
	<defs>
		<radialGradient id="card-spot" cx={CX} cy={CY} r={SIZE * 0.62} gradientUnits="userSpaceOnUse">
			<stop offset="0" stop-color={LIGHT.spot} />
			<stop offset="1" stop-color={LIGHT.bg} />
		</radialGradient>
		<filter id="card-shadow" x="-25%" y="-25%" width="150%" height="150%">
			<feGaussianBlur in="SourceAlpha" stdDeviation="0.02" />
			<feOffset dx="0.02" dy="0.04" result="blur" />
			<feFlood flood-color={CAST} flood-opacity="0.26" />
			<feComposite in2="blur" operator="in" />
		</filter>
	</defs>

	<rect width={SIZE} height={SIZE} fill="url(#card-spot)" />
	<text x="80" y="140" fill={LIGHT.text} font-family={FONT} font-size="64" font-weight="700">unfold</text>
	<text x={SIZE - 80} y="140" fill={LIGHT.muted} font-family={FONT} font-size="40" text-anchor="end">{result.date}</text>

	<g transform="translate({CX} {CY}) scale({SCALE})">
		<g filter="url(#card-shadow)"><path d={silhouette} fill="#000" /></g>
		<Paper
			flaps={stack.map((k) => flaps[k])}
			faceUp={stack.map((k) => faceUp[k])}
			{pos}
			colors={PAPER}
			creaseWidth={0.009}
			{pulse}
			lit={lit && new Set(stack.flatMap((k, i) => (lit!.has(k) ? [i] : [])))}
			id="card"
		/>
	</g>

	{#each squares as s, i (i)}
		<rect
			x={s.x + 1.5}
			y={GRID_Y - SQUARE + 1.5}
			width={SQUARE - 3}
			height={SQUARE - 3}
			rx="4"
			fill={s.kind === 'open' ? 'none' : LIGHT.text}
			fill-opacity={s.kind === 'half' ? 0.4 : 1}
			stroke={LIGHT.text}
			stroke-width="3"
		/>
	{/each}
	<g fill={LIGHT.text} font-family={FONT} font-size={LINE} font-weight="700">
		<text bind:this={scoreText} x={line.x} y="960" xml:space="preserve">{line.text}</text>
		{#if line.count}
			<path d={FIRE} transform="translate({line.flameX} {960 - FLAME * 0.86}) scale({FLAME / 24})" />
			<text bind:this={countText} x={line.countX} y="960">{line.count}</text>
		{/if}
	</g>
</svg>

<style>
	.card {
		display: block;
		width: 100%;
		height: auto;
		touch-action: none;
		user-select: none;
		-webkit-user-select: none;
	}
</style>
