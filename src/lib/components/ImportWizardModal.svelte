<script lang="ts">
	import { focusTrap } from '$lib/actions/focusTrap';
	import { documentStore } from '$lib/stores/documentStore.svelte';

	type Format = 'fountain' | 'fdx';
	type Destination = 'film' | 'episode';

	interface Props {
		open: boolean;
		onStart: (format: Format, asEpisode: boolean) => void | Promise<void>;
	}

	let { open = $bindable(false), onStart }: Props = $props();

	let format = $state<Format>('fountain');
	let destination = $state<Destination>('film');

	// Episode destination is shown but disabled in non-series mode.
	// Hiding it would make the menu feel different per project type,
	// which the design skill flagged as a discoverability problem.
	const isSeries = $derived(documentStore.isSeries);
	const seriesTitle = $derived(documentStore.document?.series?.title?.trim() || 'Untitled Series');

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			open = false;
		} else if (event.key === 'Enter') {
			const target = event.target as HTMLElement | null;
			if (target?.tagName !== 'BUTTON') {
				event.preventDefault();
				handleContinue();
			}
		}
	}

	function handleBackdropClick(event: MouseEvent) {
		if (event.target === event.currentTarget) {
			open = false;
		}
	}

	function handleContinue() {
		open = false;
		void onStart(format, destination === 'episode');
	}

	function handleCancel() {
		open = false;
	}

	// focusTrap lands on the first focusable descendant (the close
	// button, in source order). Override so initial focus lands on the
	// first meaningful choice instead.
	let firstFormatBtn: HTMLButtonElement | undefined = $state(undefined);
	$effect(() => {
		if (open) {
			queueMicrotask(() => firstFormatBtn?.focus());
		}
	});
</script>

{#if open}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="modal-backdrop"
		onclick={handleBackdropClick}
		onkeydown={handleKeydown}
		role="dialog"
		aria-modal="true"
		aria-labelledby="import-wizard-title"
		tabindex="-1"
	>
		<div class="modal-card import-wizard-card" use:focusTrap>
			<header class="wizard-header">
				<div class="header-text">
					<div class="mh-eyebrow" aria-hidden="true">
						<span class="mh-rule"></span>
						<span>Bring it in</span>
					</div>
					<h2 id="import-wizard-title" class="mh-title wizard-title">
						Import a <em>screenplay</em><span class="wizard-dot">.</span>
					</h2>
					<p class="wizard-deck">Pick a format and where it lands.</p>
				</div>
				<button class="btn-close" onclick={handleCancel} aria-label="Close import wizard"
					>&times;</button
				>
			</header>

			<div class="wizard-body">
				<!-- role=group + aria-labelledby over <fieldset> because
             fieldset's browser-default sizing collapsed the flex gaps. -->
				<div class="wizard-section" role="group" aria-labelledby="wizard-format-label">
					<div id="wizard-format-label" class="section-label">Format</div>
					<div class="card-grid">
						<button
							type="button"
							class="choice-card"
							class:active={format === 'fountain'}
							aria-pressed={format === 'fountain'}
							bind:this={firstFormatBtn}
							onclick={() => {
								format = 'fountain';
							}}
						>
							<span class="choice-name">Fountain</span>
							<span class="choice-sub"
								>Plain-text screenplay format. Highland, Slugline, WriterDuet, Beat.</span
							>
						</button>
						<button
							type="button"
							class="choice-card"
							class:active={format === 'fdx'}
							aria-pressed={format === 'fdx'}
							onclick={() => {
								format = 'fdx';
							}}
						>
							<span class="choice-name">Final Draft</span>
							<span class="choice-sub"
								>XML format from Final Draft 8 onward — <code>.fdx</code> files.</span
							>
						</button>
					</div>
				</div>

				<div class="wizard-section" role="group" aria-labelledby="wizard-destination-label">
					<div id="wizard-destination-label" class="section-label">Destination</div>
					<div class="card-grid">
						<button
							type="button"
							class="choice-card"
							class:active={destination === 'film'}
							aria-pressed={destination === 'film'}
							onclick={() => {
								destination = 'film';
							}}
						>
							<span class="choice-name">As a new film</span>
							<span class="choice-sub">Replaces the current document.</span>
						</button>
						<button
							type="button"
							class="choice-card"
							class:active={destination === 'episode'}
							class:disabled={!isSeries}
							aria-pressed={destination === 'episode'}
							aria-disabled={!isSeries}
							disabled={!isSeries}
							onclick={() => {
								if (isSeries) destination = 'episode';
							}}
						>
							<span class="choice-name">
								As an episode{#if isSeries}
									of <em class="series-em">{seriesTitle}</em>{/if}
							</span>
							<span class="choice-sub">
								{#if isSeries}
									Appended to the end of your series.
								{:else}
									Open a series first to import as an episode.
								{/if}
							</span>
						</button>
					</div>
				</div>
			</div>

			<footer class="wizard-footer">
				<button class="btn-ghost" onclick={handleCancel}>Cancel</button>
				<div class="footer-spacer"></div>
				<button class="btn-primary" onclick={handleContinue}> Choose file… </button>
			</footer>
		</div>
	</div>
{/if}

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: var(--backdrop);
		backdrop-filter: var(--backdrop-blur);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: var(--modal-z, 1000);
	}

	.import-wizard-card {
		background: var(--surface-float);
		border: 1px solid var(--border-medium);
		border-radius: var(--modal-radius);
		box-shadow: var(--modal-shadow);
		width: var(--modal-w-base);
		max-width: calc(100vw - 32px);
		max-height: calc(100vh - 64px);
		display: flex;
		flex-direction: column;
		overflow: hidden;
		animation: card-in var(--modal-anim-duration, 150ms) ease-out;
	}

	@keyframes card-in {
		from {
			transform: translateY(8px) scale(0.98);
			opacity: 0;
		}
		to {
			transform: translateY(0) scale(1);
			opacity: 1;
		}
	}

	.wizard-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 16px;
		padding: var(--modal-padding) var(--modal-padding) 8px;
	}

	.header-text {
		flex: 1;
		min-width: 0;
	}

	.header-text :global(.mh-eyebrow) {
		margin-bottom: 8px;
	}

	.wizard-title {
		margin: 0;
		font-size: 22px;
		font-weight: 600;
		letter-spacing: -0.015em;
		line-height: 1.15;
		font-family: var(--display-font);
		color: var(--text-primary);
	}

	.wizard-title em {
		font-style: italic;
		color: var(--accent);
		font-weight: 600;
	}

	/* Editorial period — same role as UpdateToast's `.dot`. */
	.wizard-title .wizard-dot {
		color: var(--marker-color, var(--accent));
		font-style: normal;
		font-weight: 700;
	}

	.wizard-deck {
		margin: 10px 0 0;
		font-family: var(--display-font);
		font-style: italic;
		font-size: 13.5px;
		line-height: 1.4;
		color: var(--text-muted);
	}

	.btn-close {
		background: transparent;
		border: none;
		color: var(--text-muted);
		font-size: 24px;
		line-height: 1;
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 6px;
		transition:
			background 120ms ease,
			color 120ms ease;
	}

	.btn-close:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.wizard-body {
		padding: 22px var(--modal-padding) 24px;
		display: flex;
		flex-direction: column;
		gap: 28px;
		overflow-y: auto;
	}

	.wizard-section {
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.section-label {
		font-family: var(--ui-font);
		font-size: 11px;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-muted);
		font-weight: 600;
		line-height: 1;
	}

	.card-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
		align-items: stretch;
	}

	.choice-card {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		text-align: left;
		gap: 6px;
		padding: 14px 16px;
		background: var(--surface-base);
		border: 1px solid var(--border-medium);
		border-radius: 8px;
		cursor: pointer;
		font-family: var(--ui-font);
		color: var(--text-primary);
		/* min-height keeps a row of two short subs from reading as
       underweight; align-items:stretch on the grid handles unequal
       heights between the row's two cards. */
		min-height: 92px;
		transition:
			background 120ms ease,
			border-color 120ms ease,
			color 120ms ease;
	}

	.choice-card:hover:not(.disabled) {
		background: var(--surface-hover);
		border-color: var(--border-strong, var(--border-medium));
	}

	.choice-card.active {
		background: var(--accent-muted, color-mix(in srgb, var(--accent) 14%, transparent));
		border-color: var(--accent);
	}

	.choice-card.disabled {
		opacity: 0.55;
		cursor: not-allowed;
	}

	.choice-card:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.choice-name {
		font-size: 13.5px;
		font-weight: 600;
		line-height: 1.25;
	}

	.choice-name :global(.series-em) {
		font-style: italic;
		color: var(--accent);
		font-weight: 600;
	}

	.choice-sub {
		font-size: 12px;
		color: var(--text-muted);
		line-height: 1.4;
	}

	.choice-sub code {
		font-family: 'Courier Prime', 'Courier New', monospace;
		font-size: 11.5px;
		background: var(--surface-hover);
		padding: 0 4px;
		border-radius: 3px;
		color: var(--text-secondary);
	}

	.wizard-footer {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 14px var(--modal-padding);
		border-top: 1px solid var(--border-subtle);
		background: var(--surface-float);
	}

	.footer-spacer {
		flex: 1;
	}

	.btn-ghost {
		height: 32px;
		padding: 0 14px;
		border-radius: 6px;
		border: 1px solid transparent;
		background: transparent;
		color: var(--text-secondary);
		font-family: var(--ui-font);
		font-size: 12px;
		cursor: pointer;
		transition:
			background 120ms ease,
			color 120ms ease;
	}

	.btn-ghost:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.btn-primary {
		height: 32px;
		padding: 0 18px;
		border-radius: 6px;
		border: 1px solid transparent;
		background: var(--accent);
		color: var(--text-on-accent, white);
		font-family: var(--ui-font);
		font-size: 12.5px;
		font-weight: 500;
		cursor: pointer;
		transition: background 120ms ease;
	}

	.btn-primary:hover {
		background: var(--accent-hover);
	}

	.btn-primary:focus-visible,
	.btn-ghost:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	@media (max-width: 540px) {
		.card-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
