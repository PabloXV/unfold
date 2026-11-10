<script lang="ts">
	import favicon from '#lib/assets/favicon.svg';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<meta name="theme-color" content="#f4f0e8" media="(prefers-color-scheme: light)" />
	<meta name="theme-color" content="#1b1a18" media="(prefers-color-scheme: dark)" />
</svelte:head>

{@render children()}

<style>
	:global(:root) {
		color-scheme: light dark;
		--bg: #f4f0e8;
		--text: #2a2723;
		--muted: #7a736a;
		--line: #d9d2c5;
		--button: #fbf9f4;
		--ring: #d4ccbe;
		--paper: #fffdf8;
		--paper-deep: #ebe4d6;
		--paper-solved: #f1f7ec;
		--ink: #2f2b26;
		--dot: #2f2b26;
		--fold: #c2b8a9;
		--shadow: rgb(60 45 25 / 0.26);
		--cross: #d9543f;
		--accent: #2f7d6d;
		--carry: #c8862a;
		--on-accent: #ffffff;
	}
	@media (prefers-color-scheme: dark) {
		:global(:root) {
			--bg: #1b1a18;
			--text: #ece6db;
			--muted: #9a9286;
			--line: #3a3732;
			--button: #262421;
			--ring: #3d3a34;
			--paper: #ece6da;
			--paper-deep: #cbc3b4;
			--paper-solved: #dfeedd;
			--ink: #2a2622;
			--dot: #d8d0c2;
			--fold: #8e8576;
			--shadow: rgb(0 0 0 / 0.5);
			--cross: #e0604a;
			--accent: #5fbfa8;
			--carry: #e0a345;
			--on-accent: #10201c;
		}
	}
	:global(html),
	:global(body) {
		margin: 0;
		background: var(--bg);
		color: var(--text);
		font-family: ui-rounded, 'SF Pro Rounded', system-ui, -apple-system, 'Segoe UI', sans-serif;
		-webkit-tap-highlight-color: transparent;
	}
</style>
