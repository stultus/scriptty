import { getVersion } from '@tauri-apps/api/app';

interface UpdateInfo {
	currentVersion: string;
	latestVersion: string;
	releaseUrl: string;
}

const DOWNLOADS_URL = 'https://stultus.in/scriptty/downloads.json';
const DISMISSED_KEY = 'scriptty-update-dismissed-version';

class UpdateStore {
	available = $state<UpdateInfo | null>(null);

	async check(): Promise<void> {
		try {
			const currentVersion = await getVersion();
			const res = await fetch(DOWNLOADS_URL, { cache: 'no-store' });
			if (!res.ok) return;
			const data = (await res.json()) as { version?: string; release_url?: string };
			if (!data.version || !data.release_url) return;
			if (compareVersions(data.version, currentVersion) <= 0) return;
			if (
				typeof localStorage !== 'undefined' &&
				localStorage.getItem(DISMISSED_KEY) === data.version
			) {
				return;
			}
			this.available = {
				currentVersion,
				latestVersion: data.version,
				releaseUrl: data.release_url
			};
		} catch {
			// Offline or fetch blocked — stay silent so the app remains fully usable.
		}
	}

	dismiss(): void {
		if (this.available && typeof localStorage !== 'undefined') {
			localStorage.setItem(DISMISSED_KEY, this.available.latestVersion);
		}
		this.available = null;
	}
}

export const updateStore = new UpdateStore();

/**
 * Compare two semver-ish strings (e.g. "0.6.0" vs "0.6.1").
 * Returns 1 if a > b, -1 if a < b, 0 if equal. Pre-release suffixes are ignored.
 */
function compareVersions(a: string, b: string): number {
	const strip = (v: string) => v.replace(/^v/, '').split(/[-+]/)[0];
	const pa = strip(a).split('.').map((n) => Number(n) || 0);
	const pb = strip(b).split('.').map((n) => Number(n) || 0);
	const len = Math.max(pa.length, pb.length);
	for (let i = 0; i < len; i++) {
		const x = pa[i] ?? 0;
		const y = pb[i] ?? 0;
		if (x > y) return 1;
		if (x < y) return -1;
	}
	return 0;
}
