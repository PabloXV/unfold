import { readText, writeText } from './storage';
import { formatSeed } from './unfold';

/** Fewest moves per puzzle as `SEED/LEVEL MOVES` lines, the terminal version's format. */
const KEY = 'unfold:scores';

function code(seed: number, level: number): string {
	return `${formatSeed(seed)}/${level}`;
}

function load(): Map<string, number> {
	const best = new Map<string, number>();
	for (const line of (readText(KEY) ?? '').split('\n')) {
		const [c, moves] = line.trim().split(' ');
		const n = Number(moves);
		if (c && Number.isInteger(n) && n > 0) best.set(c.toUpperCase(), n);
	}
	return best;
}

export function bestMoves(seed: number, level: number): number | undefined {
	return load().get(code(seed, level));
}

export function record(seed: number, level: number, moves: number): void {
	const best = load();
	const c = code(seed, level);
	if ((best.get(c) ?? Infinity) <= moves) return;
	best.set(c, moves);
	writeText(KEY, [...best].map(([c, n]) => `${c} ${n}\n`).join(''));
}
