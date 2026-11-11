import { readFileSync } from 'node:fs';
import { initSync, Attempt, dailySeed } from '../src/lib/wasm/unfold_wasm.js';

initSync({ module: readFileSync(new URL('../src/lib/wasm/unfold_wasm_bg.wasm', import.meta.url)) });

/** FNV-1a over 64-bit little-endian words, as in core's tests. */
function fingerprint(attempt) {
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
	attempt.flaps().forEach(eatInt);
	attempt.slots().forEach(eatInt);
	attempt.shape().forEach(eatFloat);
	return h;
}

const known = [
	[0x000000, 1, 0xb2697debe2fa8861n],
	[0x000000, 5, 0x015fda193863ecb4n],
	[0xabc123, 9, 0x98b8e47df6bd0032n],
	[0xffffff, 20, 0xb1cf9c856490be4cn],
	[0xdc86b0, 3, 0xc25d7a263bbf4e32n],
	[0x7811db, 12, 0xdc8f8103018988dcn]
];
let failed = 0;
for (const [seed, level, hash] of known) {
	const attempt = new Attempt(seed, level);
	const got = fingerprint(attempt);
	if (got !== hash) {
		console.error(`${attempt.code}: got ${got.toString(16)}, want ${hash.toString(16)}`);
		failed++;
	}
	attempt.free();
}
for (const [day, seed] of [[0, 0x50dc75], [1, 0xb62b7b], [20000, 0xdc86b0], [20400, 0x7811db]]) {
	if (dailySeed(day) !== seed) {
		console.error(`day ${day}: got ${dailySeed(day).toString(16)}, want ${seed.toString(16)}`);
		failed++;
	}
}
if (failed) process.exit(1);
console.log(`wasm matches native: ${known.length} puzzles, 4 daily seeds`);
