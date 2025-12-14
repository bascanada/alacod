import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

export default defineConfig({
	plugins: [tailwindcss(), sveltekit(), wasm(), topLevelAwait()],
	resolve: {
		alias: {
			'argon2-browser': '/src/lib/argon2-shim.js'
		}
	},
	optimizeDeps: {
		exclude: ['argon2-browser']
	},
	build: {
		rollupOptions: {
			external: ['argon2-browser']
		}
	}
});
