<script lang="ts">
  import { focusTrap } from '$lib/actions/focusTrap';

  // `kind` drives every label/copy/default in the dialog. The shape and
  // ceremony are identical for both flows; only the copy changes —
  // hence one component instead of two near-duplicates.
  type ProjectKind = 'film' | 'series';
  let { open = $bindable(false), kind = 'series' as ProjectKind, onConfirm } = $props<{
    open: boolean;
    kind?: ProjectKind;
    onConfirm: (title: string) => void | Promise<void>;
  }>();

  // Per-kind copy table — keeps the markup below clean and makes it
  // obvious where to add a new project type.
  const COPY = {
    film: {
      eyebrow: 'A New Film',
      headlineHtml: 'Title <em>your</em> film',
      fieldLabel: 'Film title',
      placeholder: 'e.g. The Last Reel',
      hint: 'You can edit metadata, draft number, and contact info after the film is created.',
      confirmLabel: 'Create Film',
      fallback: 'Untitled',
    },
    series: {
      eyebrow: 'A New Series',
      headlineHtml: 'Title <em>your</em> series',
      fieldLabel: 'Series title',
      placeholder: 'e.g. The Return',
      hint: 'You can add episodes and edit metadata after the series is created.',
      confirmLabel: 'Create Series',
      fallback: 'Untitled Series',
    },
  } as const;

  let copy = $derived(COPY[(kind ?? 'series') as ProjectKind]);

  let title = $state('');
  let inputEl = $state<HTMLInputElement | null>(null);

  // Clear and focus the field each time the dialog is opened.
  $effect(() => {
    if (open) {
      title = '';
      // Focus on next microtask so the input exists in the DOM.
      queueMicrotask(() => inputEl?.focus());
    }
  });

  function handleCreate() {
    const trimmed = title.trim() || copy.fallback;
    onConfirm(trimmed);
  }

  function handleCancel() {
    open = false;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      open = false;
    } else if (event.key === 'Enter') {
      event.preventDefault();
      handleCreate();
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      open = false;
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="modal-backdrop"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div class="modal-card" use:focusTrap>
      <!-- Editorial masthead — eyebrow + display title. Reads as the
           opening page of a film/series bible rather than a generic
           dialog. Copy is per-kind via the COPY table above. -->
      <header class="masthead">
        <div class="mh-eyebrow is-centered" aria-hidden="true">
          <span class="mh-rule"></span>
          <span>{copy.eyebrow}</span>
          <span class="mh-rule"></span>
        </div>
        <h2 id="new-project-heading" class="series-display">{@html copy.headlineHtml}</h2>
        <button class="btn-close" onclick={handleCancel} aria-label="Close">&times;</button>
      </header>

      <div class="modal-body">
        <label class="field">
          <span class="field-label">{copy.fieldLabel}</span>
          <input
            bind:this={inputEl}
            bind:value={title}
            type="text"
            placeholder={copy.placeholder}
            autocomplete="off"
            spellcheck="false"
          />
        </label>
        <p class="hint">{copy.hint}</p>
      </div>

      <div class="modal-footer">
        <button class="btn-ghost" onclick={handleCancel}>Cancel</button>
        <button class="btn-primary" onclick={handleCreate}>{copy.confirmLabel}</button>
      </div>
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
    z-index: var(--modal-z-stacked);
    font-family: system-ui, -apple-system, sans-serif;
  }

  .modal-card {
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: var(--modal-radius);
    width: var(--modal-w-sm);
    max-width: 90vw;
    box-shadow: var(--modal-shadow);
    animation: modal-in var(--modal-anim-duration) ease-out;
    display: flex;
    flex-direction: column;
  }

  @keyframes modal-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }

  /* Centered editorial masthead — replaces the flat header bar. The
     close button floats top-right via absolute positioning so the
     centered composition stays balanced. */
  .masthead {
    position: relative;
    text-align: center;
    padding: 28px 24px 18px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }

  .series-display {
    margin: 0;
    font-family: var(--display-font);
    font-size: 26px;
    font-weight: 600;
    line-height: 1.1;
    letter-spacing: -0.01em;
    color: var(--text-primary);
  }

  /* `:global` because the <em> is injected via {@html} from the COPY
     table, so Svelte's CSS pruner can't see it statically and would
     otherwise drop this rule. */
  :global(.series-display em) {
    font-style: italic;
    font-weight: 500;
    color: var(--marker-color);
  }

  .btn-close {
    position: absolute;
    top: 14px;
    right: 14px;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-close:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .modal-body {
    padding: 24px 24px 8px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  /* Tracked-caps eyebrow label so the single field reads as a
     ceremonial inscription, not a form input. */
  .field-label {
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--text-muted);
    text-align: center;
  }

  /* Bare-bones input — no rounded box. Just a hairline rule under a
     centered display-font value. The series title is the only field
     so it gets all the visual weight. */
  .field input {
    height: 44px;
    padding: 0 6px;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border-medium);
    border-radius: 0;
    color: var(--text-primary);
    font-family: var(--display-font);
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.005em;
    text-align: center;
    outline: none;
    transition: border-color 160ms ease;
  }

  .field input::placeholder {
    color: var(--text-muted);
    font-style: italic;
    font-weight: 400;
  }

  .field input:focus {
    border-bottom-color: var(--accent);
  }

  .hint {
    margin: 4px 0 0;
    font-size: 12px;
    font-style: italic;
    color: var(--text-muted);
    text-align: center;
    line-height: 1.5;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px 16px;
    border-top: 1px solid var(--border-subtle);
  }

  .btn-ghost,
  .btn-primary {
    height: 30px;
    padding: 0 14px;
    border-radius: 6px;
    font-size: 12.5px;
    font-family: inherit;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-ghost {
    background: transparent;
    border: 1px solid var(--border-medium);
    color: var(--text-secondary);
  }

  .btn-ghost:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .btn-primary {
    background: var(--accent);
    border: 1px solid var(--accent);
    color: var(--text-on-accent);
    font-weight: 500;
  }

  .btn-primary:hover {
    filter: brightness(1.05);
  }
</style>
