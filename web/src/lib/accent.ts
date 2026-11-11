const channels = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);

/** WCAG relative luminance of a #rrggbb color. */
function luminance(hex: string): number {
	const [r, g, b] = channels(hex).map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
	return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG contrast ratio, from 1 to 21. */
export function contrast(a: string, b: string): number {
	const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
	return (x + 0.05) / (y + 0.05);
}

const colorfulness = (hex: string) => Math.max(...channels(hex)) - Math.min(...channels(hex));

/** The most colorful of `colors` with 3:1 contrast against every backdrop, and text that reads on it. */
export function accentFor(colors: string[], backdrops: string[]): { accent: string; onAccent: string } | undefined {
	const usable = colors.filter((c) => backdrops.every((b) => contrast(c, b) >= 3));
	if (!usable.length) return;
	const accent = usable.reduce((a, b) => (colorfulness(b) > colorfulness(a) ? b : a));
	const onAccent = contrast(accent, '#ffffff') >= contrast(accent, '#1c1c1c') ? '#ffffff' : '#1c1c1c';
	return { accent, onAccent };
}
