<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { focusTrap } from '$lib/actions/focusTrap';

  // Props using Svelte 5 $props rune
  let { open = $bindable(false) } = $props<{ open: boolean }>();

  // Local form state — initialized from document meta when modal opens
  let title = $state('');
  let author = $state('');
  let director = $state('');
  let contact = $state('');
  let draftNumber = $state(1);
  let draftDate = $state('');
  let titleTouched = $state(false);

  let titleInvalid = $derived(!title.trim());

  // When modal opens, populate form from current document meta
  $effect(() => {
    if (open && documentStore.document) {
      const meta = documentStore.document.meta;
      title = meta.title || '';
      author = meta.author || '';
      director = meta.director || '';
      contact = meta.contact || '';
      draftNumber = meta.draft_number || 1;
      draftDate = meta.draft_date || '';
    }
  });

  function handleSave() {
    titleTouched = true;
    if (titleInvalid) return;
    if (documentStore.document) {
      documentStore.document.meta.title = title.trim();
      documentStore.document.meta.author = author;
      documentStore.document.meta.director = director;
      documentStore.document.meta.contact = contact;
      documentStore.document.meta.draft_number = draftNumber;
      documentStore.document.meta.draft_date = draftDate;
      documentStore.markDirty();
    }
    open = false;
  }

  function handleCancel() {
    open = false;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      open = false;
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
  <div class="modal-backdrop" onclick={handleBackdropClick} onkeydown={handleKeydown} role="dialog" aria-modal="true" tabindex="-1">
    <div class="modal-card" use:focusTrap>
      <div class="modal-header">
        <h2>Screenplay Info</h2>
        <button class="btn-close" onclick={handleCancel}>&times;</button>
      </div>

      <div class="form-group">
        <label for="meta-title">Title <span class="required">*</span></label>
        <input
          id="meta-title"
          type="text"
          bind:value={title}
          onblur={() => (titleTouched = true)}
          placeholder="e.g. The Great Screenplay"
          required
          aria-required="true"
          aria-invalid={titleTouched && titleInvalid}
          class:invalid={titleTouched && titleInvalid}
        />
        {#if titleTouched && titleInvalid}
          <span class="error-text" role="alert">Title is required.</span>
        {/if}
      </div>

      <div class="form-group">
        <label for="meta-author">Author</label>
        <input id="meta-author" type="text" bind:value={author} placeholder="Writer name(s)" />
      </div>

      <div class="form-group">
        <label for="meta-director">Director</label>
        <input id="meta-director" type="text" bind:value={director} placeholder="Director name" />
      </div>

      <div class="form-group">
        <label for="meta-contact">Contact</label>
        <textarea id="meta-contact" rows="3" bind:value={contact} placeholder="Address / Phone / Email"></textarea>
      </div>

      <div class="form-row">
        <div class="form-group half">
          <label for="meta-draft">Draft number</label>
          <input id="meta-draft" type="number" min="1" bind:value={draftNumber} />
        </div>
        <div class="form-group half">
          <label for="meta-date">Draft date</label>
          <input id="meta-date" type="date" bind:value={draftDate} />
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn-ghost" onclick={handleCancel}>Cancel</button>
        <button
          class="btn-primary"
          onclick={handleSave}
          disabled={titleInvalid}
          aria-disabled={titleInvalid}
          title={titleInvalid ? 'Title is required' : 'Save'}
        >Save</button>
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
    z-index: 1000;
  }

  .modal-card {
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 12px;
    padding: 24px;
    width: 480px;
    max-width: 90vw;
    box-shadow: 0 8px 32px var(--shadow-heavy);
    animation: modal-in 150ms ease-out;
  }

  @keyframes modal-in {
    from {
      opacity: 0;
      transform: scale(0.97);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 15px;
    color: var(--text-primary);
    font-family: system-ui, -apple-system, sans-serif;
    font-weight: 600;
  }

  .btn-close {
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
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-close:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .form-group {
    margin-bottom: 14px;
  }

  .form-group label {
    display: block;
    margin-bottom: 4px;
    font-size: 13px;
    color: var(--text-secondary);
    font-family: system-ui, -apple-system, sans-serif;
  }

  .form-group input,
  .form-group textarea {
    width: 100%;
    padding: 10px 12px;
    font-size: 13px;
    color: var(--text-primary);
    background: var(--surface-base);
    border: 1px solid var(--border-medium);
    border-radius: 8px;
    font-family: system-ui, -apple-system, sans-serif;
    box-sizing: border-box;
    transition: border-color 120ms ease;
  }

  .form-group input:focus,
  .form-group textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  .form-group input.invalid {
    border-color: var(--error);
  }

  .form-group .required {
    color: var(--error);
    margin-left: 2px;
  }

  .form-group .error-text {
    display: block;
    margin-top: 4px;
    font-size: 11px;
    color: var(--error);
  }

  .form-group textarea {
    resize: vertical;
  }

  .form-row {
    display: flex;
    gap: 12px;
  }

  .form-group.half {
    flex: 1;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 20px;
  }

  .btn-ghost {
    height: 28px;
    padding: 0 12px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-ghost:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .btn-primary {
    height: 28px;
    padding: 0 12px;
    border-radius: 6px;
    border: none;
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    font-family: system-ui, -apple-system, sans-serif;
    cursor: pointer;
    transition: background 120ms ease;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }
</style>
