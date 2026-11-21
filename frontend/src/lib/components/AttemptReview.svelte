<script lang="ts">
	/**
	 * Per-question answer review, shared by the post-submit screen and the
	 * attempt review page.
	 *
	 * When `options` is non-empty each option is listed with markers for the
	 * user's pick and the correct answer; otherwise only the answer texts
	 * (`selected_option_text` / `correct_option_text`) are shown.
	 */
	export interface ReviewItem {
		question_id: string;
		question_text: string;
		options: { id: string; text: string }[];
		selected_option_id: string | null;
		selected_option_text?: string | null;
		correct_option_id: string;
		correct_option_text?: string;
		is_correct: boolean;
	}

	let { items }: { items: ReviewItem[] } = $props();

	function optionText(item: ReviewItem, optionId: string | null): string {
		if (!optionId) return '—';
		const found = item.options.find((o) => o.id === optionId)?.text;
		if (found !== undefined) return found;
		if (optionId === item.correct_option_id) return item.correct_option_text ?? '—';
		return item.selected_option_text ?? '—';
	}
</script>

<ul class="review">
	{#each items as r (r.question_id)}
		<li class:correct={r.is_correct} class:wrong={!r.is_correct}>
			<p class="q">{r.question_text}</p>
			<p class="status">
				{#if r.is_correct}
					✅ Correct
				{:else if r.selected_option_id === null}
					❌ Not answered
				{:else}
					❌ Incorrect
				{/if}
			</p>
			{#if r.options.length > 0}
				<ul class="options">
					{#each r.options as option (option.id)}
						<li
							class:correct-option={option.id === r.correct_option_id}
							class:selected-option={option.id === r.selected_option_id &&
								option.id !== r.correct_option_id}
						>
							<span class="marker">
								{#if option.id === r.correct_option_id}
									✓
								{:else if option.id === r.selected_option_id}
									✗
								{:else}
									·
								{/if}
							</span>
							<span class:strike={option.id === r.selected_option_id &&
								option.id !== r.correct_option_id}>
								{option.text}
							</span>
							{#if option.id === r.correct_option_id}
								<span class="tag correct">correct answer</span>
							{:else if option.id === r.selected_option_id}
								<span class="tag wrong">your answer</span>
							{/if}
						</li>
					{/each}
				</ul>
			{:else}
				<p class="answer-line">
					{#if r.selected_option_id !== null}
						Your answer: {optionText(r, r.selected_option_id)} ·
					{/if}
					Correct: {optionText(r, r.correct_option_id)}
				</p>
			{/if}
		</li>
	{/each}
</ul>

<style>
	.review {
		list-style: none;
		padding: 0;
		display: grid;
		gap: 0.75rem;
	}
	.review li {
		border: 1px solid #e2e8f0;
		border-radius: 8px;
		padding: 0.75rem 1rem;
	}
	.review li.wrong {
		border-color: #fecaca;
		background: #fef2f2;
	}
	.review li.correct {
		border-color: #bbf7d0;
		background: #f0fdf4;
	}
	.review p {
		margin: 0;
	}
	.q {
		font-weight: 600;
	}
	.review .q {
		margin-bottom: 0.5rem;
	}
	.review .status {
		margin-bottom: 0.5rem;
		font-size: 0.9rem;
	}
	.options {
		list-style: none;
		padding: 0;
		margin: 0;
		display: grid;
		gap: 0.25rem;
	}
	.options li {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.3rem 0.5rem;
		border: none;
		border-radius: 6px;
		background: #fff;
		color: #334155;
	}
	.options li.correct-option {
		background: #dcfce7;
		color: #14532d;
		font-weight: 600;
	}
	.options li.selected-option {
		background: #fee2e2;
		color: #7f1d1d;
	}
	.marker {
		width: 1em;
		text-align: center;
		flex-shrink: 0;
	}
	.strike {
		text-decoration: line-through;
		text-decoration-thickness: 2px;
	}
	.tag {
		margin-left: auto;
		font-size: 0.75rem;
		white-space: nowrap;
	}
	.tag.correct {
		color: #15803d;
	}
	.tag.wrong {
		color: #b91c1c;
	}
	.answer-line {
		color: #475569;
	}
</style>
