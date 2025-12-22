<script lang="ts">
	import DevProgress from '$lib/game/DevProgress.svelte';
	import SvgIcon from '$lib/SVGIcon.svelte';
	import Title from '$lib/Title.svelte';
</script>

<div class="flex flex-col w-full min-h-full">
	<header>
		<Title />
	</header>
	<main class="flex flex-col md:flex-row gap-6 p-4">
		<article class="flex-1 bg-surface-100-800-token rounded-lg p-6">
			<h2 class="font-metal-mania text-2xl font-bold mb-4">Welcome !</h2>
			<p class="mb-4">
				This is an highly customazible engine/game for p2p multiplayer 2D top down rogue like.
			</p>

			<div class="mb-6">
				<h3 class="font-metal-mania text-xl font-semibold mb-2">Engine: ALACOD</h3>
				<ul class="list-disc pl-6">
					<li class="mb-1">Random map generator based on LDTK level</li>
					<li class="mb-1">Multiple extendable algorithm for map generation</li>
					<li class="mb-1">P2P networking for serverless game</li>
					<li class="mb-1">Lobby/Matchmaking server that can be self hosted</li>
					<li class="mb-1">Customizable character, weapons, enemies and bonus</li>
				</ul>
			</div>

			<div class="mb-6">
				<h3 class="font-metal-mania text-xl font-semibold mb-2">Game: 1837</h3>
				<p class="mb-4">
					The engine is develop alongside 1837 an apocalyptic rogue like survivor game set
					during the revolt in lower canada.
				</p>	
			</div>

			<div>
				<h3 class="font-metal-mania text-xl font-semibold mb-2">Technologies</h3>
				<ul class="list-disc pl-6">
					<li class="mb-1">Engine: Rust / Bevy</li>
					<li class="mb-1">Networking: Matchbox and GGRS</li>
					<li class="mb-1">Targets: Web , Native and Mobile</li>
				</ul>
			</div>
		</article>

		<aside class="w-full md:w-1/3 lg:w-2/5 flex flex-col gap-6 items-center md:items-end">
      		<DevProgress />
		</aside>
	</main>
	<footer class="mt-auto p-8">
		<div class="rounded-container grid w-full grid-cols-1 gap-4 overflow-hidden md:grid-cols-3">
			<a
				target="_blank"
				class="card flex justify-center preset-filled-surface-100-900 p-4 py-8 text-center hover:scale-105 transition-transform"
				href="https://www.youtube.com/@BasCanada"
			>
				<SvgIcon color="var(--color-primary-500)" id="youtube" height="32px" />
			</a>
			<a
				target="_blank"
				class="card flex justify-center preset-filled-surface-100-900 p-4 py-8 text-center hover:scale-105 transition-transform"
				href="https://github.com/bascanada/alacod"
			>
				<SvgIcon color="var(--color-primary-500)" id="github" height="32px" />
			</a>
			<a
				target="_blank"
				class="card flex justify-center preset-filled-surface-100-900 p-4 py-8 text-center hover:scale-105 transition-transform"
				href="mailto:bascanada1841@gmail.com"
			>
				<SvgIcon color="var(--color-primary-500)" id="gmail" height="32px" />
			</a>
		</div>
	</footer>
</div>

