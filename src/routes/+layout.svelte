<script lang="ts">
	import '../app.css';
	import { initializeLanguage, locale } from '#lib/i18n';
	import { setLocale } from '#lib/api';
	import { logError } from '#lib/logger';
	import Sidebar from '../components/Sidebar.svelte';
	import { status } from '#lib/stores/status';
	import { elevation } from '#lib/stores/elevation';
	import { onMount, onDestroy } from 'svelte';
	import type { Snippet } from 'svelte';

	let { children }: { children: Snippet } = $props();

	onMount(() => {
		const stopLanguage = initializeLanguage();
		const unsubscribeLocale = locale.subscribe((value) => {
			document.documentElement.lang = value;
			setLocale(value).catch((error) => logError('language.sync', error));
		});
		status.startListening();
		elevation.load();
		return () => { stopLanguage(); unsubscribeLocale(); };
	});

	onDestroy(() => {
		status.stopListening();
	});
</script>

<div id="app">
	<Sidebar />
	<main class="main-content">
		{@render children()}
	</main>
</div>

<style>
	.main-content {
		flex: 1;
		padding: 24px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
	}
</style>
