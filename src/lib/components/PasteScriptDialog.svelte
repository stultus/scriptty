<script lang="ts">
  import { focusTrap } from '$lib/actions/focusTrap';
  import { parsePastedScript } from '$lib/editor/parsePastedScript';

  let { open = $bindable(false), onConfirm } = $props<{
    open: boolean;
    onConfirm: (parsedContent: unknown) => void | Promise<void>;
  }>();

  let pasted = $state('');
  let textareaEl = $state<HTMLTextAreaElement | null>(null);

  // Live preview — re-parses on every keystroke (cheap; the input is
  // capped by what the user can paste, and the parser is one linear pass).
  let parsed = $derived.by(() => {
    if (!pasted.trim()) return null;
    return parsePastedScript(pasted);
  });

  /** Per-element label for the preview — abbreviated so the legend reads
   *  as a vertical strip of glyphs alongside the rendered text. */
  function elementLabel(type: string): string {
    switch (type) {
      case 'scene_heading': return 'SCENE';
      case 'character':     return 'CHAR';
      case 'parenthetical': return 'PAREN';
      case 'dialogue':      return 'DIAL';
      case 'transition':    return 'TRANS';
      case 'action':        return 'ACTION';
      default:              return type.toUpperCase();
    }
  }

  function elementText(node: { type: string; content?: Array<{ text?: string }> }): string {
    return (node.content ?? []).map((c) => c.text ?? '').join('');
  }

  $effect(() => {
    if (open) {
      pasted = '';
      // Defer focus until the textarea exists in the DOM.
      queueMicrotask(() => textareaEl?.focus());
    }
  });

  function handleCancel() {
    open = false;
  }

  async function handleConfirm() {
    if (!parsed) return;
    await onConfirm(parsed);
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
  <div
    class="modal-backdrop"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div class="modal-card paste-card" use:focusTrap>
      <div class="modal-header">
        <h2>Paste Script</h2>
        <button class="btn-close" onclick={handleCancel} aria-label="Close">&times;</button>
      </div>

      <p class="hint">
        Paste plain text and Scriptty will detect scene headings (INT./EXT.), character names
        (ALL CAPS lines), dialogue, parentheticals, and transitions. You can refine the result in
        the editor afterwards.
      </p>

      <div class="paste-grid">
        <textarea
          bind:this={textareaEl}
          bind:value={pasted}
          class="paste-input"
          placeholder={`INT. KITCHEN — DAY

JOHN
(softly)
We need to talk.

JANE
About what?

CUT TO:`}
          spellcheck="false"
        ></textarea>

        <div class="preview" aria-label="Parsed preview">
          {#if parsed && parsed.content.length > 0}
            <ol class="preview-list">
              {#each parsed.content as node, i (i)}
                <li class="preview-item" data-type={node.type}>
                  <span class="preview-label">{elementLabel(node.type)}</span>
                  <span class="preview-text">{elementText(node) || '(empty)'}</span>
                </li>
              {/each}
            </ol>
          {:else}
            <p class="preview-empty">Preview appears here once you paste.</p>
          {/if}
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn-ghost" onclick={handleCancel}>Cancel</button>
        <button class="btn-primary" onclick={handleConfirm} disabled={!parsed}>Open as Screenplay</button>
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
    font-family: var(--ui-font);
  }

  /* Wider than the standard modal — the preview pane needs room to read. */
  .paste-card {
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: var(--modal-radius);
    padding: var(--modal-padding);
    width: 720px;
    max-width: 92vw;
    max-height: 86vh;
    box-shadow: var(--modal-shadow);
    animation: modal-in var(--modal-anim-duration) ease-out;
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow: hidden;
  }

  @keyframes modal-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .modal-header h2 {
    margin: 0;
    font-size: var(--modal-header-size);
    font-weight: var(--modal-header-weight);
    color: var(--text-primary);
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
    line-height: 1;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-close:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .paste-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    flex: 1;
    min-height: 320px;
    max-height: 56vh;
  }

  .paste-input {
    background: var(--surface-base);
    border: 1px solid var(--border-medium);
    border-radius: 8px;
    padding: 12px;
    color: var(--text-primary);
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 12.5px;
    line-height: 1.5;
    outline: none;
    resize: none;
    transition: border-color 120ms ease;
  }

  .paste-input:focus {
    border-color: var(--accent);
  }

  .preview {
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 8px;
    overflow-y: auto;
  }

  .preview-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .preview-item {
    display: grid;
    grid-template-columns: 60px 1fr;
    gap: 8px;
    padding: 4px 6px;
    border-radius: 4px;
    align-items: baseline;
  }

  .preview-item:hover {
    background: var(--surface-hover);
  }

  .preview-label {
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    text-transform: uppercase;
    font-variant-numeric: tabular-nums;
  }

  /* Color the labels per element type so the parsed structure is visible
     at a glance — saves the writer from reading every row to spot what
     was misclassified. */
  .preview-item[data-type='scene_heading'] .preview-label,
  .preview-item[data-type='transition'] .preview-label {
    color: var(--accent);
  }

  .preview-item[data-type='character'] .preview-label,
  .preview-item[data-type='parenthetical'] .preview-label,
  .preview-item[data-type='dialogue'] .preview-label {
    color: var(--accent-warm, var(--accent));
  }

  .preview-text {
    font-size: 12px;
    color: var(--text-primary);
    font-family: var(--editor-font-en), ui-monospace, monospace;
    overflow-wrap: anywhere;
  }

  .preview-item[data-type='scene_heading'] .preview-text,
  .preview-item[data-type='character'] .preview-text,
  .preview-item[data-type='transition'] .preview-text {
    text-transform: uppercase;
  }

  .preview-empty {
    margin: 0;
    padding: 24px 12px;
    text-align: center;
    font-size: 12px;
    color: var(--text-muted);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
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

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
