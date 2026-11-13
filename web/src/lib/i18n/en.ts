import { BUDGET } from '../run';

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
	board: (moves: number) => `Paper: ${moves} moves`,
	cardLabel: (date: string, score: string) => `Share card: ${date}, ${score}`,
	help: [
		'The paper is folded up. Tap a crease to unfold it, or fold it again, until the paper lies flat: no flap over another, and none turned over.',
		'Each fold is a move, and the smaller side always swings. Creases that can fold right now glow. A crease can only fold once nothing lies on the side that swings; tap one that\'s stuck and whatever is in the way lights up. Drag anywhere to turn the paper over, which is free.',
		`Each day is one run of levels that get harder. "Solvable in 4" is the fewest moves a level takes, and the run has ${BUDGET} extra moves to spend. Go over, finish that level, and the run is done. A new one comes tomorrow.`,
		'Replay any level you reached with ‹ ›, and share any you solved; tap the card\'s creases to fold the paper into a shape of your own first. The fire counts the days in a row you finished a run.'
	],
	keys: {
		levels: 'previous, next level',
		restart: 'restart',
		copy: 'copy link'
	}
};

export type Messages = typeof en;
