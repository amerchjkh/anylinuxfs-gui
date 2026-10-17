<script lang="ts">
	import { t } from '#lib/i18n';
	import ConfigPanel from '../../components/ConfigPanel.svelte';
</script>

<svelte:head>
	<title>{$t('Settings - anylinuxfs')}</title>
</svelte:head>

<div class="page">
	<ConfigPanel />
</div>

<style>
</style>
