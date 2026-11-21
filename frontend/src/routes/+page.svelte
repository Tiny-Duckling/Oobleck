<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api, type QuizSummary } from '#lib/api';
	import { auth } from '#lib/auth.svelte';
	import { downloadQuizPdf } from '#lib/pdf';

	let quizzes = $state<QuizSummary[]>([]);
	let error = $state('');
	let downloadError = $state('');
	let downloadingId = $state<string | null>(null);
	let loading = $state(true);

	let subjects = $derived(
		[...new Set(quizzes.flatMap((q) => q.subjects))].sort((a, b) => a.localeCompare(b))
	);
	let activeSubject = $state('All');
	let filtered = $derived(
		activeSubject === 'All' ? quizzes : quizzes.filter((q) => q.subjects.includes(activeSubject))
	);

	let totalQuestions = $derived(quizzes.reduce((sum, q) => sum + q.question_count, 0));
	let selectedSubject = $state('');
	let quizSize = $state<number>(10);
	let generating = $state(false);
	let generateError = $state('');

	async function generateQuiz() {
		if (!selectedSubject) return;
		generating = true;
		generateError = '';
		try {
			const { quiz_id } = await api.generateQuiz(selectedSubject, quizSize, auth.token!);
			goto(`/quiz/${quiz_id}`);
		} catch (e) {
			generateError = e instanceof Error ? e.message : 'failed to generate quiz';
		} finally {
			generating = false;
		}
	}

	async function downloadPdf(id: string) {
		const token = auth.token;
		if (!token) {
			window.location.href = '/login';
			return;
		}

		downloadingId = id;
		downloadError = '';
		try {
			const quiz = await api.getQuiz(id, token);
			downloadQuizPdf(quiz);
		} catch (e) {
			downloadError = e instanceof Error ? e.message : 'failed to download quiz';
		} finally {
			downloadingId = null;
		}
	}

	onMount(async () => {
		try {
			quizzes = await api.listQuizzes();
			if (subjects.length > 0) selectedSubject = subjects[0];
		} catch (e) {
			error = e instanceof Error ? e.message : 'failed to load quizzes';
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>Oobleck</title>
</svelte:head>

<section class="hero">
	<div class="hero-copy">
		<h1>
			<svg class="hero-blob" viewBox="0 0 32 32" aria-hidden="true">
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
			Oobleck
		</h1>
		<p class="tagline">Practice that sticks.</p>
		<p class="sub">
			Pick a quiz below, or generate a fresh mix of random questions from any subject.
		</p>
	</div>
	{#if !loading && !error}
		<dl class="stats">
			<div><dt>Quizzes</dt><dd>{quizzes.length}</dd></div>
			<div><dt>Questions</dt><dd>{totalQuestions.toLocaleString()}</dd></div>
			<div><dt>Subjects</dt><dd>{subjects.length}</dd></div>
		</dl>
	{/if}
</section>

{#if auth.isLoggedIn && subjects.length > 0}
	<section class="generate-panel">
		<div class="gen-field">
			<label for="gen-subject">Subject</label>
			<select id="gen-subject" bind:value={selectedSubject}>
				{#each subjects as subject (subject)}
					<option value={subject}>{subject}</option>
				{/each}
			</select>
		</div>
		<div class="gen-field">
			<label for="gen-size">Questions</label>
			<input id="gen-size" type="number" min="1" max="50" bind:value={quizSize} />
		</div>
		<button class="gen-button" type="button" onclick={generateQuiz} disabled={generating}>
			{generating ? 'Mixing…' : 'Generate practice quiz'}
		</button>
		{#if generateError}
			<p class="error">{generateError}</p>
		{/if}
	</section>
{/if}

{#if downloadError}
	<p class="error">{downloadError}</p>
{/if}

{#if loading}
	<p class="muted">Loading quizzes…</p>
{:else if error}
	<p class="error">{error}</p>
{:else if quizzes.length === 0}
	<p class="muted">No quizzes yet. Run the backend seed script to load sample quizzes.</p>
{:else}
	<div class="chips" role="tablist" aria-label="Filter by subject">
		<button
			class="chip"
			class:active={activeSubject === 'All'}
			type="button"
			onclick={() => (activeSubject = 'All')}
		>
			All <span class="chip-count">{quizzes.length}</span>
		</button>
		{#each subjects as subject (subject)}
			<button
				class="chip"
				class:active={activeSubject === subject}
				type="button"
				onclick={() => (activeSubject = subject)}
			>
				{subject}
				<span class="chip-count">
					{quizzes.filter((q) => q.subjects.includes(subject)).length}
				</span>
			</button>
		{/each}
	</div>

	<ul class="quiz-list">
		{#each filtered as quiz (quiz.id)}
			<li class="card">
				<a href="/quiz/{quiz.id}" class="quiz-link">
					<h2>{quiz.title}</h2>
					<p>{quiz.description}</p>
				</a>
				<div class="card-footer">
					<div class="meta">
						<span class="count">{quiz.question_count} questions</span>
						{#each quiz.subjects as subject (subject)}
							<span class="subject-badge">{subject}</span>
						{/each}
					</div>
					{#if auth.isLoggedIn}
						<button
							type="button"
							onclick={() => downloadPdf(quiz.id)}
							disabled={downloadingId !== null}
						>
							{downloadingId === quiz.id ? 'Preparing PDF…' : 'Download PDF'}
						</button>
					{/if}
				</div>
			</li>
		{:else}
			<li class="muted">No quizzes in this subject yet.</li>
		{/each}
	</ul>
{/if}

<style>
	.hero {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 2rem;
		flex-wrap: wrap;
		background:
			radial-gradient(48rem 14rem at 85% -20%, rgb(5 150 105 / 0.16), transparent 60%),
			radial-gradient(30rem 12rem at 0% 120%, rgb(5 150 105 / 0.1), transparent 60%);
		border: 1px solid var(--line);
		border-radius: 18px;
		padding: 2rem;
		margin-bottom: 1.25rem;
	}
	.hero-copy h1 {
		display: flex;
		align-items: center;
		gap: 0.55rem;
		margin: 0;
		font-size: 2.3rem;
		font-weight: 800;
		letter-spacing: -0.03em;
	}
	.hero-blob {
		width: 40px;
		height: 40px;
	}
	.tagline {
		margin: 0.25rem 0 0.35rem;
		font-size: 1.15rem;
		font-weight: 600;
		color: var(--brand-dark);
	}
	.sub {
		margin: 0;
		color: var(--muted);
		max-width: 30rem;
	}
	.stats {
		display: flex;
		gap: 0.75rem;
		margin: 0;
	}
	.stats div {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 12px;
		padding: 0.65rem 1rem;
		text-align: center;
		min-width: 5.2rem;
	}
	.stats dt {
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--muted);
	}
	.stats dd {
		margin: 0;
		font-size: 1.35rem;
		font-weight: 800;
		color: var(--brand-dark);
	}

	.generate-panel {
		display: flex;
		align-items: flex-end;
		gap: 0.75rem;
		flex-wrap: wrap;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 14px;
		padding: 1rem 1.25rem;
		margin-bottom: 1.25rem;
	}
	.gen-field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.85rem;
		color: var(--muted);
	}
	.generate-panel select,
	.generate-panel input {
		padding: 0.45rem 0.55rem;
		border: 1px solid #cbd5e1;
		border-radius: 8px;
		background: var(--surface);
		color: var(--ink);
	}
	.generate-panel input {
		width: 5.5rem;
	}
	.gen-button {
		border: none;
		border-radius: 8px;
		background: var(--brand);
		color: #fff;
		padding: 0.55rem 1rem;
		font-weight: 600;
		cursor: pointer;
	}
	.gen-button:hover:not(:disabled) {
		background: var(--brand-dark);
	}
	.gen-button:disabled {
		opacity: 0.6;
		cursor: wait;
	}

	.chips {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		margin-bottom: 1rem;
	}
	.chip {
		border: 1px solid var(--line);
		border-radius: 999px;
		background: var(--surface);
		color: #334155;
		padding: 0.35rem 0.8rem;
		font-size: 0.85rem;
		cursor: pointer;
		transition: border-color 0.15s, background 0.15s;
	}
	.chip:hover {
		border-color: var(--brand);
	}
	.chip.active {
		background: var(--brand);
		border-color: var(--brand);
		color: #fff;
	}
	.chip-count {
		opacity: 0.65;
		font-size: 0.78rem;
		margin-left: 0.15rem;
	}

	.quiz-list {
		list-style: none;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 1rem;
	}
	.card {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--line);
		border-radius: 14px;
		padding: 1.25rem;
		background: var(--surface);
		color: inherit;
		transition: border-color 0.15s, box-shadow 0.15s, transform 0.15s;
	}
	.card:hover {
		border-color: var(--brand);
		box-shadow: 0 4px 14px rgb(5 150 105 / 0.12);
		transform: translateY(-2px);
	}
	.quiz-link {
		display: block;
		color: inherit;
		text-decoration: none;
		flex: 1;
	}
	.card h2 {
		margin: 0 0 0.5rem;
		font-size: 1.1rem;
		letter-spacing: -0.01em;
	}
	.card p {
		margin: 0 0 1rem;
		color: var(--muted);
		font-size: 0.92rem;
	}
	.card-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		flex-wrap: wrap;
	}
	.meta {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		flex-wrap: wrap;
	}
	.count {
		font-size: 0.8rem;
		color: #475569;
		background: #f1f5f9;
		padding: 0.2rem 0.6rem;
		border-radius: 999px;
	}
	.subject-badge {
		font-size: 0.75rem;
		color: var(--brand-dark);
		background: var(--brand-soft);
		padding: 0.2rem 0.6rem;
		border-radius: 999px;
		white-space: nowrap;
	}
	.quiz-list button {
		border: 1px solid #cbd5e1;
		border-radius: 6px;
		background: var(--surface);
		color: var(--ink);
		padding: 0.4rem 0.7rem;
		font-size: 0.85rem;
		cursor: pointer;
	}
	.quiz-list button:hover:not(:disabled) {
		background: #f8fafc;
		border-color: #94a3b8;
	}
	.quiz-list button:disabled {
		opacity: 0.6;
		cursor: wait;
	}
	.muted {
		color: var(--muted);
	}
	.error {
		color: #dc2626;
	}
</style>
