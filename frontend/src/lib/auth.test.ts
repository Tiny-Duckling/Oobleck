import { describe, it, expect, vi, beforeEach } from 'vitest';

// Set up mock localStorage before importing auth
const storage = new Map<string, string>();
globalThis.localStorage = {
	getItem: (key: string) => storage.get(key) ?? null,
	setItem: (key: string, val: string) => {
		storage.set(key, val);
	},
	removeItem: (key: string) => {
		storage.delete(key);
	},
	clear: () => {
		storage.clear();
	},
	length: 0,
	key: () => null
};

import { auth } from './auth.svelte';
import { api } from './api';

describe('auth store', () => {
	beforeEach(() => {
		storage.clear();
		auth.logout();
		vi.restoreAllMocks();
	});

	it('starts logged out when storage is empty', () => {
		expect(auth.isLoggedIn).toBe(false);
		expect(auth.token).toBeNull();
		expect(auth.username).toBeNull();
	});

	it('setAuth updates state and persists credentials to localStorage', () => {
		auth.setAuth('jwt-xyz', 'alice');

		expect(auth.isLoggedIn).toBe(true);
		expect(auth.token).toBe('jwt-xyz');
		expect(auth.username).toBe('alice');

		const stored = JSON.parse(storage.get('quiz_auth') || '{}');
		expect(stored).toEqual({ token: 'jwt-xyz', username: 'alice' });
	});

	it('logout clears state and removes credentials from localStorage', () => {
		auth.setAuth('jwt-xyz', 'alice');
		expect(auth.isLoggedIn).toBe(true);

		auth.logout();

		expect(auth.isLoggedIn).toBe(false);
		expect(auth.token).toBeNull();
		expect(auth.username).toBeNull();
		expect(storage.get('quiz_auth')).toBeUndefined();
	});

	it('login calls api.login and updates auth state on success', async () => {
		vi.spyOn(api, 'login').mockResolvedValueOnce({
			token: 'token-abc',
			username: 'bob'
		});

		await auth.login('bob', 'secretpass');

		expect(api.login).toHaveBeenCalledWith('bob', 'secretpass');
		expect(auth.isLoggedIn).toBe(true);
		expect(auth.token).toBe('token-abc');
		expect(auth.username).toBe('bob');
	});

	it('register calls api.register and updates auth state on success', async () => {
		vi.spyOn(api, 'register').mockResolvedValueOnce({
			token: 'token-def',
			username: 'charlie'
		});

		await auth.register('charlie', 'mypassword');

		expect(api.register).toHaveBeenCalledWith('charlie', 'mypassword');
		expect(auth.isLoggedIn).toBe(true);
		expect(auth.token).toBe('token-def');
		expect(auth.username).toBe('charlie');
	});
});
