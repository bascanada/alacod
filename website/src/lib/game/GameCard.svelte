<script lang="ts">
	import Card from '$lib/layouts/Card.svelte';

	export let id = 'id';
	export let title = 'Card Title';
	export let content = 'Card content goes here.';
	export let online = false;

	let footers = [{ text: 'Experimental', classes: 'badge preset-tonal-primary' }];
	if (online) {
		footers.push({ text: 'Online', classes: 'badge preset-tonal-secondary' });
	}
</script>

<Card {title} {content} {footers} href={'/play?id=' + id + '&online=false'} />
