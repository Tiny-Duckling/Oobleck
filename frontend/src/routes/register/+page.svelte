<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '#lib/auth.svelte';

	let username = $state('');
	let password = $state('');
	let error = $state('');
	let busy = $state(false);

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		busy = true;
		try {
			await auth.register(username, password);
			goto('/');
		} catch (err) {
			error = err instanceof Error ? err.message : 'registration failed';
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head>
	<title>Register — Oobleck</title>
</svelte:head>

<h1>Register</h1>

<form onsubmit={submit} class="form">
	<label>
		Username
		<input bind:value={username} autocomplete="username" required minlength={3} maxlength={32} />
	</label>
	<label>
		Password
		<input type="password" bind:value={password} autocomplete="new-password" required minlength={6} />
	</label>
	{#if error}
		<p class="error">{error}</p>
	{/if}
	<button type="submit" disabled={busy}>
		{busy ? 'Creating account…' : 'Create account'}
	</button>
	<p class="hint">
		Already have an account? <a href="/login">Log in</a>
	</p>
</form>

<style>
	.form {
		display: grid;
		gap: 1rem;
		max-width: 360px;
	}
	label {
		display: grid;
		gap: 0.35rem;
		font-size: 0.9rem;
		color: #334155;
	}
	input {
		padding: 0.5rem 0.75rem;
		border: 1px solid #cbd5e1;
		border-radius: 6px;
		font-size: 1rem;
	}
	button {
		padding: 0.6rem;
		border: none;
		border-radius: 6px;
		background: #0f172a;
		color: #fff;
		font-size: 1rem;
		cursor: pointer;
	}
	button:disabled {
		opacity: 0.6;
	}
	.error {
		color: #dc2626;
		margin: 0;
	}
	.hint {
		font-size: 0.9rem;
		color: #64748b;
	}
</style>
