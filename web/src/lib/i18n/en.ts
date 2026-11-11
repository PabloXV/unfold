export const en = {
	name: 'English',
	level: (n: number) => `level ${n}`,
	solvableIn: (n: number) => `solvable in ${n}`,
	moves: (n: number) => `${n} ${n === 1 ? 'move' : 'moves'}`,
	over: (score: string, n: number) => `${score} (+${n})`,
	perfect: (score: string) => `${score} · perfect`,
	solved: (score: string) => `Solved in ${score}`,
	streak: (n: number) => `${n} day streak`,
	restartAgain: 'Press restart again to start over',
	loading: 'folding…',
	failed: "This browser couldn't load the puzzle.",
	linkCopied: 'link copied',
	cardCopied: 'card copied',
	cardSaved: 'card saved',
	restart: 'Restart',
	replay: 'Replay',
	yourCard: 'Your card',
	copyLink: 'Copy link',
	howToPlay: 'How to play',
	close: 'Close',
	previousLevel: 'Previous level',
	nextLevel: 'Next level',
	resetShape: 'Reset shape',
	shareCard: 'Share card',
	shareLink: 'Share link',
	cardTools: 'Card',
	language: 'Language',
	board: (crossings: number, moves: number) => `Puzzle board: ${crossings} crossings, ${moves} moves`,
	cardLabel: (date: string, score: string) => `Share card: ${date}, ${score}`,
	help: [
		'The paper is folded up around a circle. Move its dots until no creases cross, and it unfolds.',
		'Drag a dot along the circle; the others shift over to make room. Setting a dot down somewhere new is a move.',
		'"Solvable in 6" is the fewest moves it can be done in; each move over shows as +1. The fire counts the days in a row you\'ve solved. There\'s a new puzzle every day.',
		"When it's solved, share your card. Drag its corners to fold the paper into a shape of your own first."
	],
	keys: {
		move: 'move around the circle',
		select: 'pick up, set down',
		putBack: 'put back',
		restart: 'restart',
		copy: 'copy link'
	}
};

export type Messages = typeof en;
