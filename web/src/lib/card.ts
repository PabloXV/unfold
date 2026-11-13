import regular from '@fontsource/courier-prime/files/courier-prime-latin-400-normal.woff2?url';
import bold from '@fontsource/courier-prime/files/courier-prime-latin-700-normal.woff2?url';
import { t } from './i18n/index.svelte';

export type Result = {
	/** Like "Nov 10, 2026", in the reader's locale. */
	date: string;
	/** The last level solved in the run. */
	level: number;
	/** Per level, moves over the fewest possible. */
	overs: number[];
	/** Like ■■▣□, one square per level. */
	grid: string;
	/** Days in a row with a finished run, counting this one. */
	streak: number;
	/** Extra moves left while the run is still going. */
	spare?: number;
};

/** Like "level 6", or "level 2 · 8 to spare" while the run is still going. */
export function runLine({ level, spare }: Result): string {
	const m = t();
	return spare === undefined ? m.level(level) : `${m.level(level)} · ${m.toSpare(spare)}`;
}

/** The card's width and height in pixels. */
export const SIZE = 1080;

export const FONT = '"Courier Prime", "Courier New", monospace';

/** Like "9 moves (+3)", over the fewest it can be done in, or "6 moves · perfect". */
export function score(moves: number, par: number): string {
	const m = t();
	return moves <= par ? m.perfect(m.moves(moves)) : m.over(m.moves(moves), moves - par);
}

/** The result as text ending with the link, which is what people tap. */
export function shareText(result: Result, url: string): string {
	return `unfold ${result.date} · ${runLine(result)}\n${result.grid}\n${url}`;
}

let fonts: Promise<string> | undefined;

/** Courier Prime as @font-face rules with the files inlined, since an exported image can't fetch them. */
function embeddedFonts(): Promise<string> {
	const inline = async (url: string, weight: number) => {
		const bytes = new Uint8Array(await (await fetch(url)).arrayBuffer());
		let bin = '';
		for (const b of bytes) bin += String.fromCharCode(b);
		return `@font-face{font-family:"Courier Prime";font-weight:${weight};src:url(data:font/woff2;base64,${btoa(bin)}) format("woff2")}`;
	};
	return (fonts ??= Promise.all([inline(regular, 400), inline(bold, 700)]).then((rules) => rules.join('')));
}

/** The card's svg as a PNG. */
export async function toPng(svg: SVGSVGElement): Promise<Blob> {
	const copy = svg.cloneNode(true) as SVGSVGElement;
	const style = document.createElementNS('http://www.w3.org/2000/svg', 'style');
	style.textContent = await embeddedFonts();
	copy.prepend(style);
	copy.querySelectorAll('.glow, .lit').forEach((e) => e.remove());
	copy.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
	copy.setAttribute('width', String(SIZE));
	copy.setAttribute('height', String(SIZE));
	const url = URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(copy)], { type: 'image/svg+xml' }));
	try {
		const img = new Image();
		img.src = url;
		await img.decode();
		const canvas = document.createElement('canvas');
		canvas.width = canvas.height = SIZE;
		canvas.getContext('2d')!.drawImage(img, 0, 0, SIZE, SIZE);
		return await new Promise((done, fail) => canvas.toBlob((b) => (b ? done(b) : fail(new Error('no image'))), 'image/png'));
	} finally {
		URL.revokeObjectURL(url);
	}
}

/** Copies text, saying whether the clipboard allowed it. */
export async function copyText(text: string): Promise<boolean> {
	try {
		await navigator.clipboard.writeText(text);
		return true;
	} catch {
		return false;
	}
}

/** Shares the text alone, which apps that drop text beside an image (like LINE) still post. */
export async function shareLink(text: string): Promise<'shared' | 'copied' | 'cancelled' | 'failed'> {
	if (navigator.share) {
		try {
			await navigator.share({ text });
			return 'shared';
		} catch (e) {
			if ((e as Error).name === 'AbortError') return 'cancelled';
		}
	}
	return (await copyText(text)) ? 'copied' : 'failed';
}

/** Shares with the share sheet, copying the link for apps that drop the text, else copies to the clipboard, else downloads. */
export async function shareCard(blob: Blob, text: string, link: string): Promise<'shared' | 'shared with link' | 'copied' | 'downloaded' | 'cancelled'> {
	const file = new File([blob], 'unfold.png', { type: 'image/png' });
	if (navigator.canShare?.({ files: [file] })) {
		const linkCopied = copyText(link);
		try {
			await navigator.share({ files: [file], text });
			return (await linkCopied) ? 'shared with link' : 'shared';
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
