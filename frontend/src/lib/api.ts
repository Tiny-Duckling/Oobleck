// Relative by default: in dev, vite proxies /api to localhost:4000; in
// production, nginx serves the app and proxies /api to the backend container.
// Override with VITE_API_BASE (e.g. http://localhost:4000) to target a
// different origin.
export const API_BASE = import.meta.env.VITE_API_BASE ?? '';

export interface QuizSummary {
	id: string;
	title: string;
	description: string;
	question_count: number;
	/** Distinct subjects of the quiz's questions; empty when untagged. */
	subjects: string[];
}

export interface QuizOption {
	id: string;
	text: string;
}

export interface QuizQuestion {
	id: string;
	text: string;
	subject: string | null;
	options: QuizOption[];
}

export interface QuizDetail {
	id: string;
	title: string;
	description: string;
	questions: QuizQuestion[];
}

export interface AnswerResult {
	question_id: string;
	/** null when the question was not answered */
	selected_option_id: string | null;
	correct_option_id: string;
	is_correct: boolean;
}

export interface SubmitResponse {
	attempt_id: string;
	score: number;
	total_questions: number;
	results: AnswerResult[];
}

export interface Attempt {
	id: string;
	quiz_id: string;
	quiz_title: string;
	/** null while the attempt has not been submitted */
	score: number | null;
	total_questions: number;
	/** null while the attempt has not been submitted */
	submitted_at: string | null;
	/** when the quiz was opened */
	started_at: string;
}

/** One reviewed answer of a past attempt (option texts included server-side). */
export interface AttemptAnswer {
	question_id: string;
	question_text: string;
	position: number;
	selected_option_id: string | null;
	selected_option_text: string | null;
	correct_option_id: string;
	correct_option_text: string;
	is_correct: boolean;
}

export interface AttemptDetail extends Attempt {
	results: AttemptAnswer[];
}

export class ApiError extends Error {
	constructor(
		public status: number,
		message: string
	) {
		super(message);
	}
}

async function request<T>(
	path: string,
	options: { method?: string; body?: unknown; token?: string | null } = {}
): Promise<T> {
	const headers: Record<string, string> = {};
	if (options.body !== undefined) headers['Content-Type'] = 'application/json';
	if (options.token) headers['Authorization'] = `Bearer ${options.token}`;

	const res = await fetch(`${API_BASE}${path}`, {
		method: options.method ?? 'GET',
		headers,
		body: options.body !== undefined ? JSON.stringify(options.body) : undefined
	});

	const data = await res.json().catch(() => ({}));
	if (!res.ok) {
		throw new ApiError(res.status, data.error ?? `request failed (${res.status})`);
	}
	return data as T;
}

export const api = {
	register: (username: string, password: string) =>
		request<{ token: string; username: string }>('/api/auth/register', {
			method: 'POST',
			body: { username, password }
		}),

	login: (username: string, password: string) =>
		request<{ token: string; username: string }>('/api/auth/login', {
			method: 'POST',
			body: { username, password }
		}),

	listQuizzes: () => request<QuizSummary[]>('/api/quizzes'),

	getQuiz: (id: string, token: string) =>
		request<QuizDetail>(`/api/quizzes/${id}`, { token }),

	submitQuiz: (
		quizId: string,
		answers: { question_id: string; selected_option_id: string }[],
		token: string,
		attemptId?: string | null
	) =>
		request<SubmitResponse>(`/api/quizzes/${quizId}/submit`, {
			method: 'POST',
			body: attemptId ? { answers, attempt_id: attemptId } : { answers },
			token
		}),

	startQuizAttempt: (quizId: string, token: string) =>
		request<{ attempt_id: string }>(`/api/quizzes/${quizId}/attempt`, {
			method: 'POST',
			token
		}),

	generateQuiz: (subject: string, count: number, token: string) =>
		request<{ quiz_id: string }>('/api/quizzes/generate', {
			method: 'POST',
			body: { subject, count },
			token
		}),

	myHistory: (token: string) => request<Attempt[]>('/api/me/history', { token }),

	getAttempt: (id: string, token: string) => request<AttemptDetail>(`/api/attempts/${id}`, { token })
};
