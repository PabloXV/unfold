// Fewest moves per puzzle, kept in this browser as `SEED/LEVEL MOVES` lines, the same format as
// the terminal version's scores file. Storage can be missing or blocked (private windows, some
// embeds), so every read and write is allowed to fail: the game just forgets.
import { formatSeed } from './unfold';

const KEY = 'unfold:scores';

function code(seed: number, level: number): string {
	return `${formatSeed(seed)}/${level}`;
}

function load(): Map<string, number> {
	const best = new Map<string, number>();
	let text = '';
	try {
		text = localStorage.getItem(KEY) ?? '';
	} catch {
		return best;
	}
	for (const line of text.split('\n')) {
		const [c, moves] = line.trim().split(' ');
		const n = Number(moves);
		if (c && Number.isInteger(n) && n > 0) best.set(c.toUpperCase(), n);
	}
	return best;
}

export function bestMoves(seed: number, level: number): number | undefined {
	return load().get(code(seed, level));
}

export function firstUnsolved(seed: number): number {
	const best = load();
	let level = 1;
	while (best.has(code(seed, level))) level++;
	return level;
}

export function record(seed: number, level: number, moves: number): void {
	const best = load();
	const c = code(seed, level);
	if ((best.get(c) ?? Infinity) <= moves) return;
	best.set(c, moves);
	const text = [...best].map(([c, n]) => `${c} ${n}\n`).join('');
	try {
		localStorage.setItem(KEY, text);
	} catch {
		// losing a score isn't worth interrupting the game
	}
}
