import type { Point } from './unfold';

const EPS = 1e-6;

/** Whether two triangles share area, not just an edge or a corner. */
export function overlap(a: Point[], b: Point[]): boolean {
	for (const tri of [a, b]) {
		for (let i = 0; i < 3; i++) {
			const [p, q] = [tri[i], tri[(i + 1) % 3]];
			const n = { x: q.y - p.y, y: p.x - q.x };
			const span = (t: Point[]) => t.map((r) => n.x * (r.x - p.x) + n.y * (r.y - p.y));
			const [sa, sb] = [span(a), span(b)];
			if (Math.max(...sa) <= Math.min(...sb) + EPS || Math.max(...sb) <= Math.min(...sa) + EPS) return false;
		}
	}
	return true;
}

/** Every pair of flaps sharing area, as "low,high" flap indices. */
export function overlaps(flaps: number[][], pos: Point[]): Set<string> {
	const tris = flaps.map((f) => f.map((v) => pos[v]));
	const out = new Set<string>();
	for (let i = 0; i < tris.length; i++) for (let j = i + 1; j < tris.length; j++) if (overlap(tris[i], tris[j])) out.add(`${i},${j}`);
	return out;
}

/** The stack after `moving` flaps moved: old overlaps keep their order, new ones put the mover on top. */
export function restack(stack: number[], moving: Set<number>, before: Set<string>, now: Set<string>): number[] {
	const level = new Map(stack.map((k, i) => [k, i]));
	const above = new Map<number, Set<number>>(stack.map((k) => [k, new Set()]));
	for (const pair of now) {
		const [i, j] = pair.split(',').map(Number);
		if (before.has(pair)) {
			const [lo, hi] = level.get(i)! < level.get(j)! ? [i, j] : [j, i];
			above.get(hi)!.add(lo);
		} else if (moving.has(i) !== moving.has(j)) {
			const [mover, still] = moving.has(i) ? [i, j] : [j, i];
			above.get(mover)!.add(still);
		}
	}
	const out: number[] = [];
	const placed = new Set<number>();
	while (out.length < stack.length) {
		const ready = stack.filter((k) => !placed.has(k) && [...above.get(k)!].every((u) => placed.has(u)));
		const next = ready.length ? ready[0] : stack.find((k) => !placed.has(k))!;
		out.push(next);
		placed.add(next);
	}
	return out;
}
