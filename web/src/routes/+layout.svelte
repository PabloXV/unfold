<script lang="ts">
	import '@fontsource/courier-prime/400.css';
	import '@fontsource/courier-prime/700.css';
	import favicon from '#lib/assets/favicon.svg';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<meta name="theme-color" content="#ebe4d6" media="(prefers-color-scheme: light)" />
	<meta name="theme-color" content="#121110" media="(prefers-color-scheme: dark)" />
</svelte:head>

{@render children()}

<style>
	:global(:root) {
		color-scheme: light dark;
		--bg: #ebe4d6;
		--spot: #fffdf8;
		--text: #2a2723;
		--muted: #7a736a;
		--line: #d9d2c5;
		--button: #fbf9f4;
		--paper-front: #d9473f;
		--paper-back: #f2c14e;
		--dot: #2f2b26;
		--shadow: rgb(60 45 25 / 0.26);
		--cast: rgb(60 45 25);
		--cast-step: 0.032;
		--cross: #d9543f;
		--accent: #2f7d6d;
		--carry: #c8862a;
		--on-accent: #ffffff;
	}
	@media (prefers-color-scheme: dark) {
		:global(:root) {
			--bg: #121110;
			--spot: #3a3733;
			--text: #ece6db;
			--muted: #9a9286;
			--line: #3a3732;
			--button: #262421;
			--paper-front: #d24a42;
			--paper-back: #e8b84a;
			--dot: #d8d0c2;
			--shadow: rgb(0 0 0 / 0.5);
			--cast: rgb(0 0 0);
			--cast-step: 0.081;
			--cross: #e0604a;
			--accent: #5fbfa8;
			--carry: #e0a345;
			--on-accent: #10201c;
		}
	}
	:global(html) {
		background: var(--bg);
	}
	:global(html),
	:global(body) {
		margin: 0;
		color: var(--text);
		font-family: 'Courier Prime', 'Courier New', monospace;
		-webkit-tap-highlight-color: transparent;
		overflow-x: clip;
	}
</style>
