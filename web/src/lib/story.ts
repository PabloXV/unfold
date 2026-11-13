import { contrast } from './accent';
import { FONT, SIZE } from './card';
import { LIGHT } from './theme';

const STORY = { width: 1080, height: 1920 };
const CARD = 880;
const CARD_TOP = 420;
const RADIUS = 40;

const darker = (a: string, b: string) => (contrast(a, '#000000') < contrast(b, '#000000') ? a : b);

/** A triangle grid over the canvas, calling `paint` with each triangle's corners and its row and column. */
function triangles(side: number, paint: (pts: number[][], row: number, col: number) => void): void {
	const h = (side * Math.sqrt(3)) / 2;
	for (let row = -1; row * h < STORY.height + h; row++) {
		for (let col = -2; (col * side) / 2 < STORY.width + side; col++) {
			const x = (col * side) / 2 - (row % 2 ? side / 2 : 0);
			const up = (row + col) % 2 === 0;
			const [top, bottom] = [row * h, (row + 1) * h];
			paint(up ? [[x, bottom], [x + side / 2, top], [x + side, bottom]] : [[x, top], [x + side, top], [x + side / 2, bottom]], row, col);
		}
	}
}

/** A fixed pseudo-random number in [0, 1) for a grid cell, so the pattern is the same every time. */
function hash(row: number, col: number): number {
	const x = Math.sin(row * 127.1 + col * 311.7) * 43758.5453;
	return x - Math.floor(x);
}

function fillPoly(ctx: CanvasRenderingContext2D, pts: number[][], color: string): void {
	ctx.beginPath();
	pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
	ctx.closePath();
	ctx.fillStyle = color;
	ctx.fill();
}

function vignette(ctx: CanvasRenderingContext2D, alpha: number): void {
	const g = ctx.createRadialGradient(STORY.width / 2, CARD_TOP + CARD / 2, CARD * 0.4, STORY.width / 2, CARD_TOP + CARD / 2, STORY.height * 0.75);
	g.addColorStop(0, 'rgb(0 0 0 / 0)');
	g.addColorStop(1, `rgb(0 0 0 / ${alpha})`);
	ctx.fillStyle = g;
	ctx.fillRect(0, 0, STORY.width, STORY.height);
}

/** The darker paper color as a folded sheet: triangles each lit a little differently, darkened toward the edges. */
function paintBackdrop(ctx: CanvasRenderingContext2D, front: string, back: string): void {
	ctx.fillStyle = darker(front, back);
	ctx.fillRect(0, 0, STORY.width, STORY.height);
	triangles(200, (pts, row, col) => {
		const lit = hash(row, col);
		fillPoly(ctx, pts, lit < 0.5 ? `rgb(255 255 255 / ${0.18 * lit})` : `rgb(0 0 0 / ${0.3 * (lit - 0.5)})`);
	});
	ctx.fillStyle = 'rgb(0 0 0 / 0.3)';
	ctx.fillRect(0, 0, STORY.width, STORY.height);
	vignette(ctx, 0.55);
}

/** The card as a tall image for stories: rounded, with a shadow, on the day's paper folded behind it. */
export async function toStoryPng(card: Blob, front: string, back: string, host: string): Promise<Blob> {
	const canvas = document.createElement('canvas');
	canvas.width = STORY.width;
	canvas.height = STORY.height;
	const ctx = canvas.getContext('2d')!;
	paintBackdrop(ctx, front, back);
	const x = (STORY.width - CARD) / 2;
	const image = await createImageBitmap(card);
	ctx.save();
	ctx.shadowColor = 'rgb(0 0 0 / 0.55)';
	ctx.shadowBlur = 70;
	ctx.shadowOffsetY = 28;
	ctx.beginPath();
	ctx.roundRect(x, CARD_TOP, CARD, CARD, RADIUS);
	ctx.fillStyle = LIGHT.bg;
	ctx.fill();
	ctx.restore();
	ctx.save();
	ctx.beginPath();
	ctx.roundRect(x, CARD_TOP, CARD, CARD, RADIUS);
	ctx.clip();
	ctx.drawImage(image, 0, 0, SIZE, SIZE, x, CARD_TOP, CARD, CARD);
	ctx.restore();
	await document.fonts.load(`700 52px ${FONT}`);
	ctx.font = `700 52px ${FONT}`;
	ctx.textAlign = 'center';
	ctx.fillStyle = 'rgb(255 255 255 / 0.85)';
	ctx.fillText(host, STORY.width / 2, CARD_TOP + CARD + 150);
	return new Promise((done, fail) => canvas.toBlob((b) => (b ? done(b) : fail(new Error('no image'))), 'image/png'));
}
