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
	board: (crossings, moves) => `謎題：${crossings} 個交叉，${moves} 步`,
	cardLabel: (date, score) => `分享卡片：${date}，${score}`,
	help: [
		'紙摺疊在一個圓上。移動圓上的點，直到摺痕不再交叉，紙就會展開。',
		'沿著圓拖曳一個點，其他點會讓出位置。把點放到新的位置算一步。要取消，就把點拖到圓外遠一點的地方。',
		'每天一輪，關卡越來越難。「4 步可解」是一關最少需要的步數，整輪共有 10 步可以多用。超過之後，解開那一關，這一輪就結束了。明天會有新的一輪。',
		'可以用 ‹ › 重玩已到達的關卡，也能分享任何解開的關卡；分享前可以拖曳卡片的角，把紙摺成你自己的形狀。火焰是你連續完成一輪的天數。'
	],
	keys: {
		move: '沿著圓移動',
		select: '拿起、放下',
		putBack: '放回',
		restart: '重來',
		copy: '複製連結'
	}
};
