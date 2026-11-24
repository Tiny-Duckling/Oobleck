import { describe, it, expect, vi, beforeEach } from 'vitest';
import { api, ApiError } from './api';

describe('api client', () => {
	beforeEach(() => {
		vi.restoreAllMocks();
	});

	it('registers a user successfully', async () => {
		const mockResponse = { token: 'jwt.token.here', username: 'alice' };
		const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
			ok: true,
			status: 200,
			json: async () => mockResponse
		} as unknown as Response);

		const result = await api.register('alice', 'password123');

		expect(fetchSpy).toHaveBeenCalledWith('/api/auth/register', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ username: 'alice', password: 'password123' })
		});
		expect(result).toEqual(mockResponse);
	});

	it('logs in successfully and sends correct payload', async () => {
		const mockResponse = { token: 'jwt.token.here', username: 'bob' };
		const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
			ok: true,
			status: 200,
			json: async () => mockResponse
		} as unknown as Response);

		const result = await api.login('bob', 'secretpass');

		expect(fetchSpy).toHaveBeenCalledWith('/api/auth/login', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ username: 'bob', password: 'secretpass' })
		});
		expect(result).toEqual(mockResponse);
	});

	it('attaches Authorization header when token is provided', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
			ok: true,
			status: 200,
			json: async () => []
		} as unknown as Response);

		await api.myHistory('my-token-123');

		expect(fetchSpy).toHaveBeenCalledWith('/api/me/history', {
			method: 'GET',
			headers: {
				Authorization: 'Bearer my-token-123'
			},
			body: undefined
		});
	});

	it('submits quiz with attempt_id included when present', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
			ok: true,
			status: 200,
			json: async () => ({
				attempt_id: 'att-1',
				score: 10,
				total_questions: 10,
				results: []
			})
		} as unknown as Response);

		const answers = [{ question_id: 'q1', selected_option_id: 'o1' }];
		await api.submitQuiz('quiz-123', answers, 'user-token', 'attempt-456');

		expect(fetchSpy).toHaveBeenCalledWith('/api/quizzes/quiz-123/submit', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Authorization: 'Bearer user-token'
			},
			body: JSON.stringify({ answers, attempt_id: 'attempt-456' })
		});
	});

	it('throws ApiError with server error message on 400/409', async () => {
		vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
			ok: false,
			status: 409,
			json: async () => ({ error: 'username already taken' })
		} as unknown as Response);

		let thrown: unknown;
		try {
			await api.register('alice', 'pass');
		} catch (err) {
			thrown = err;
		}

		expect(thrown).toBeInstanceOf(ApiError);
		const apiErr = thrown as ApiError;
		expect(apiErr.status).toBe(409);
		expect(apiErr.message).toBe('username already taken');
	});

	it('throws ApiError with fallback status message when response is not JSON', async () => {
		vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
			ok: false,
			status: 502,
			json: async () => {
				throw new Error('Bad Gateway HTML');
			}
		} as unknown as Response);

		try {
			await api.listQuizzes();
			expect.unreachable('should have thrown');
		} catch (err: unknown) {
			const apiErr = err as ApiError;
			expect(apiErr.status).toBe(502);
			expect(apiErr.message).toBe('request failed (502)');
		}
	});
});
