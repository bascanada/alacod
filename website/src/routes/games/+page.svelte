<script lang="ts">
	import CardGrid from '$lib/layouts/CardGrid.svelte';
	import applications from '$lib/game/applications.json';
	import GameCard from '$lib/game/GameCard.svelte';
</script>

<CardGrid items={applications}>
	{#snippet content(item)}
		<GameCard id={item.id} title={item.name} online={item.online} content={item.content} />
	{/snippet}
</CardGrid>
