<script lang="ts">
	import { t } from '#lib/i18n';
	import LogViewer from '../../components/LogViewer.svelte';
</script>

<svelte:head>
	<title>{$t('Logs - anylinuxfs')}</title>
</svelte:head>

<div class="page">
	<LogViewer />
</div>

<style>
</style>
