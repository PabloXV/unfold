import { readText, writeText } from './storage';

/** Fewest moves per puzzle as `SEED/LEVEL MOVES` lines, the terminal version's format. */
const KEY = 'unfold:scores';

function load(): Map<string, number> {
	const best = new Map<string, number>();
	for (const line of (readText(KEY) ?? '').split('\n')) {
		const [c, moves] = line.trim().split(' ');
		const n = Number(moves);
		if (c && Number.isInteger(n) && n > 0) best.set(c.toUpperCase(), n);
	}
	return best;
}

/** By puzzle code, like ABC123/9. */
export function bestMoves(code: string): number | undefined {
	return load().get(code);
}

export function record(code: string, moves: number): void {
	const best = load();
	if ((best.get(code) ?? Infinity) <= moves) return;
	best.set(code, moves);
	writeText(KEY, [...best].map(([c, n]) => `${c} ${n}\n`).join(''));
}
