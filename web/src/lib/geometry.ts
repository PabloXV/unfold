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

/** Where two segments meet, if they do. */
export function meet(a: Point, b: Point, c: Point, d: Point): Point | undefined {
	const r = { x: b.x - a.x, y: b.y - a.y };
	const s = { x: d.x - c.x, y: d.y - c.y };
	const denom = r.x * s.y - r.y * s.x;
	if (denom === 0) return;
	const t = ((c.x - a.x) * s.y - (c.y - a.y) * s.x) / denom;
	const u = ((c.x - a.x) * r.y - (c.y - a.y) * r.x) / denom;
	if (t < 0 || t > 1 || u < 0 || u > 1) return;
	return { x: a.x + r.x * t, y: a.y + r.y * t };
}

/** Strictly inside a triangle, so points on its edges don't count. */
export function inside([a, b, c]: Point[], p: Point): boolean {
	const side = (u: Point, v: Point) => (v.x - u.x) * (p.y - u.y) - (v.y - u.y) * (p.x - u.x);
	const [d1, d2, d3] = [side(a, b), side(b, c), side(c, a)];
	const eps = 1e-9;
	return (d1 > eps && d2 > eps && d3 > eps) || (d1 < -eps && d2 < -eps && d3 < -eps);
}

/** The point nearest `p` within `reach`, by index. */
export function nearestTo(pts: Point[], p: Point, reach = Infinity): number | undefined {
	let best: number | undefined;
	pts.forEach((q, i) => {
		const d = Math.hypot(q.x - p.x, q.y - p.y);
		if (d < reach) [best, reach] = [i, d];
	});
	return best;
}
