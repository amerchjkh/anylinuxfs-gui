<script lang="ts">
	import { t } from '#lib/i18n';
	import { onMount } from 'svelte';
	import { listPackages, addPackages, removePackages } from '#lib/api';
	import { isMounted } from '#lib/stores/status';
	import { wrapAsync, parseError } from '#lib/errors';

	let packages = $state<string[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let processing = $state(false);
	let newPackage = $state('');
	let removingPackage = $state<string | null>(null);

	async function loadPackages() {
		loading = true;
		error = null;
		const result = await wrapAsync(() => listPackages());
		if (result.ok) {
			packages = result.data;
		} else {
			error = result.error.message;
		}
		loading = false;
	}

	onMount(() => {
		if (!$isMounted) loadPackages();
	});

	$effect(() => {
		if (!$isMounted) loadPackages();
	});

	async function handleAdd() {
		const pkgNames = newPackage
			.split(/[\s,]+/)
			.map((p) => p.trim())
			.filter((p) => p.length > 0);

		if (pkgNames.length === 0) return;

		processing = true;
		error = null;
		try {
			await addPackages(pkgNames);
			newPackage = '';
			await loadPackages();
		} catch (e) {
			error = parseError(e).message;
		}
		processing = false;
	}

	async function handleRemove(pkg: string) {
		removingPackage = pkg;
		error = null;
		try {
			await removePackages([pkg]);
			await loadPackages();
		} catch (e) {
			error = parseError(e).message;
		}
		removingPackage = null;
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !processing) {
			handleAdd();
		}
	}
</script>

<svelte:head>
	<title>{$t('Packages - anylinuxfs')}</title>
</svelte:head>

<div class="packages-page">
	<div class="header">
		<h2>{$t('Alpine Packages')}</h2>
		<button class="btn-secondary" onclick={loadPackages} disabled={loading || $isMounted}>
			{loading && !$isMounted ? $t('Loading...') : $t('Refresh')}
		</button>
	</div>

	{#if $isMounted}
		<div class="mounted-banner">
			{$t('Unmount all filesystems to manage packages.')}
		</div>
	{:else if error}
		<div class="error-banner" role="alert">
			<span>{$t(error || '')}</span>
			<button onclick={() => (error = null)}>{$t('Dismiss')}</button>
		</div>
	{/if}

	{#if !$isMounted}
	<div class="add-package">
		<input
			type="text"
			placeholder={$t('Package name (e.g., vim, htop, curl)')}
			bind:value={newPackage}
			onkeydown={handleKeydown}
			disabled={processing}
			autocomplete="off"
			autocorrect="off"
			spellcheck="false"
		/>
		<button class="btn-primary" onclick={handleAdd} disabled={processing || !newPackage.trim()}>
			{processing ? $t('Adding...') : $t('Add Package')}
		</button>
	</div>

	<div class="packages-section">
		<h3>{$t('Installed Custom Packages')}</h3>
		{#if loading && packages.length === 0}
			<div class="loading">{$t('Loading packages...')}</div>
		{:else if packages.length === 0}
			<div class="empty">
				<p>{$t('No custom packages installed.')}</p>
				<p class="hint">{$t('Add packages to extend the VM\'s capabilities.')}</p>
			</div>
		{:else}
			<div class="packages-list">
				{#each packages as pkg}
					<div class="package-item">
						<span class="package-name">{pkg}</span>
						{#if removingPackage === pkg}
							<span class="removing">{$t('Removing...')}</span>
						{:else}
							<button
								class="btn-remove"
								onclick={() => handleRemove(pkg)}
								disabled={removingPackage !== null}
								title={$t('Remove package')}
							>
								&times;
							</button>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>
	{/if}

	<div class="info-section">
		<p>{$t('Custom packages are installed in the Alpine Linux VM and persist across mounts.')}</p>
		<p>{$t('You can add multiple packages at once by separating them with spaces or commas.')}</p>
		<p>
			{$t('Search for packages at')}
			<a href="https://pkgs.alpinelinux.org/packages" target="_blank" rel="noopener">
				pkgs.alpinelinux.org
			</a>
		</p>
	</div>
</div>

<style>
	.packages-page {
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.add-package {
		display: flex;
		gap: 10px;
		margin-bottom: 20px;
	}

	.add-package input {
		flex: 1;
		padding: 10px 14px;
		border: 1px solid var(--border-color);
		border-radius: 6px;
		font-size: 14px;
		background: var(--input-bg);
		color: var(--text-primary);
	}

	.add-package input:focus {
		border-color: var(--accent-color);
		outline: none;
	}

	.add-package input::placeholder {
		color: var(--text-tertiary);
	}

	.packages-section {
		flex: 1;
		overflow-y: auto;
	}

	.packages-section h3 {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-secondary);
		margin: 0 0 12px;
	}

	.loading,
	.empty {
		background: var(--card-bg);
		border: 1px solid var(--border-color);
		border-radius: 8px;
	}

	.empty p {
		margin: 0 0 4px;
	}

	.empty .hint {
		font-size: 12px;
		color: var(--text-tertiary);
	}

	.packages-list {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.package-item {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 10px;
		background: var(--card-bg);
		border: 1px solid var(--border-color);
		border-radius: 6px;
	}

	.package-name {
		font-size: 13px;
		font-family: monospace;
		color: var(--text-primary);
	}

	.removing {
		font-size: 11px;
		color: var(--text-tertiary);
		font-style: italic;
	}

	.btn-remove {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 18px;
		height: 18px;
		padding: 0;
		border: none;
		border-radius: 50%;
		background: transparent;
		color: var(--text-tertiary);
		font-size: 16px;
		line-height: 1;
		cursor: pointer;
		transition: background-color 0.15s, color 0.15s;
	}

	.btn-remove:hover:not(:disabled) {
		background: var(--danger-color);
		color: white;
	}

	.btn-remove:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.btn-primary,
	.btn-secondary {
		padding: 8px 16px;
		white-space: nowrap;
	}

	.mounted-banner {
		padding: 12px 16px;
		background: var(--info-bg);
		border: 1px solid var(--info-border);
		border-radius: 8px;
		color: var(--info-text);
		font-size: 13px;
		margin-bottom: 16px;
	}

	.info-section a {
		color: var(--accent-color);
		text-decoration: none;
	}

	.info-section a:hover {
		text-decoration: underline;
	}
</style>
