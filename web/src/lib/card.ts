// The share card: a day's finished shape on a postcard with the score, as a PNG, plus the same
// result as text for anywhere an image won't go.
import { turn } from './game.svelte';
import type { Point } from './unfold';

export type Result = {
	/** Like "Nov 10". */
	date: string;
	/** Moves and par per level of the day, in order. */
	moves: number[];
	pars: number[];
};

const SIZE = 1080;

/** Per level, Wordle style: 🟩 at par, 🟨 within two of it, ⬜ beyond. */
export function squares({ moves, pars }: Result): string {
	return moves.map((m, i) => (m <= pars[i] ? '🟩' : m <= pars[i] + 2 ? '🟨' : '⬜')).join('');
}

const total = (xs: number[]) => xs.reduce((a, b) => a + b, 0);

export function shareText(result: Result, url: string): string {
	const { moves, pars } = result;
	return `unfold ${result.date}\n${squares(result)}\n${total(moves)} moves · par ${total(pars)}\n${url}`;
}

/**
 * Draws the card. `flaps` and `faceUp` are as in Game; `corners` are where each vertex is now,
 * which may have been reshaped.
 */
export async function renderCard(
	result: Result,
	shape: { flaps: number[][]; faceUp: number[]; corners: Point[] },
	host: string
): Promise<Blob> {
	const canvas = document.createElement('canvas');
	canvas.width = canvas.height = SIZE;
	const ctx = canvas.getContext('2d')!;
	const css = getComputedStyle(document.documentElement);
	const front = css.getPropertyValue('--paper-front').trim() || '#d9473f';
	const back = css.getPropertyValue('--paper-back').trim() || '#f2c14e';
	const font = 'ui-rounded, "SF Pro Rounded", system-ui, -apple-system, "Segoe UI", sans-serif';

	ctx.fillStyle = '#f4f0e8';
	ctx.fillRect(0, 0, SIZE, SIZE);

	ctx.fillStyle = '#2a2723';
	ctx.font = `600 64px ${font}`;
	ctx.textBaseline = 'alphabetic';
	ctx.fillText('unfold', 80, 140);
	ctx.fillStyle = '#7a736a';
	ctx.font = `400 40px ${font}`;
	ctx.textAlign = 'right';
	ctx.fillText(result.date, SIZE - 80, 140);
	ctx.textAlign = 'left';

	// the shape, fitted into the middle of the card, y up
	const { flaps, faceUp, corners } = shape;
	const xs = corners.map((p) => p.x);
	const ys = corners.map((p) => p.y);
	const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
	const scale = Math.min(720 / (x1 - x0 || 1), 560 / (y1 - y0 || 1));
	const [cx, cy] = [(x0 + x1) / 2, (y0 + y1) / 2];
	const at = (p: Point): [number, number] => [SIZE / 2 + (p.x - cx) * scale, 520 - (p.y - cy) * scale];
	const outline = (f: number[]) => {
		const [a, b, c] = f.map((v) => at(corners[v]));
		ctx.moveTo(...a);
		ctx.lineTo(...b);
		ctx.lineTo(...c);
		ctx.closePath();
	};

	// one soft shadow under the whole sheet, then the flaps bottom to top
	ctx.save();
	ctx.shadowColor = 'rgba(60, 45, 25, 0.3)';
	ctx.shadowBlur = 40;
	ctx.shadowOffsetX = 10;
	ctx.shadowOffsetY = 20;
	ctx.fillStyle = front;
	ctx.beginPath();
	for (const f of flaps) {
		// wound one way, so overlaps fill as one shape
		const pts = f.map((v) => corners[v]);
		outline(turn(pts) > 0 ? f : [f[0], f[2], f[1]]);
	}
	ctx.fill('nonzero');
	ctx.restore();
	flaps.forEach((f, k) => {
		ctx.beginPath();
		outline(f);
		ctx.fillStyle = Math.sign(turn(f.map((v) => corners[v]))) === faceUp[k] ? front : back;
		ctx.fill();
		// hairline of the same color closes the seams between neighbours
		ctx.strokeStyle = ctx.fillStyle;
		ctx.lineWidth = 1;
		ctx.stroke();
	});

	// a square per level, then the score
	const colors = { '🟩': '#2f7d6d', '🟨': '#d9a23a', '⬜': '#d9d2c5' };
	const marks = [...squares(result)];
	const sq = 56;
	const gap = 16;
	let x = SIZE / 2 - (marks.length * sq + (marks.length - 1) * gap) / 2;
	for (const m of marks) {
		ctx.fillStyle = colors[m as keyof typeof colors];
		ctx.beginPath();
		ctx.roundRect(x, 860, sq, sq, 12);
		ctx.fill();
		x += sq + gap;
	}
	ctx.fillStyle = '#2a2723';
	ctx.font = `600 44px ${font}`;
	ctx.textAlign = 'center';
	ctx.fillText(`${total(result.moves)} moves · par ${total(result.pars)}`, SIZE / 2, 976);
	ctx.fillStyle = '#7a736a';
	ctx.font = `400 30px ${font}`;
	ctx.fillText(host, SIZE / 2, 1024);

	return new Promise((done, fail) => canvas.toBlob((b) => (b ? done(b) : fail(new Error('no image'))), 'image/png'));
}

/**
 * Shares the card the best way this browser can: the share sheet with the image and text on
 * phones, else the image on the clipboard, else a download.
 */
export async function shareCard(blob: Blob, text: string): Promise<'shared' | 'copied' | 'downloaded' | 'cancelled'> {
	const file = new File([blob], 'unfold.png', { type: 'image/png' });
	if (navigator.canShare?.({ files: [file] })) {
		try {
			await navigator.share({ files: [file], text });
			return 'shared';
		} catch (e) {
			if ((e as Error).name === 'AbortError') return 'cancelled';
		}
	}
	try {
		await navigator.clipboard.write([
			new ClipboardItem({ 'image/png': blob, 'text/plain': new Blob([text], { type: 'text/plain' }) })
		]);
		return 'copied';
	} catch {
		const a = document.createElement('a');
		a.href = URL.createObjectURL(blob);
		a.download = 'unfold.png';
		a.click();
		setTimeout(() => URL.revokeObjectURL(a.href), 1000);
		return 'downloaded';
	}
}
