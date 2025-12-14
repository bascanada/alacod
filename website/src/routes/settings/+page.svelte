<script lang="ts">
	import { onMount } from 'svelte';
	import { settingsStore } from './settingsStore.js';
	import type { Settings } from './settingsStore.js';
	import { toaster } from '$lib/toaster.js';

	let settings: Settings = { allumetteServerUrl: '' };

	const unsubscribe = settingsStore.subscribe((value) => {
		settings = { ...value };
	});

	onMount(() => {
		return () => unsubscribe();
	});

	function saveSettings() {
		if (!settings) return;
		const success = settingsStore.save(settings);

		if (success) {
			settingsStore.set(settings);
			toaster.create({
				type: 'info',
				title: 'Settings saved',
				duration: 5000
			});
		} else {
			toaster.create({
				type: 'error',
				title: 'Error saving',
				duration: 5000
			});
		}
	}

	function resetSettings() {
		settingsStore.reset();
		toaster.create({
			type: 'info',
			title: 'Settings reset',
			duration: 5000
		});
	}
</script>

<div class="card p-4 w-full max-w-md mx-auto">
	<header class="card-header">
		<h2 class="h2 mb-4">Application Settings</h2>
	</header>

	<section class="p-4">
		<form on:submit|preventDefault={saveSettings}>
			<div class="form-group">
				<label class="label" for="allumetteServerUrl">
					<span>Allumette Server URL</span>
				</label>
				<input class="input" type="text" id="allumetteServerUrl" bind:value={settings.allumetteServerUrl} placeholder="Enter server URL" />
				<p class="text-sm text-slate-500">The URL for the Allumette/Matchbox server</p>
			</div>

			<div class="grid grid-cols-2 gap-4 mt-8">
				<button type="button" class="btn preset-outlined-tertiary-500 w-full" on:click={resetSettings}> Reset to Defaults </button>
				<button type="submit" class="btn preset-tonal-primary w-full"> Save Settings </button>
			</div>
		</form>
	</section>
</div>

<style>
	.range {
		accent-color: var(--color-secondary-contrast-500);
		width: 100%;
	}
</style>
