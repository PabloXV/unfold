import { readFileSync } from 'node:fs';
import { initSync, Puzzle, dailySeed } from '../src/lib/wasm/unfold_wasm.js';

initSync({ module: readFileSync(new URL('../src/lib/wasm/unfold_wasm_bg.wasm', import.meta.url)) });

/** FNV-1a over 64-bit little-endian words of the flaps and the shape's exact bits, recorded from native. */
function fingerprint(puzzle) {
	let h = 0xcbf29ce484222325n;
	const view = new DataView(new ArrayBuffer(8));
	const eatBits = () => {
		for (let i = 0; i < 8; i++) {
			h ^= BigInt(view.getUint8(i));
			h = (h * 0x100000001b3n) & 0xffffffffffffffffn;
		}
	};
	const eatInt = (n) => (view.setBigUint64(0, BigInt(n), true), eatBits());
	const eatFloat = (x) => (view.setFloat64(0, x, true), eatBits());
	puzzle.flaps().forEach(eatInt);
	puzzle.shape().forEach(eatFloat);
	return h;
}

const known = [
	[0x000000, 1, 0x723e0f5324d1ee81n],
	[0x000000, 5, 0xda2388d9923124b4n],
	[0xabc123, 9, 0x11c87c38eb352872n],
	[0xffffff, 20, 0xa35b1774db4e1377n],
	[0xdc86b0, 3, 0xa521501eda7ba95fn],
	[0x7811db, 12, 0x7d0b670c1641179fn]
];
let failed = 0;
for (const [seed, level, hash] of known) {
	const puzzle = new Puzzle(seed, level);
	const got = fingerprint(puzzle);
	if (got !== hash) {
		console.error(`${puzzle.code}: got ${got.toString(16)}, want ${hash.toString(16)}`);
		failed++;
	}
	puzzle.free();
}
for (const [day, seed] of [[0, 0x50dc75], [1, 0xb62b7b], [20000, 0xdc86b0], [20400, 0x7811db]]) {
	if (dailySeed(day) !== seed) {
		console.error(`day ${day}: got ${dailySeed(day).toString(16)}, want ${seed.toString(16)}`);
		failed++;
	}
}
if (failed) process.exit(1);
console.log(`wasm matches native: ${known.length} puzzles, 4 daily seeds`);
