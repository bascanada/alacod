<script>
	import { AllumetteLobbies } from '@bascanada/allumette-web';
	import { goto } from '$app/navigation';
	import applications from '$lib/game/applications.json';

	const onlineGames = applications.filter((app) => app.online);

	// @ts-ignore
	function handleJoinLobby({ lobbyId, token, players, isPrivate, gameId }) {
		console.log('Starting game with:', { lobbyId, token, players, isPrivate, gameId });
		// Redirect to play page with necessary params
		// We assume 'chess' is the default game or gameId is passed.
		// If gameId is not provided by the component, we might need a default or selection.
		// For now, let's assume 'chess' or take it from gameId if available.
		const id = gameId || 'chess';

		const params = new URLSearchParams();
		params.set('online', 'true');
		params.set('id', id);
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
