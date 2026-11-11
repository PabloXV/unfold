type Pt = [number, number];

const BODY: Pt[] = [[3, 13.9], [13.2, 48.5], [35.7, 48.5], [50.8, 33.3]];
const NECK: Pt[] = [[35.7, 48.5], [50.8, 33.3], [42.8, 13.5], [35.7, 19.5]];
const HEAD: Pt[] = [[35.7, 19.5], [42.8, 13.5], [59, 23.6]];

const points = (ps: Pt[]) => ps.map((p) => p.join(',')).join(' ');
const polygon = (ps: Pt[], fill: string) => `<polygon points="${points(ps)}" fill="${fill}"/>`;

/** A pressed crease from a to b: a dark line with a light one beside it, toward the top left. */
function crease([a, b]: Pt[]): string {
	const line = (dx: number, dy: number, color: string, opacity: number, width: number) =>
		`<line x1="${a[0] + dx}" y1="${a[1] + dy}" x2="${b[0] + dx}" y2="${b[1] + dy}" stroke="${color}" stroke-opacity="${opacity}" stroke-width="${width}" stroke-linecap="round"/>`;
	return line(-0.5, -0.5, '#fff', 0.5, 0.8) + line(0, 0, '#000', 0.3, 1);
}

/** The swan folded from one sheet, in the day's two paper colors, as an svg document. */
export function swanSvg(front: string, back: string): string {
	const shadow = `<g fill="#3c2d19" opacity="0.22" transform="translate(1 1.6)">${[BODY, NECK, HEAD].map((p) => polygon(p, 'inherit')).join('')}</g>`;
	return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">${shadow}${polygon(BODY, front)}${polygon(NECK, back)}${crease(NECK)}${polygon(HEAD, front)}${crease(HEAD)}</svg>`;
}

export const swanUrl = (front: string, back: string) => `data:image/svg+xml,${encodeURIComponent(swanSvg(front, back))}`;
