<script lang="ts">
  export let title = 'Card Title';
  export let content = 'Card content goes here.';

  export let href = "";

  export let footers: { text: string, classes: string }[] = [];
</script>

<a
  href={href}
  class="card preset-filled-surface-100-900 border-[1px] border-surface-200-800 card-hover divide-surface-200-800 block divide-y overflow-hidden"
>
  <header>
    <h3 class="h3 m-5 text-left">{title}</h3>

  </header>
  <article class="space-y-4 p-4 h-30">
    <p class="opacity-61">
      {content}
    </p>
  </article>
  {#if footers && footers.length > 0}
  <footer class="flex items-center justify-between gap-4 p-4">
	{#each footers as item (item.text)}
    <span class={item.classes}>{item.text}</span>
	{/each}
  </footer>
  {/if}
</a>


<style>
  /* Ensure consistent heights */
  .card article {
    display: flex;
    flex-direction: column;
    min-height: 200px;
  }
  
  /* Optional: Add hover effect */
  .card:hover {
    transform: translateY(-4px);
    transition: transform 0.2s ease-in-out;
  }
</style>