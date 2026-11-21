import adapterStatic from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			// adapter-static serves the app as a client-side SPA; the container
			// (nginx) falls back to index.html for unknown routes and proxies
			// /api to the backend service.
			adapter: adapterStatic({
				fallback: 'index.html',
				pages: 'build',
				assets: 'build'
			})
		})
	],

	server: {
		// Dev convenience: forward /api to the local backend so the relative
		// API base (see src/lib/api.ts) works during development.
		proxy: {
			'/api': 'http://localhost:4000'
		}
	}
});
