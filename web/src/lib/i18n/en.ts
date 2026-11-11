export const en = {
	name: 'English',
	level: (n: number) => `level ${n}`,
	solvableIn: (n: number) => `solvable in ${n}`,
	moves: (n: number) => `${n} ${n === 1 ? 'move' : 'moves'}`,
	over: (score: string, n: number) => `${score} (+${n})`,
	perfect: (score: string) => `${score} · perfect`,
	solved: (score: string) => `Solved in ${score}`,
	streak: (n: number) => `${n} day streak`,
	toSpare: (n: number) => `${n} to spare`,
	lastLevel: 'last level',
	reached: (n: number) => `Reached level ${n}`,
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
		'Drag a dot along the circle; the others shift over to make room. Setting a dot down somewhere new is a move. To cancel, drag it far outside the circle.',
		'Each day is one run of levels that get harder. "Solvable in 4" is the fewest moves a level takes, and the run has 10 extra moves to spend. Go over, finish that level, and the run is done. A new one comes tomorrow.',
		'Replay any level you reached with ‹ ›, and share any you solved; drag the card\'s corners to fold the paper into a shape of your own first. The fire counts the days in a row you finished a run.'
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
