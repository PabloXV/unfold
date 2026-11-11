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
		'沿著圓拖曳一個點，其他點會讓出位置。把點放到新的位置算一步。',
		'「6 步可解」是最少需要的步數，每多一步顯示 +1。火焰是你連續解題的天數。每天都有新謎題。',
		'解開後就分享你的卡片。分享前可以拖曳卡片的角，把紙摺成你自己的形狀。'
	],
	keys: {
		move: '沿著圓移動',
		select: '拿起、放下',
		putBack: '放回',
		restart: '重來',
		copy: '複製連結'
	}
};
