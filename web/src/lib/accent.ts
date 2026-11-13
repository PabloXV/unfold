const channels = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);

const toLinear = (v: number) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
const fromLinear = (v: number) => (v <= 0.0031308 ? v * 12.92 : 1.055 * v ** (1 / 2.4) - 0.055);

/** WCAG relative luminance of a #rrggbb color. */
function luminance(hex: string): number {
	const [r, g, b] = channels(hex).map(toLinear);
	return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG contrast ratio, from 1 to 21. */
export function contrast(a: string, b: string): number {
	const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
	return (x + 0.05) / (y + 0.05);
}

type Lch = { l: number; c: number; h: number };

function oklch(hex: string): Lch {
	const [r, g, b] = channels(hex).map(toLinear);
	const [l, m, s] = [
		0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b,
		0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b,
		0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b
	].map(Math.cbrt);
	const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
	const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
	const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
	return { l: L, c: Math.hypot(A, B), h: Math.atan2(B, A) };
}

/** Linear sRGB of an OKLCH color, possibly outside 0..1. */
function linearRgb({ l: L, c, h }: Lch): number[] {
	const [A, B] = [c * Math.cos(h), c * Math.sin(h)];
	const [l, m, s] = [L + 0.3963377774 * A + 0.2158037573 * B, L - 0.1055613458 * A - 0.0638541728 * B, L - 0.0894841775 * A - 1.291485548 * B].map(
		(v) => v ** 3
	);
	return [
		4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
		-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
		-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s
	];
}

/** The color as #rrggbb, losing chroma rather than hue when sRGB can't show it. */
function toHex(color: Lch): string {
	const fits = (c: number) => linearRgb({ ...color, c }).every((v) => v >= -1e-4 && v <= 1 + 1e-4);
	let c = color.c;
	if (!fits(c)) {
		let [lo, hi] = [0, c];
		for (let i = 0; i < 20; i++) [lo, hi] = fits((lo + hi) / 2) ? [(lo + hi) / 2, hi] : [lo, (lo + hi) / 2];
		c = lo;
	}
	const rgb = linearRgb({ ...color, c }).map((v) => Math.round(Math.min(1, Math.max(0, fromLinear(v))) * 255));
	return `#${rgb.map((v) => v.toString(16).padStart(2, '0')).join('')}`;
}

const colorfulness = (hex: string) => Math.max(...channels(hex)) - Math.min(...channels(hex));

const MIN_CONTRAST = 3;
const MIN_CHROMA = 0.04;
const passes = (hex: string, backdrops: string[]) => backdrops.every((b) => contrast(hex, b) >= MIN_CONTRAST);

/** The color at the nearest lightness, same hue, with 3:1 contrast against every backdrop. */
function reachContrast(hex: string, backdrops: string[]): string | undefined {
	if (passes(hex, backdrops)) return hex;
	const color = oklch(hex);
	for (let step = 1; step <= 100; step++) {
		for (const l of [color.l - step / 100, color.l + step / 100]) {
			if (l < 0 || l > 1) continue;
			const out = toHex({ ...color, l });
			if (passes(out, backdrops)) return out;
		}
	}
}

/** The most colorful paper color that stands out, else the most colorful one shifted in lightness until it does. */
export function accentFor(colors: string[], backdrops: string[]): { accent: string; onAccent: string } | undefined {
	const usable = colors.filter((c) => passes(c, backdrops) && oklch(c).c >= MIN_CHROMA);
	const pool = usable.length ? usable : colors;
	const accent = reachContrast(pool.reduce((a, b) => (colorfulness(b) > colorfulness(a) ? b : a)), backdrops);
	if (!accent) return;
	const onAccent = contrast(accent, '#ffffff') >= contrast(accent, '#1c1c1c') ? '#ffffff' : '#1c1c1c';
	return { accent, onAccent };
}
