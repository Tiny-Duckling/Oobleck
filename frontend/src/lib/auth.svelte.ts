import { api } from './api';

const STORAGE_KEY = 'quiz_auth';

interface StoredAuth {
	token: string;
	username: string;
}

function load(): StoredAuth | null {
	if (typeof localStorage === 'undefined') return null;
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		return raw ? (JSON.parse(raw) as StoredAuth) : null;
	} catch {
		return null;
	}
}

class AuthStore {
	token = $state<string | null>(null);
	username = $state<string | null>(null);

	constructor() {
		const stored = load();
		if (stored) {
			this.token = stored.token;
			this.username = stored.username;
		}
	}

	get isLoggedIn() {
		return this.token !== null;
	}

	setAuth(token: string, username: string) {
		this.token = token;
		this.username = username;
		localStorage.setItem(STORAGE_KEY, JSON.stringify({ token, username }));
	}

	logout() {
		this.token = null;
		this.username = null;
		localStorage.removeItem(STORAGE_KEY);
	}

	async login(username: string, password: string) {
		const res = await api.login(username, password);
		this.setAuth(res.token, res.username);
	}

	async register(username: string, password: string) {
		const res = await api.register(username, password);
		this.setAuth(res.token, res.username);
	}
}

export const auth = new AuthStore();
