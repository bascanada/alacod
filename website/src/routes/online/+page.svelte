<script lang="ts">
	import { AllumetteLobbies, friendsList } from '@bascanada/allumette-web';
	import { goto } from '$app/navigation';
	import { get } from 'svelte/store';
	import applications from '$lib/game/applications.json';

	const onlineGames = applications.filter((app) => app.online);

	interface LobbyPlayer {
		publicKey: string;
		is_you: boolean;
	}

	interface JoinLobbyEvent {
		lobbyId: string;
		token: string;
		players: LobbyPlayer[];
		isPrivate: boolean;
		gameId: string;
	}

	function handleJoinLobby({ lobbyId, token, players, isPrivate, gameId }: JoinLobbyEvent) {
		console.log('Starting game with:', { lobbyId, token, players, isPrivate, gameId });

		// Validate that gameId is provided
		if (!gameId) {
			console.error('No gameId provided by AllumetteLobbies component');
			alert('Error: No game selected. Please select a game before joining a lobby.');
			return;
		}

		// Verify gameId exists in available games
		const gameExists = onlineGames.some((game) => game.id === gameId);
		if (!gameExists) {
			console.error(`Invalid gameId: ${gameId}`);
			alert(`Error: Game '${gameId}' not found.`);
			return;
		}

		// Build player data with pubkeys and resolved names from friend list
		const friends = get(friendsList) || [];
		const playerData = players.map((p) => {
			// Try to find name in friend list
			const friend = friends.find((f) => f.publicKey === p.publicKey);
			const name = p.is_you ? 'You' : (friend?.name || `Player-${p.publicKey.substring(0, 8)}`);
			return {
				pubkey: p.publicKey,
				name: name,
				is_local: p.is_you
			};
		});

		const params = new URLSearchParams();
		params.set('online', 'true');
		params.set('id', gameId);
		params.set('lobby', token);
		params.set('size', players.length.toString());
		params.set('players', encodeURIComponent(JSON.stringify(playerData)));

		goto(`/play?${params.toString()}`);
	}
</script>

<div class="h-full w-full overflow-hidden relative">
	<div class="h-full w-full overflow-y-auto p-4">
		<AllumetteLobbies onJoinLobby={handleJoinLobby} availableGames={onlineGames} />
	</div>
</div>
