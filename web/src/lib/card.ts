// The share card: a day's finished shape on a postcard with the score, drawn by Card.svelte and
// exported as a PNG, plus the same result as text, with the link people can tap.

export type Result = {
	/** Like "Nov 10". */
	date: string;
	/** Moves and par per level of the day, in order. */
	moves: number[];
	pars: number[];
};

/** The card's size in pixels, square, which suits every app it's shared to. */
export const SIZE = 1080;

/** Per level, Wordle style: 🟩 at par, 🟨 within two of it, ⬜ beyond. */
export function squares({ moves, pars }: Result): string {
	return moves.map((m, i) => (m <= pars[i] ? '🟩' : m <= pars[i] + 2 ? '🟨' : '⬜')).join('');
}

export const total = (xs: number[]) => xs.reduce((a, b) => a + b, 0);

/** The result as text, ending with the link: shared alongside the card, it's what people tap. */
export function shareText(result: Result, url: string): string {
	const { moves, pars } = result;
	return `unfold ${result.date}\n${squares(result)}\n${total(moves)} moves · par ${total(pars)}\n${url}`;
}

/**
 * The card as a PNG: its <svg> drawn onto a canvas. The svg has to carry its own colors, since an
 * image can't see the page's CSS; anything with the class `no-export` is left out.
 */
export async function toPng(svg: SVGSVGElement, size = SIZE): Promise<Blob> {
	const copy = svg.cloneNode(true) as SVGSVGElement;
	copy.querySelectorAll('.no-export').forEach((el) => el.remove());
	copy.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
	copy.setAttribute('width', String(size));
	copy.setAttribute('height', String(size));
	const url = URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(copy)], { type: 'image/svg+xml' }));
	try {
		const img = new Image();
		img.src = url;
		await img.decode();
		const canvas = document.createElement('canvas');
		canvas.width = canvas.height = size;
		canvas.getContext('2d')!.drawImage(img, 0, 0, size, size);
		return await new Promise((done, fail) => canvas.toBlob((b) => (b ? done(b) : fail(new Error('no image'))), 'image/png'));
	} finally {
		URL.revokeObjectURL(url);
	}
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
