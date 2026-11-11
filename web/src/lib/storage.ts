/** Stored text, or undefined when missing or storage is blocked. */
export function readText(key: string): string | undefined {
	try {
		return localStorage.getItem(key) ?? undefined;
	} catch {
		return undefined;
	}
}

/** Stores text, quietly doing nothing when storage is blocked. */
export function writeText(key: string, text: string): void {
	try {
		localStorage.setItem(key, text);
	} catch {
		return;
	}
}

export function forget(key: string): void {
	try {
		localStorage.removeItem(key);
	} catch {
		return;
	}
}

/** A stored JSON value, or undefined when missing or unreadable. */
export function readJson<T>(key: string): T | undefined {
	const text = readText(key);
	try {
		return text === undefined ? undefined : (JSON.parse(text) as T);
	} catch {
		return undefined;
	}
}

export function writeJson(key: string, value: unknown): void {
	writeText(key, JSON.stringify(value));
}
