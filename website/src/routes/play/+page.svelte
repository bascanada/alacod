<script lang="ts">
	import { onMount } from 'svelte';
	import { settingsStore } from '../settings/settingsStore';
	import { get } from 'svelte/store';

	const customAppVersion: string = import.meta.env.VITE_APP_VERSION || 'DEV';

	let src = '';

	onMount(() => {
		const urlParams = new URLSearchParams(window.location.search);

		const id = urlParams.get('id');
		const supportOnline = urlParams.get('online');

		const argLobbyName = urlParams.get('lobby');
		const argSize = urlParams.get('size'); // lobby_size
		const argToken = urlParams.get('token');
		const argMatchbox = urlParams.get('matchbox'); // If still passed or needed

		// Construct the source URL for the iframe
		// loader.html expects: name, version, lobby, matchbox (optional), lobby_size (optional)
		// We will pass token as well just in case updated loader.js needs it later or via custom param

		let baseSrc = `/loader.html?name=${id}&version=${customAppVersion}`;

		if (argLobbyName) {
			baseSrc += `&lobby=${argLobbyName}`;
		}

		// Append telemetry settings
		const settings = get(settingsStore);
		if (settings.telemetryEnabled) {
			baseSrc += `&telemetry=true`;
			// Encode these to ensure safety in URL
			if (settings.telemetryUrl) baseSrc += `&telemetry_url=${encodeURIComponent(settings.telemetryUrl)}`;
			if (settings.telemetryAuth) baseSrc += `&telemetry_auth=${encodeURIComponent(settings.telemetryAuth)}`;
		}

		if (supportOnline == 'true') {
			// If online, we expect lobby components to have passed necessary params
			if (argSize) baseSrc += `&lobby_size=${argSize}`;
			if (argToken) baseSrc += `&token=${argToken}`; // Passing token if loader/wasm needs it
			
			// Get matchbox server from settings or use provided URL as fallback
			const matchboxUrl = argMatchbox || settings.matchboxServer;
			baseSrc += `&matchbox=${matchboxUrl}`;
		} else {
			// Offline / default test lobby
			if (!argLobbyName) baseSrc += `&lobby=test`;
		}

		src = baseSrc;
	});
</script>

<iframe id="app-frame" title="game iframe" {src} style="width: 100%; border: none; height: 100vh"></iframe>
