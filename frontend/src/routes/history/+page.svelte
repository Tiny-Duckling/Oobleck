<script lang="ts">
	import { onMount } from 'svelte';
	import { api, type Attempt } from '#lib/api';
	import { auth } from '#lib/auth.svelte';

	let attempts = $state<Attempt[]>([]);
	let error = $state('');
	let loading = $state(true);

	onMount(async () => {
		if (!auth.isLoggedIn || !auth.token) {
			window.location.href = '/login';
			return;
		}
		try {
			attempts = await api.myHistory(auth.token);
		} catch (e) {
			error = e instanceof Error ? e.message : 'failed to load history';
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>My History — Oobleck</title>
</svelte:head>

<h1>My Score History</h1>

{#if loading}
	<p>Loading…</p>
{:else if error}
	<p class="error">{error}</p>
{:else if attempts.length === 0}
	<p>You haven't taken any quizzes yet. <a href="/">Take one now</a>.</p>
{:else}
	<table>
		<thead>
			<tr>
				<th>Quiz</th>
				<th>Score</th>
				<th>Date</th>
				<th></th>
			</tr>
		</thead>
		<tbody>
			{#each attempts as attempt (attempt.id)}
				<tr>
					<td><a href="/quiz/{attempt.quiz_id}">{attempt.quiz_title}</a></td>
					<td>{attempt.score} / {attempt.total_questions}</td>
					<td>{new Date(attempt.submitted_at).toLocaleString()}</td>
					<td><a href="/attempts/{attempt.id}">Review</a></td>
				</tr>
			{/each}
		</tbody>
	</table>
{/if}

<style>
	table {
		width: 100%;
		border-collapse: collapse;
	}
	th,
	td {
		text-align: left;
		padding: 0.6rem 0.75rem;
		border-bottom: 1px solid #e2e8f0;
	}
	th {
		color: #64748b;
		font-size: 0.85rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	.error {
		color: #dc2626;
	}
</style>
