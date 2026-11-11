import regular from '@fontsource/courier-prime/files/courier-prime-latin-400-normal.woff2?url';
import bold from '@fontsource/courier-prime/files/courier-prime-latin-700-normal.woff2?url';

export type Result = {
	/** Like "Nov 10, 2026", in the reader's locale. */
	date: string;
	moves: number;
	par: number;
};

/** The card's width and height in pixels. */
export const SIZE = 1080;

export const FONT = '"Courier Prime", "Courier New", monospace';

/** The result as text ending with the link, which is what people tap. */
export function shareText({ date, moves, par }: Result, url: string): string {
	return `unfold ${date}\n${moves} moves · par ${par}\n${url}`;
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

/** The card's svg as a PNG, leaving out anything with the class `no-export`. */
export async function toPng(svg: SVGSVGElement, size = SIZE): Promise<Blob> {
	const copy = svg.cloneNode(true) as SVGSVGElement;
	copy.querySelectorAll('.no-export').forEach((el) => el.remove());
	const style = document.createElementNS('http://www.w3.org/2000/svg', 'style');
	style.textContent = await embeddedFonts();
	copy.prepend(style);
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
	try {
		await navigator.clipboard.writeText(text);
		return 'copied';
	} catch {
		return 'failed';
	}
}

/** Shares with the share sheet, else copies to the clipboard, else downloads. */
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
