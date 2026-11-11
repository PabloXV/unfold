<script lang="ts">
	import '@fontsource/courier-prime/400.css';
	import '@fontsource/courier-prime/700.css';
	import favicon from '#lib/assets/favicon.svg';
	import { DARK, LIGHT } from '#lib/theme.ts';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<link rel="apple-touch-icon" href="/apple-touch-icon.png" />
	<meta name="color-scheme" content="light dark" />
	<meta name="theme-color" content={LIGHT.bg} media="(prefers-color-scheme: light)" />
	<meta name="theme-color" content={DARK.bg} media="(prefers-color-scheme: dark)" />
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
		--shadow: rgb(60 45 25 / 0.26);
		--cast: rgb(60 45 25);
		--cast-step: 0.032;
		--accent: #2f7d6d;
		--carry: #c8862a;
		--on-accent: #ffffff;
	}
	@media (prefers-color-scheme: dark) {
		:global(:root) {
			--bg: #3d3833;
			--spot: #6a645c;
			--text: #f1ece3;
			--muted: #bdb5aa;
			--line: #5a544d;
			--button: #4a4540;
			--shadow: rgb(0 0 0 / 0.4);
			--cast: rgb(0 0 0);
			--cast-step: 0.05;
			--accent: #5fbfa8;
			--carry: #f0b257;
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
