<script lang="ts" generics="T extends {id: string}">
  	import type { Snippet } from 'svelte';

	import Title from '$lib/Title.svelte';

    let { items, content }: {
		items: T[];
		content: Snippet<[T]>;
	}  = $props();

</script>

<div class="flex flex-col h-full w-full px-4 pb-4 md:px-8 md:pb-8">
	<Title />
	{#if items && items.length > 0}
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-2 xl:grid-cols-4 gap-6 my-4 md:my-8">
			{#each items as item (item.id)}
                <div class="h-full">
                    {@render content(item)}
                </div>
			{/each}
		</div>
	{/if}
</div>
