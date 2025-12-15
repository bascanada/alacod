<script lang="ts">
	import { Navigation } from '@skeletonlabs/skeleton-svelte';
	import { House, Settings, ToiletIcon, Gamepad2 } from '@lucide/svelte/icons';
	import { Toaster } from '@skeletonlabs/skeleton-svelte';
	import { toaster } from '$lib/toaster';
	import { setApiUrl } from '@bascanada/allumette-web';
	import { settingsStore } from './settings/settingsStore';
	import { onDestroy } from 'svelte';

	import '../app.css';
	import CurrentVersion from '$lib/game/CurrentVersion.svelte';
	import UserMenu from '$lib/components/UserMenu.svelte';

	let { children } = $props();

	// Subscribe to settings to update API URL globally
	const unsubscribe = settingsStore.subscribe((settings) => {
		if (settings.allumetteServerUrl) {
			setApiUrl(settings.allumetteServerUrl);
		}
	});

	onDestroy(() => {
		unsubscribe();
	});
</script>

<div class="h-full w-full flex flex-row">
	<div class="h-full z-50 bg-surface-100-800-token border-t border-surface-300-600-token">
		<Navigation.Rail>
			{#snippet header()}
				<Navigation.Tile label="" href="/"><img class="logo" src="/icons/android-chrome-192x192.png" alt="logo" /></Navigation.Tile>
				<CurrentVersion></CurrentVersion>
			{/snippet}
			{#snippet tiles()}
				<Navigation.Tile label="Home" href="/"><House /></Navigation.Tile>
				<Navigation.Tile label="Online" href="/online"><Gamepad2 /></Navigation.Tile>
				<Navigation.Tile label="Blog" href="/blog"><House /></Navigation.Tile>
				<Navigation.Tile label="Games" href="/games"><ToiletIcon /></Navigation.Tile>
			{/snippet}

			{#snippet footer()}
				<div class="flex flex-col gap-2 items-center p-2">
					<UserMenu />
					<Navigation.Tile label="Settings" href="/settings"><Settings /></Navigation.Tile>
				</div>
			{/snippet}
		</Navigation.Rail>
	</div>

	{@render children()}
</div>

<Toaster {toaster}></Toaster>

<style>
	.logo {
		border-radius: 10px;
	}
</style>
