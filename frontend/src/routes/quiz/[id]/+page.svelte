<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { api, type QuizDetail, type SubmitResponse } from '#lib/api';
	import { auth } from '#lib/auth.svelte';
	import { downloadQuizPdf } from '#lib/pdf';
	import AttemptReview, { type ReviewItem } from '#lib/components/AttemptReview.svelte';

	let quiz = $state<QuizDetail | null>(null);
	let error = $state('');
	let loading = $state(true);
	let result = $state<SubmitResponse | null>(null);
	let busy = $state(false);
	// question_id -> selected option_id
	let selections = $state<Record<string, string>>({});
	// In-progress attempt created when the quiz was opened; null when starting
	// one failed (submitting still works — the backend creates an attempt then).
	let attemptId = $state<string | null>(null);

	const quizId = $derived(page.params.id);

	// Per-question review rows for the post-submit screen (full option lists).
	const reviewItems = $derived.by<ReviewItem[]>(() => {
		const currentQuiz = quiz;
		const currentResult = result;
		if (!currentQuiz || !currentResult) return [];
		return currentResult.results.map((r) => {
			const q = currentQuiz.questions.find((q) => q.id === r.question_id);
			return {
				question_id: r.question_id,
				question_text: q?.text ?? '(question no longer exists)',
				options: q?.options ?? [],
				selected_option_id: r.selected_option_id,
				correct_option_id: r.correct_option_id,
				is_correct: r.is_correct
			};
		});
	});

	$effect(() => {
		// Redirect to login if not authenticated — quiz taking requires an account.
		if (!loading && !auth.isLoggedIn) {
			window.location.href = '/login';
		}
	});

	onMount(async () => {
		const token = auth.token;
		if (!token) {
			window.location.href = '/login';
			return;
		}

		try {
			quiz = await api.getQuiz(quizId ?? '', token);
			// Record that the quiz was started so it shows up in history even
			// if it is never submitted. Non-fatal on failure.
			attemptId = await api
				.startQuizAttempt(quiz.id, token)
				.then((r) => r.attempt_id)
				.catch(() => null);
		} catch (e) {
			error = e instanceof Error ? e.message : 'failed to load quiz';
		} finally {
			loading = false;
		}
	});

	async function submit() {
		if (!quiz || !auth.token) return;
		busy = true;
		error = '';
		try {
			// Only send answers the user actually picked; the backend records
			// questions without an answer as unanswered (selected_option_id: null).
			const answers = quiz.questions
				.filter((q) => selections[q.id])
				.map((q) => ({ question_id: q.id, selected_option_id: selections[q.id] }));
			result = await api.submitQuiz(quiz.id, answers, auth.token, attemptId);
			window.scrollTo({ top: 0 });
		} catch (e) {
			error = e instanceof Error ? e.message : 'failed to submit quiz';
		} finally {
			busy = false;
		}
	}

	function downloadPdf() {
		if (!quiz) return;
		if (!auth.isLoggedIn) {
			window.location.href = '/login';
			return;
		}
		error = '';
		try {
			downloadQuizPdf(quiz);
		} catch (e) {
			error = e instanceof Error ? e.message : 'failed to download quiz';
		}
	}

</script>

<svelte:head>
	<title>{quiz ? `${quiz.title} — Oobleck` : 'Quiz — Oobleck'}</title>
</svelte:head>

{#if loading}
	<p>Loading quiz…</p>
{:else if error && !quiz}
	<p class="error">{error}</p>
{:else if quiz}
	<div class="quiz-header">
		<div>
			<h1>{quiz.title}</h1>
			{#if !result}
				<p class="desc">{quiz.description}</p>
			{/if}
		</div>
		{#if auth.isLoggedIn}
			<button type="button" class="download-button" onclick={downloadPdf}>Download PDF</button>
		{/if}
	</div>
	{#if error}
		<p class="error">{error}</p>
	{/if}
	{#if result}
		<div class="result">
			<p class="score">
				You scored <strong>{result.score}</strong> / {result.total_questions}
				({Math.round((result.score / result.total_questions) * 100)}%)
			</p>
			<AttemptReview items={reviewItems} />
			<p>
				<a href="/attempts/{result.attempt_id}">Permalink to this review</a> ·
				<a href="/history">View my history</a> · <a href="/">Back to quizzes</a>
			</p>
		</div>
	{:else}
		<form onsubmit={(e) => { e.preventDefault(); submit(); }}>
			{#each quiz.questions as question, qi (question.id)}
				<fieldset>
					<legend>Question {qi + 1}{#if question.subject} · {question.subject}{/if}</legend>
					<p class="q">{question.text}</p>
					{#each question.options as option (option.id)}
						<label class="option">
							<input
								type="radio"
								name={question.id}
								value={option.id}
								bind:group={selections[question.id]}
							/>
							{option.text}
						</label>
					{/each}
				</fieldset>
			{/each}
			<button type="submit" disabled={busy}>
				{busy ? 'Submitting…' : 'Submit answers'}
			</button>
		</form>
	{/if}
{/if}

<style>
	.quiz-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}
	h1 {
		margin: 0 0 0.5rem;
	}
	.download-button {
		flex-shrink: 0;
		white-space: nowrap;
	}
	.desc {
		color: #64748b;
		margin: 0;
	}
	@media (max-width: 560px) {
		.quiz-header {
			flex-direction: column;
		}
	}
	fieldset {
		border: 1px solid #e2e8f0;
		border-radius: 10px;
		padding: 1rem 1.25rem;
		margin-bottom: 1rem;
	}
	legend {
		font-size: 0.85rem;
		color: #64748b;
		padding: 0 0.5rem;
	}
	.q {
		font-weight: 600;
		margin: 0 0 0.75rem;
	}
	.option {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.4rem 0.5rem;
		border-radius: 6px;
		cursor: pointer;
	}
	.option:hover {
		background: #f1f5f9;
	}
	button {
		padding: 0.6rem 1.5rem;
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
	.score {
		font-size: 1.25rem;
		margin-bottom: 1rem;
	}
	.error {
		color: #dc2626;
	}
</style>
