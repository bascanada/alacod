<script>
	import { AllumetteLobbies } from '@bascanada/allumette-web';
	import { goto } from '$app/navigation';
	import applications from '$lib/game/applications.json';

	const onlineGames = applications.filter((app) => app.online);

	// @ts-ignore
	function handleJoinLobby({ lobbyId, token, players, isPrivate, gameId }) {
		console.log('Starting game with:', { lobbyId, token, players, isPrivate, gameId });
		
		// Validate that gameId is provided
		if (!gameId) {
			console.error('No gameId provided by AllumetteLobbies component');
			alert('Error: No game selected. Please select a game before joining a lobby.');
			return;
		}

		// Verify gameId exists in available games
		const gameExists = onlineGames.some(game => game.id === gameId);
		if (!gameExists) {
			console.error(`Invalid gameId: ${gameId}`);
			alert(`Error: Game '${gameId}' not found.`);
			return;
		}

		const params = new URLSearchParams();
		params.set('online', 'true');
		params.set('id', gameId);
		params.set('lobby', token);
		params.set('size', players.length.toString());

		goto(`/play?${params.toString()}`);
	}
</script>

<div class="h-full w-full overflow-hidden relative">
	<div class="h-full w-full overflow-y-auto p-4">
		<AllumetteLobbies onJoinLobby={handleJoinLobby} availableGames={onlineGames} />
	</div>
</div>
