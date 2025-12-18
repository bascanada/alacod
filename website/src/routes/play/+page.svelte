<script lang="ts">
	import { onMount } from 'svelte';
	import { settingsStore } from '../settings/settingsStore';
	import { get } from 'svelte/store';
	import GameLoader from '$lib/components/GameLoader.svelte';

	const customAppVersion: string = import.meta.env.VITE_APP_VERSION || 'DEV';

	let gameProps: {
		name: string;
		version: string;
		lobby: string | null;
		lobby_size: string | null;
		matchbox: string | null;
	} | null = null;

	onMount(() => {
		const urlParams = new URLSearchParams(window.location.search);

		const id = urlParams.get('id');
		const supportOnline = urlParams.get('online');

		const argLobbyName = urlParams.get('lobby');
		const argSize = urlParams.get('size'); // lobby_size
		const argMatchbox = urlParams.get('matchbox'); // If still passed or needed

		if (!id) return;

		let lobby = argLobbyName;
		let lobby_size = argSize;
		let matchbox: string | null = null;

		if (supportOnline == 'true') {
			// If online, we expect lobby components to have passed necessary params
			
			// Get matchbox server from settings or use provided URL as fallback
			const settings = get(settingsStore);
			matchbox = argMatchbox || settings.matchboxServer;
		} else {
			// Offline / default test lobby
			if (!argLobbyName) lobby = 'test';
		}

		gameProps = {
			name: id,
			version: customAppVersion,
			lobby: lobby || null,
			lobby_size: lobby_size || null,
			matchbox: matchbox || null
		};
	});
</script>

<div class="w-full h-full">
	{#if gameProps}
		<GameLoader 
			name={gameProps.name}
			version={gameProps.version}
			lobby={gameProps.lobby}
			lobby_size={gameProps.lobby_size}
			matchbox={gameProps.matchbox}
		/>
	{/if}
</div>
