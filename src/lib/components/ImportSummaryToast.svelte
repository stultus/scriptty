<script lang="ts">
	// Bottom-left toast that surfaces what an importer transformed or
	// dropped. Shared by Fountain (#187) and Final Draft (#192) — the
	// discriminated `kind` field selects which counts to surface.
	//
	// Shown once per import; the writer dismisses it explicitly — auto-
	// dismissing would mean a writer who looked away can miss warnings
	// (e.g. the Malayalam @-prefix warning that explains why all their
	// character cues landed as action).
	//
	// Lives at the same screen z-index as UpdateToast but anchored bottom-
	// left to keep both visible if a release happens to drop while a writer
	// is importing a file.

	import type { AnyImportSummary } from '$lib/stores/documentStore.svelte';

	interface Props {
		summary: AnyImportSummary | null;
		filename: string;
		onDismiss: () => void;
	}

	let { summary, filename, onDismiss }: Props = $props();

	// Build a human-readable counts blurb. We deliberately omit zero counts
	// so the toast doesn't spell out the boring news.
	const counts = $derived.by(() => {
		if (!summary) return [] as string[];
		const out: string[] = [];
		if (summary.kind === 'fountain') {
			if (summary.synopses_count > 0)
				out.push(plural(summary.synopses_count, 'synopsis', 'synopses'));
			if (summary.sections_count > 0) out.push(plural(summary.sections_count, 'section'));
			if (summary.notes_count > 0) out.push(plural(summary.notes_count, 'note'));
			if (summary.dual_dialogue_count > 0)
				out.push(
					`${summary.dual_dialogue_count} dual-dialogue cue${summary.dual_dialogue_count === 1 ? '' : 's'} (collapsed)`
				);
			if (summary.boneyards_dropped > 0)
				out.push(
					`${summary.boneyards_dropped} boneyard${summary.boneyards_dropped === 1 ? '' : 's'} dropped`
				);
			if (summary.scene_numbers_dropped > 0)
				out.push(
					`${summary.scene_numbers_dropped} scene number${summary.scene_numbers_dropped === 1 ? '' : 's'} dropped`
				);
			if (summary.emphasis_stripped > 0)
				out.push(
					`${summary.emphasis_stripped} emphasis run${summary.emphasis_stripped === 1 ? '' : 's'} stripped`
				);
		} else {
			// FDX summary
			if (summary.dual_dialogue_count > 0)
				out.push(
					`${summary.dual_dialogue_count} dual-dialogue block${summary.dual_dialogue_count === 1 ? '' : 's'} (collapsed)`
				);
			if (summary.script_notes_dropped > 0)
				out.push(
					`${summary.script_notes_dropped} script note${summary.script_notes_dropped === 1 ? '' : 's'} dropped`
				);
			if (summary.revisions_dropped > 0)
				out.push(
					`${summary.revisions_dropped} revision${summary.revisions_dropped === 1 ? '' : 's'} dropped`
				);
			if (summary.locked_scene_numbers_dropped > 0)
				out.push(
					`${summary.locked_scene_numbers_dropped} locked scene number${summary.locked_scene_numbers_dropped === 1 ? '' : 's'} dropped`
				);
			if (summary.tag_data_dropped > 0)
				out.push(
					`${summary.tag_data_dropped} tag block${summary.tag_data_dropped === 1 ? '' : 's'} dropped`
				);
			if (summary.unknown_types_folded_to_action > 0)
				out.push(
					`${summary.unknown_types_folded_to_action} unknown paragraph type${summary.unknown_types_folded_to_action === 1 ? '' : 's'} folded to action`
				);
		}
		return out;
	});

	// Eyebrow label changes per format so the toast tells the writer
	// which importer ran without them having to read the filename.
	const eyebrowLabel = $derived(
		summary?.kind === 'fdx' ? 'Final Draft imported' : 'Fountain imported'
	);

	function plural(n: number, singular: string, pluralForm: string = singular + 's'): string {
		return `${n} ${n === 1 ? singular : pluralForm}`;
	}
</script>

{#if summary}
	<div class="import-toast" role="status" aria-live="polite">
		<div class="mh-eyebrow is-centered" aria-hidden="true">
			<span class="mh-rule"></span>
			<span>{eyebrowLabel}</span>
			<span class="mh-rule"></span>
		</div>

		<h2 class="import-title">
			{filename}
		</h2>

		{#if counts.length > 0}
			<p class="import-deck">{counts.join(' · ')}</p>
		{:else}
			<p class="import-deck"><em>Clean import.</em></p>
		{/if}

		{#if summary.warnings.length > 0}
			<ul class="import-warnings">
				{#each summary.warnings as warning (warning)}
					<li>{warning}</li>
				{/each}
			</ul>
		{/if}

		<div class="import-actions">
			<button type="button" class="primary" onclick={onDismiss}>Got it</button>
		</div>
	</div>
{/if}

<style>
	.import-toast {
		position: fixed;
		bottom: 22px;
		left: 22px;
		z-index: 900;
		width: 360px;
		padding: 18px 22px 16px;
		background: var(--surface-float);
		border: 1px solid var(--border-medium);
		border-radius: 10px;
		box-shadow:
			0 14px 36px var(--shadow-heavy),
			0 2px 6px var(--shadow-soft);
		animation: import-slide-in var(--motion-slow) cubic-bezier(0.2, 0.7, 0.3, 1);
		text-align: left;
		background-image: var(--page-grain);
		background-repeat: repeat;
		background-size: 240px 240px;
	}

	@keyframes import-slide-in {
		from {
			transform: translateY(10px);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	.import-toast :global(.mh-eyebrow) {
		margin-bottom: 10px;
		/* Eyebrow is centred for the upstream component but reads better
       left-aligned here since the title and counts below are flush left. */
		justify-content: flex-start;
	}

	.import-title {
		margin: 0;
		font-family: var(--display-font);
		font-size: 18px;
		font-weight: 600;
		letter-spacing: -0.01em;
		line-height: 1.2;
		color: var(--text-primary);
		/* Long filenames get truncated rather than wrapping into something
       that crowds the toast vertically. */
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.import-deck {
		margin: 8px 0 0;
		font-family: var(--display-font);
		font-size: 13px;
		line-height: 1.45;
		color: var(--text-muted);
	}

	.import-deck em {
		font-style: italic;
		color: var(--accent);
	}

	.import-warnings {
		margin: 12px 0 0;
		padding: 10px 12px;
		list-style: none;
		background: var(--surface-soft);
		border-left: 2px solid var(--marker-color);
		font-size: 12px;
		line-height: 1.4;
		color: var(--text-secondary);
		border-radius: 0 4px 4px 0;
	}

	.import-warnings li + li {
		margin-top: 6px;
	}

	.import-actions {
		margin-top: 14px;
		display: flex;
		justify-content: flex-end;
	}

	.import-actions .primary {
		padding: 7px 14px;
		border-radius: 6px;
		font-size: 12.5px;
		font-weight: 500;
		border: 1px solid transparent;
		cursor: pointer;
		font-family: var(--ui-font);
		background: var(--accent);
		color: var(--text-on-accent);
		transition:
			background-color var(--motion-fast, 120ms) ease,
			color var(--motion-fast, 120ms) ease;
	}

	.import-actions .primary:hover {
		background: var(--accent-hover);
	}

	.import-actions .primary:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
</style>
