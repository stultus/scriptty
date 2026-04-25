import { getVersion } from '@tauri-apps/api/app';

interface UpdateInfo {
	currentVersion: string;
	latestVersion: string;
	releaseUrl: string;
}

/**
 * GitHub's official Releases API. We fetch directly from github.com rather
 * than stultus.in/scriptty/downloads.json because:
 *
 *   1. github.com's TLS cert is pinned at the CA level — an attacker who
 *      compromises stultus.in or hijacks its DNS still can't impersonate
 *      api.github.com to a victim.
 *   2. There is no second JSON file to author, sign, or keep in sync with
 *      releases — `gh release create` already publishes to this endpoint.
 *   3. If stultus.in is ever down, the in-app updater keeps working.
 *
 * The endpoint requires no auth for public repos (60 unauth req/h per IP,
 * which is well over what an in-app "Check for Updates" button generates).
 * (#115)
 */
const RELEASES_API = 'https://api.github.com/repos/stultus/scriptty/releases/latest';

interface GithubReleaseResponse {
	tag_name?: string;
	html_url?: string;
	draft?: boolean;
	prerelease?: boolean;
}

class UpdateStore {
	available = $state<UpdateInfo | null>(null);

	async check(): Promise<'update' | 'current' | 'error'> {
		try {
			const currentVersion = await getVersion();
			const res = await fetch(RELEASES_API, {
				cache: 'no-store',
				headers: { Accept: 'application/vnd.github+json' }
			});
			if (!res.ok) return 'error';
			const data = (await res.json()) as GithubReleaseResponse;
			// Skip drafts and pre-releases — only ship stable updates to users.
			if (data.draft || data.prerelease) return 'current';
			if (!data.tag_name || !data.html_url) return 'error';
			// Defensive: only trust release URLs that actually point at our repo.
			// Protects against a future API quirk where html_url could redirect
			// somewhere unexpected.
			if (!data.html_url.startsWith('https://github.com/stultus/scriptty/releases/')) {
				return 'error';
			}
			if (compareVersions(data.tag_name, currentVersion) <= 0) return 'current';
			this.available = {
				currentVersion,
				latestVersion: data.tag_name.replace(/^v/, ''),
				releaseUrl: data.html_url
			};
			return 'update';
		} catch {
			return 'error';
		}
	}

	dismiss(): void {
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
