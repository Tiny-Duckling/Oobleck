<script lang="ts">
	import '../app.css';
	import { auth } from '#lib/auth.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<title>Oobleck</title>
	<meta name="theme-color" content="#059669" />
</svelte:head>

<div class="app">
	<header>
		<nav>
			<a href="/" class="brand">
				<svg class="blob" viewBox="0 0 32 32" aria-hidden="true">
					<path
						d="M16 3c6.5-.6 12.6 4.6 13 11 .4 6.7-5.5 14.6-13 15S3.4 23.7 3 14C2.7 7.4 9.5 3.6 16 3z"
						fill="#059669"
					/>
					<path
						d="M11 13.2c.8-.9 2.2-.9 3 0M18 13.2c.8-.9 2.2-.9 3 0M11.5 18.5c2.4 1.6 6.6 1.6 9 0"
						stroke="#ecfdf5"
						stroke-width="1.6"
						stroke-linecap="round"
						fill="none"
					/>
				</svg>
				Oobleck</a
			>
			<div class="links">
				{#if auth.isLoggedIn}
					<a href="/history">My History</a>
					<span class="user">Hi, {auth.username}</span>
					<button onclick={() => auth.logout()}>Log out</button>
				{:else}
					<a href="/login">Log in</a>
					<a href="/register">Register</a>
				{/if}
			</div>
		</nav>
	</header>
	<main>
		{@render children()}
	</main>
</div>

<style>
	.app {
		min-height: 100vh;
	}
	header {
		border-bottom: 1px solid var(--line);
		background: rgb(255 255 255 / 0.85);
		backdrop-filter: blur(8px);
		position: sticky;
		top: 0;
		z-index: 10;
	}
	nav {
		max-width: 960px;
		margin: 0 auto;
		padding: 0.75rem 1rem;
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.brand {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		font-weight: 800;
		font-size: 1.2rem;
		letter-spacing: -0.02em;
		color: var(--ink);
		text-decoration: none;
	}
	.blob {
		width: 26px;
		height: 26px;
		display: block;
	}
	.links {
		display: flex;
		align-items: center;
		gap: 1rem;
	}
	.links a {
		color: #334155;
		text-decoration: none;
	}
	.links a:hover {
		color: var(--brand-dark);
	}
	.user {
		color: #64748b;
		font-size: 0.9rem;
	}
	button {
		border: 1px solid #cbd5e1;
		background: #f8fafc;
		border-radius: 6px;
		padding: 0.35rem 0.75rem;
		cursor: pointer;
	}
	button:hover {
		background: #e2e8f0;
	}
	main {
		max-width: 960px;
		margin: 0 auto;
		padding: 2rem 1rem;
	}
</style>
