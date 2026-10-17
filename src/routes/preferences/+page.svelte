<script lang="ts">
	import { t } from '#lib/i18n';
	import PreferencesPanel from '../../components/PreferencesPanel.svelte';
</script>

<svelte:head>
	<title>{$t('Preferences - anylinuxfs')}</title>
</svelte:head>

<div class="page">
	<PreferencesPanel />
</div>

<style>
</style>
