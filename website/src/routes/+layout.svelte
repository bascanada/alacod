<script lang="ts">
	// Responsive Layout
	import { Navigation, AppBar } from '@skeletonlabs/skeleton-svelte';
	import { House, Settings, ToiletIcon, Gamepad2, Menu } from '@lucide/svelte/icons';
	import { Toaster } from '@skeletonlabs/skeleton-svelte';
	import { toaster } from '$lib/toaster';
	import { setApiUrl } from '@bascanada/allumette-web';
	import { settingsStore } from './settings/settingsStore';
	import { onDestroy } from 'svelte';
	import { fly, fade } from 'svelte/transition';

	import '../app.css';
	import CurrentVersion from '$lib/game/CurrentVersion.svelte';
	import UserMenu from '$lib/components/UserMenu.svelte';

	let { children } = $props();

	let open = $state(false);

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

{#if open}
	<!-- Backdrop -->
	<div 
		class="fixed inset-0 bg-black/50 z-[999] md:hidden" 
		transition:fade={{ duration: 200 }}
		onclick={() => open = false}
		aria-hidden="true"
	></div>
	
	<!-- Drawer Content -->
	<div 
		class="fixed top-0 left-0 bottom-0 w-auto bg-surface-100-800-token z-[1000] md:hidden"
		transition:fly={{ x: -200, duration: 200 }}
	>
		<div class="h-full overflow-y-auto">
			<Navigation.Rail>
				{#snippet header()}
					<Navigation.Tile label="" href="/" onclick={() => open = false}><img class="logo" src="/icons/android-chrome-192x192.png" alt="logo" /></Navigation.Tile>
					<CurrentVersion></CurrentVersion>
				{/snippet}
				{#snippet tiles()}
					<Navigation.Tile label="Home" href="/" onclick={() => open = false}><House /></Navigation.Tile>
					<Navigation.Tile label="Online" href="/online" onclick={() => open = false}><Gamepad2 /></Navigation.Tile>
					<Navigation.Tile label="Blog" href="/blog" onclick={() => open = false}><House /></Navigation.Tile>
					<Navigation.Tile label="Games" href="/games" onclick={() => open = false}><ToiletIcon /></Navigation.Tile>
				{/snippet}
				{#snippet footer()}
					<div class="flex flex-col gap-2 items-center p-2">
						<Navigation.Tile label="Settings" href="/settings" onclick={() => open = false}><Settings /></Navigation.Tile>
					</div>
				{/snippet}
			</Navigation.Rail>
		</div>
	</div>
{/if}

<div class="h-screen w-full flex flex-col md:flex-row overflow-hidden">
	<!-- Mobile Header -->
	<div class="md:hidden w-full">
		<AppBar>
			{#snippet lead()}
				<button class="btn p-0" onclick={() => (open = !open)}>
					<img src="/icons/android-chrome-192x192.png" alt="menu" class="w-12 h-12 rounded" />
				</button>
			{/snippet}
			{#snippet headline()}
				<h1 class="font-metal-mania text-3xl text-center">Alacod</h1>
			{/snippet}
			{#snippet trail()}
				<UserMenu showLabel={false} />
			{/snippet}
		</AppBar>
	</div>

	<!-- Desktop Navigation -->
	<div class="hidden md:flex h-full z-50 bg-surface-100-800-token border-t border-surface-300-600-token">
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

	<!-- Main Content -->
	<div class="flex-1 overflow-auto w-full">
		{@render children()}
	</div>
</div>

<Toaster {toaster}></Toaster>

<style>
	.logo {
		border-radius: 10px;
	}
</style>
