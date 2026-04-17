<script lang="ts">
  import { focusTrap } from '$lib/actions/focusTrap';

  let { open = $bindable(false), onConfirm } = $props<{
    open: boolean;
    onConfirm: (title: string) => void | Promise<void>;
  }>();

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
    const trimmed = title.trim() || 'Untitled Series';
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
      <div class="modal-header">
        <h2>New Series</h2>
        <button class="btn-close" onclick={handleCancel} aria-label="Close">&times;</button>
      </div>

      <div class="modal-body">
        <label class="field">
          <span class="field-label">Series title</span>
          <input
            bind:this={inputEl}
            bind:value={title}
            type="text"
            placeholder="e.g. The Return"
            autocomplete="off"
            spellcheck="false"
          />
        </label>
        <p class="hint">You can add episodes and edit metadata after the series is created.</p>
      </div>

      <div class="modal-footer">
        <button class="btn-ghost" onclick={handleCancel}>Cancel</button>
        <button class="btn-primary" onclick={handleCreate}>Create Series</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--backdrop);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
    font-family: system-ui, -apple-system, sans-serif;
  }

  .modal-card {
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 12px;
    width: 420px;
    max-width: 90vw;
    box-shadow: 0 8px 32px var(--shadow-heavy);
    animation: modal-in 150ms ease-out;
    display: flex;
    flex-direction: column;
  }

  @keyframes modal-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .btn-close {
    width: 26px;
    height: 26px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
    border-radius: 4px;
  }

  .btn-close:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .field input {
    height: 34px;
    padding: 0 10px;
    background: var(--surface-base);
    border: 1px solid var(--border-medium);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    outline: none;
    transition: border-color 120ms ease;
  }

  .field input:focus {
    border-color: var(--accent);
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
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
    color: var(--accent-on);
    font-weight: 500;
  }

  .btn-primary:hover {
    filter: brightness(1.05);
  }
</style>
