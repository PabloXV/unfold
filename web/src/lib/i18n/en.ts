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
		'Tap creases to unfold the paper until it lies flat: no flap over another, none turned over.',
		'Each fold is a move. Glowing creases can fold now; tap a stuck one to see what\'s in the way. Some flaps are folded behind: drag to turn the paper over, for free.',
		`One run a day, each level harder. "Solvable in 4" is the fewest moves; the run has ${BUDGET} extra to spend. Go over and the run ends with that level.`,
		'Replay levels with ‹ ›. Share any you solved, folding the card into your own shape first. The fire is your streak.'
	],
	keys: {
		levels: 'previous, next level',
		restart: 'restart',
		copy: 'copy link'
	}
};

export type Messages = typeof en;
