import { readJson, writeJson } from './storage';

/** Moves over the fewest possible that a whole run can spend. */
export const BUDGET = 3;

/** A day's run: moves and fewest possible per solved level, and moves so far on the next one. */
export type Run = { solved: [moves: number, par: number][]; pending: number };

const key = (day: number) => `unfold:run:${day}`;

export function loadRun(day: number): Run {
	const run = readJson<Run>(key(day));
	return Array.isArray(run?.solved) && Number.isInteger(run.pending) ? run : { solved: [], pending: 0 };
}

export function saveRun(day: number, run: Run): void {
	writeJson(key(day), run);
}

/** Per solved level, how many moves over the fewest possible it took. */
export const overs = (run: Run) => run.solved.map(([moves, par]) => Math.max(0, moves - par));

export const spent = (run: Run) => overs(run).reduce((a, b) => a + b, 0);

/** Over budget, which ends the run once the level it went over on is solved. */
export const finished = (run: Run) => spent(run) > BUDGET;

/** The highest level that can be played: the one being worked on, or the last one once finished. */
export const frontier = (run: Run) => (finished(run) ? run.solved.length : run.solved.length + 1);

/** Days in a row with a finished run, up to today, or up to yesterday while today's is still going. */
export function streak(today: number): number {
	let day = finished(loadRun(today)) ? today : today - 1;
	let days = 0;
	for (; finished(loadRun(day)); day--) days++;
	return days;
}

/** One square per level: filled for the fewest possible, half for one or two over, open for more. */
export function grid(run: Run): string {
	return overs(run)
		.map((over) => (over === 0 ? '■' : over <= 2 ? '▣' : '□'))
		.join('');
}
