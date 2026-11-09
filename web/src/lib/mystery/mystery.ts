// Mystery shapes: each is a picture (an animal emoji, traced by tools/shapes/make_shapes.py)
// drawn as a sheet at more corners each level. Solving a level unfolds it in color, and each
// solved level earns one guess at what it is.
import data from './shapes.json';
import { Game } from '../game.svelte';
import { Attempt, dailySeed } from '../unfold';

export type Level = { corners: number[]; triangles: number[]; colors: string[] };
export type Shape = { name: string; emoji: string; aliases: string[]; levels: Level[] };

export const shapes: Shape[] = data.shapes;
export const LEVELS = data.levelCorners.length;

/** Every name a guess can be, canonical names and decoys, for the suggestion list. */
export const guessable: string[] = [...new Set([...shapes.map((s) => s.name), ...data.decoys])].sort();

const canonical = new Map<string, string>();
for (const s of shapes) for (const n of [s.name, ...s.aliases]) canonical.set(n, s.name);
for (const n of data.decoys) if (!canonical.has(n)) canonical.set(n, n);

function normalize(text: string): string {
	return text
		.toLowerCase()
		.trim()
		.replace(/^(an?|the)\s+/, '')
		.replace(/\s+/g, ' ');
}

/** The listed animal a guess names, if it names one. Plurals count. */
export function resolveGuess(text: string): string | undefined {
	const t = normalize(text);
	return canonical.get(t) ?? canonical.get(t.replace(/e?s$/, ''));
}

/** A level of a shape, ready to play, with each flap's color in the game's stacking order. */
export function playLevel(shapeIndex: number, level: number): { game: Game; colors: string[] } {
	const lv = shapes[shapeIndex].levels[level - 1];
	const attempt = Attempt.fromSheet(dailySeed(10_000 + shapeIndex), level, new Float64Array(lv.corners), new Uint32Array(lv.triangles));
	const game = new Game(attempt, { upright: true });
	// the core shuffles the stacking order, so colors are matched back by corners
	const key = (t: number[]) => [...t].sort((a, b) => a - b).join(',');
	const byCorners = new Map<string, string>();
	for (let i = 0; i < lv.colors.length; i++) byCorners.set(key(lv.triangles.slice(3 * i, 3 * i + 3)), lv.colors[i]);
	return { game, colors: game.flaps.map((f) => byCorners.get(key(f)) ?? '#cccccc') };
}

const PLAYED = 'unfold:mystery:played';

/** A shape not among the last few played here, at random. */
export function pickShape(): number {
	let played: string[] = [];
	try {
		played = JSON.parse(localStorage.getItem(PLAYED) ?? '[]');
	} catch {
		// a fresh pick is fine
	}
	const fresh = shapes.map((_, i) => i).filter((i) => !played.includes(shapes[i].name));
	const pool = fresh.length ? fresh : shapes.map((_, i) => i);
	return pool[Math.floor(Math.random() * pool.length)];
}

export function markPlayed(shapeIndex: number): void {
	try {
		const played: string[] = JSON.parse(localStorage.getItem(PLAYED) ?? '[]');
		const next = [shapes[shapeIndex].name, ...played.filter((n) => n !== shapes[shapeIndex].name)].slice(0, 40);
		localStorage.setItem(PLAYED, JSON.stringify(next));
	} catch {
		// forgetting is fine
	}
}
