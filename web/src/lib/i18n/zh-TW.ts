import { BUDGET } from '../run';
import type { Messages } from './en';

export const zhTW: Messages = {
	name: '繁體中文',
	level: (n) => `第 ${n} 關`,
	solvableIn: (n) => `${n} 步可解`,
	moves: (n) => `${n} 步`,
	over: (score, n) => `${score} (+${n})`,
	perfect: (score) => `${score} · 完美`,
	solved: (score) => `完成：${score}`,
	streak: (n) => `連續 ${n} 天`,
	toSpare: (n) => `還可多 ${n} 步`,
	lastLevel: '最後一關',
	reached: (n) => `到達第 ${n} 關`,
	restartAgain: '再按一次重來',
	loading: '摺紙中…',
	failed: '這個瀏覽器無法載入謎題。',
	linkCopied: '已複製連結',
	cardCopied: '已複製卡片',
	cardSaved: '已儲存卡片',
	restart: '重來',
	replay: '再玩一次',
	yourCard: '你的卡片',
	copyLink: '複製連結',
	howToPlay: '玩法',
	close: '關閉',
	previousLevel: '上一關',
	nextLevel: '下一關',
	resetShape: '還原形狀',
	shareCard: '分享卡片',
	shareLink: '分享連結',
	cardTools: '卡片',
	language: '語言',
	board: (moves) => `紙：${moves} 步`,
	cardLabel: (date, score) => `分享卡片：${date}，${score}`,
	help: [
		'點摺痕把紙攤開，直到紙攤平：沒有一片疊在另一片上，也沒有一片翻面。',
		'每摺一次算一步。發光的摺痕現在能摺；點一下摺不動的，就會看到擋住它的部分。有些部分摺在背面：拖曳就能把紙翻過來，不算步數。',
		`每天一輪，關卡越來越難。「4 步可解」是最少的步數，整輪可以多用 ${BUDGET} 步。超過的話，這一關結束後這一輪就結束了。`,
		'用 ‹ › 重玩關卡。解開的關卡都能分享，分享前可以把卡片摺成你自己的形狀。火焰是你連續的天數。'
	],
	keys: {
		levels: '上一關、下一關',
		restart: '重來',
		copy: '複製連結'
	}
};
