<script lang="ts">
	import { Modal, Navigation } from '@skeletonlabs/skeleton-svelte';
	import { User } from '@lucide/svelte/icons';
	import { AllumetteAuth, AllumetteFriendsList, isLoggedIn } from '@bascanada/allumette-web';

	let openState = $state(false);
</script>

<Navigation.Tile label="Account" onclick={() => (openState = true)}>
	<User />
</Navigation.Tile>

<Modal bind:open={openState} contentBase="card p-4 space-y-4 shadow-xl w-full max-w-sm bg-surface-100 dark:bg-surface-900">
	{#snippet content()}
		<header class="flex justify-between items-center">
			<h2 class="h3">Account</h2>
			<button class="btn-icon btn-icon-sm preset-tonal-error" onclick={() => (openState = false)}>✕</button>
		</header>
		<div class="max-h-[80vh] overflow-y-auto">
			<AllumetteAuth />

			<div class="hr border-t border-surface-200-800-token my-4"></div>

			{#if $isLoggedIn}
				<h3 class="h4 mb-2">Friends</h3>
				<AllumetteFriendsList />
			{:else}
				<div class="alert variant-soft-secondary">
					<p>Log in to manage friends.</p>
				</div>
			{/if}
		</div>
	{/snippet}
</Modal>
