import type { Point } from './unfold';

/** Twice a triangle's signed area: positive when its corners run counterclockwise. */
export function turn([a, b, c]: Point[]): number {
	return (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
}

/** Flat x, y pairs as points. */
export function points(xy: ArrayLike<number>): Point[] {
	return Array.from({ length: xy.length / 2 }, (_, i) => ({ x: xy[2 * i], y: xy[2 * i + 1] }));
}

/** A triangle as an SVG path with y flipped, always wound the same way so overlapping ones fill as one. */
export function trianglePath(corners: Point[]): string {
	let [a, b, c] = corners;
	if (turn(corners) < 0) [b, c] = [c, b];
	return `M${a.x} ${-a.y}L${b.x} ${-b.y}L${c.x} ${-c.y}Z`;
}

/** A pointer's position in an SVG's own coordinates, y flipped to point up. */
export function svgPoint(svg: SVGSVGElement, e: PointerEvent): Point {
	const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(svg.getScreenCTM()!.inverse());
	return { x: p.x, y: -p.y };
}

export function inside([a, b, c]: Point[], p: Point): boolean {
	const s = [turn([a, b, p]), turn([b, c, p]), turn([c, a, p])];
	return s.every((d) => d > 1e-9) || s.every((d) => d < -1e-9);
}

const along = (pos: Point[], d: Point) => pos.map((p) => p.x * d.x + p.y * d.y);

/** How wide the paper is along a direction. */
export function width(pos: Point[], d: Point): number {
	const s = along(pos, d);
	return Math.max(...s) - Math.min(...s);
}

/** The paper turned over part way along a direction: squeezed toward its middle, mirrored once t reaches 1. */
export function squeeze(pos: Point[], d: Point, t: number): Point[] {
	const s = along(pos, d);
	const mid = (Math.min(...s) + Math.max(...s)) / 2;
	const k = Math.cos(Math.PI * t) - 1;
	return pos.map((p, v) => ({ x: p.x + (s[v] - mid) * k * d.x, y: p.y + (s[v] - mid) * k * d.y }));
}

/** The corners moved and scaled so the paper's extremes are centered and its farthest corner sits on the unit circle. */
export function fit(pos: Point[]): Point[] {
	const [xs, ys] = [pos.map((p) => p.x), pos.map((p) => p.y)];
	const [cx, cy] = [(Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2];
	const r = Math.max(...pos.map((p) => Math.hypot(p.x - cx, p.y - cy))) || 1;
	return pos.map((p) => ({ x: (p.x - cx) / r, y: (p.y - cy) / r }));
}

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
