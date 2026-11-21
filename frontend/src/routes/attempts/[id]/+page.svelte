<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { api, type AttemptDetail } from '#lib/api';
	import type { ReviewItem as ReviewItemShape } from '#lib/components/AttemptReview.svelte';
	import { auth } from '#lib/auth.svelte';
	import AttemptReview from '#lib/components/AttemptReview.svelte';

	let attempt = $state<AttemptDetail | null>(null);
	let optionsByQuestion = $state<Record<string, { id: string; text: string }[]>>({});
	let error = $state('');
	let loading = $state(true);

	const attemptId = $derived(page.params.id);

	const items = $derived<ReviewItemShape[]>(
		attempt
			? attempt.results.map((r) => ({
					question_id: r.question_id,
					question_text: r.question_text,
					// Full option lists only when the quiz itself is still available;
					// otherwise fall back to the server-provided answer texts.
					options: optionsByQuestion[r.question_id] ?? [],
					selected_option_id: r.selected_option_id,
					selected_option_text: r.selected_option_text,
					correct_option_id: r.correct_option_id,
					correct_option_text: r.correct_option_text,
					is_correct: r.is_correct
				}))
			: []
	);

	onMount(async () => {
		const token = auth.token;
		if (!token) {
			window.location.href = '/login';
			return;
		}

		try {
			attempt = await api.getAttempt(attemptId ?? '', token);
			// Best-effort: load the quiz so the review can show full option lists.
			// Fails for deleted (e.g. re-seeded) quizzes — the review still works
			// via the stored answer texts.
			try {
				const quiz = await api.getQuiz(attempt.quiz_id, token);
				optionsByQuestion = Object.fromEntries(
					quiz.questions.map((q) => [q.id, q.options.map((o) => ({ id: o.id, text: o.text }))])
				);
			} catch {
				optionsByQuestion = {};
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'failed to load attempt';
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>Attempt review — Oobleck</title>
</svelte:head>

{#if loading}
	<p>Loading attempt…</p>
{:else if error}
	<p class="error">{error}</p>
	<p><a href="/history">Back to my history</a></p>
{:else if attempt}
	<h1>{attempt.quiz_title}</h1>
	<p class="meta">
		Score: <strong>{attempt.score}</strong> / {attempt.total_questions}
		({Math.round((attempt.score / attempt.total_questions) * 100)}%) ·
		{new Date(attempt.submitted_at).toLocaleString()}
	</p>
	<AttemptReview {items} />
	<p><a href="/history">Back to my history</a> · <a href="/">Back to quizzes</a></p>
{/if}

<style>
	h1 {
		margin: 0 0 0.25rem;
	}
	.meta {
		color: #64748b;
		margin: 0 0 1.5rem;
	}
	.error {
		color: #dc2626;
	}
</style>
