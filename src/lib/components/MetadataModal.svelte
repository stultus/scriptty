<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { focusTrap } from '$lib/actions/focusTrap';

  // Props using Svelte 5 $props rune
  let { open = $bindable(false) } = $props<{ open: boolean }>();

  // Local form state — initialized from document meta when modal opens
  let title = $state('');
  let episodeTitle = $state('');
  let tagline = $state('');
  let author = $state('');
  let director = $state('');
  let contact = $state('');
  let registrationNumber = $state('');
  let footnote = $state('');
  let draftNumber = $state(1);
  let draftDate = $state('');
  let titleTouched = $state(false);

  let titleInvalid = $derived(!title.trim());
  let isSeries = $derived(documentStore.isSeries);

  /** Smart credit line for the title-page preview — matches the PDF
   *  generator's combined "Written and Directed by" treatment when the
   *  same person fills both roles, separate lines otherwise. */
  let creditLines = $derived.by<Array<{ label: string; name: string }>>(() => {
    const a = author.trim();
    const d = director.trim();
    if (!a && !d) return [];
    if (a && d && a === d) return [{ label: 'Written and Directed by', name: a }];
    const lines: Array<{ label: string; name: string }> = [];
    if (a) lines.push({ label: 'Written by', name: a });
    if (d) lines.push({ label: 'Directed by', name: d });
    return lines;
  });

  /** Pretty-formatted draft date — falls back to the raw value if the
   *  string isn't a parseable ISO date (e.g. the writer typed "Day 3"). */
  let formattedDraftDate = $derived.by<string>(() => {
    if (!draftDate.trim()) return '';
    const ms = Date.parse(draftDate);
    if (Number.isNaN(ms)) return draftDate;
    return new Date(ms).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  });

  // Populate the form from the active meta (series → active episode,
  // film → top-level) on open, and re-populate if the writer switches
  // episodes while the modal is still open — reading activeEpisode?.id
  // inside the effect makes it a dependency so Svelte reruns the block
  // on episode change, not just on the open → true transition.
  $effect(() => {
    if (!open) return;
    const activeEpId = documentStore.activeEpisode?.id;
    void activeEpId; // explicit dependency on the active episode
    const meta = documentStore.activeMeta;
    if (!meta) return;
    title = meta.title || '';
    episodeTitle = documentStore.activeEpisode?.title || '';
    tagline = meta.tagline || '';
    author = meta.author || '';
    director = meta.director || '';
    contact = meta.contact || '';
    registrationNumber = meta.registration_number || '';
    footnote = meta.footnote || '';
    draftNumber = meta.draft_number || 1;
    draftDate = meta.draft_date || '';
    titleTouched = false;
  });

  function handleSave() {
    titleTouched = true;
    if (titleInvalid) return;
    const meta = documentStore.activeMeta;
    if (meta) {
      meta.title = title.trim();
      meta.tagline = tagline.trim();
      meta.author = author;
      meta.director = director;
      meta.contact = contact;
      meta.registration_number = registrationNumber.trim();
      meta.footnote = footnote.trim();
      meta.draft_number = draftNumber;
      meta.draft_date = draftDate;
      if (isSeries) {
        // Write the short episode label back to `ep.title` — this is what
        // Scene Navigator shows and what series PDF exports use as the
        // per-episode heading, independent from `meta.title` (which drives
        // the title page).
        const ep = documentStore.activeEpisode;
        if (ep) ep.title = episodeTitle.trim();
      }
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
    <div class="modal-card meta-card" use:focusTrap>
      <header class="meta-header">
        <div class="header-text">
          <span class="header-eyebrow">Title page · cover sheet</span>
          <h2>Metadata</h2>
        </div>
        <button class="btn-close" onclick={handleCancel} aria-label="Close metadata">&times;</button>
      </header>

      {#if documentStore.isSeries}
        <div class="series-strip">
          <span class="series-strip-label">Series</span>
          <span class="series-strip-value">{documentStore.document?.series?.title || 'Untitled Series'}</span>
          <span class="series-strip-ep">Episode {documentStore.activeEpisode?.number ?? ''}{documentStore.activeEpisode?.title ? ` — ${documentStore.activeEpisode.title}` : ''}</span>
        </div>
      {/if}

      <div class="meta-body">
        <!-- ── Form column ────────────────────────────────────── -->
        <section class="meta-form" aria-labelledby="form-heading">
          <h3 id="form-heading" class="form-section-heading">Identity</h3>

          <div class="field">
            <label for="meta-title">
              <span class="field-name">Title</span>
              <span class="field-meta required">required</span>
            </label>
            <input
              id="meta-title"
              type="text"
              bind:value={title}
              onblur={() => (titleTouched = true)}
              placeholder={isSeries ? "Used on this episode's title page" : 'e.g. The Great Screenplay'}
              required
              aria-required="true"
              aria-invalid={titleTouched && titleInvalid}
              class:invalid={titleTouched && titleInvalid}
            />
            {#if titleTouched && titleInvalid}
              <span class="error-text" role="alert">Title is required.</span>
            {/if}
          </div>

          {#if isSeries}
            <div class="field">
              <label for="meta-episode-title">
                <span class="field-name">Episode title</span>
                <span class="field-meta">Navigator · series header</span>
              </label>
              <input
                id="meta-episode-title"
                type="text"
                bind:value={episodeTitle}
                placeholder="e.g. Pilot, The Return"
              />
            </div>
          {/if}

          <div class="field">
            <label for="meta-tagline">
              <span class="field-name">Tagline</span>
              <span class="field-meta">Renders under the title</span>
            </label>
            <input id="meta-tagline" type="text" bind:value={tagline} placeholder="A one-line logline" />
          </div>

          <div class="field-row">
            <div class="field">
              <label for="meta-author">
                <span class="field-name">Author</span>
              </label>
              <input id="meta-author" type="text" bind:value={author} placeholder="Writer name(s)" />
            </div>
            <div class="field">
              <label for="meta-director">
                <span class="field-name">Director</span>
              </label>
              <input id="meta-director" type="text" bind:value={director} placeholder="Director name" />
            </div>
          </div>

          <h3 class="form-section-heading">Production</h3>

          <div class="field">
            <label for="meta-contact">
              <span class="field-name">Contact</span>
              <span class="field-meta">Address · phone · email</span>
            </label>
            <textarea id="meta-contact" rows="3" bind:value={contact} placeholder="Bottom-left of the title page"></textarea>
          </div>

          <div class="field-row">
            <div class="field">
              <label for="meta-draft">
                <span class="field-name">Draft #</span>
              </label>
              <input id="meta-draft" type="number" min="1" bind:value={draftNumber} />
            </div>
            <div class="field">
              <label for="meta-date">
                <span class="field-name">Draft date</span>
              </label>
              <input id="meta-date" type="date" bind:value={draftDate} />
            </div>
          </div>

          <div class="field">
            <label for="meta-reg">
              <span class="field-name">Registration number</span>
              <span class="field-meta">WGA · film board · copyright</span>
            </label>
            <input id="meta-reg" type="text" bind:value={registrationNumber} placeholder="Optional" />
          </div>

          <div class="field">
            <label for="meta-footnote">
              <span class="field-name">Footnote</span>
              <span class="field-meta">Bottom of the title page</span>
            </label>
            <textarea id="meta-footnote" rows="2" bind:value={footnote} placeholder="Confidentiality line, 'based on...', dedication"></textarea>
          </div>
        </section>

        <!-- ── Live preview column ────────────────────────────── -->
        <aside class="meta-preview" aria-label="Title page preview">
          <div class="preview-eyebrow">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
              <path d="M2 12 s4 -7 10 -7 s10 7 10 7 s-4 7 -10 7 s-10 -7 -10 -7 z"/>
              <circle cx="12" cy="12" r="2.5"/>
            </svg>
            <span>Title page preview</span>
          </div>

          <!-- Stylized mini title page — sized to fit the preview pane,
               proportions roughly match the printed PDF so the writer
               sees a faithful preview without rendering Typst. -->
          <div class="preview-page">
            {#if registrationNumber.trim()}
              <div class="preview-reg">{registrationNumber}</div>
            {/if}
            <div class="preview-title-block">
              <div class="preview-title" class:placeholder={!title.trim()}>
                {title.trim() || 'Untitled'}
              </div>
              {#if tagline.trim()}
                <div class="preview-tagline">{tagline}</div>
              {/if}
            </div>

            {#if creditLines.length > 0}
              <div class="preview-credits">
                {#each creditLines as line}
                  <div class="preview-credit">
                    <span class="preview-credit-label">{line.label}</span>
                    <span class="preview-credit-name">{line.name}</span>
                  </div>
                {/each}
              </div>
            {:else}
              <div class="preview-credits muted">
                <div class="preview-credit-placeholder">add an author or director...</div>
              </div>
            {/if}

            <div class="preview-bottom">
              {#if contact.trim()}
                <div class="preview-contact">{contact}</div>
              {/if}
              <div class="preview-draft">
                <span>Draft {draftNumber}</span>
                {#if formattedDraftDate}
                  <span class="preview-draft-sep">·</span>
                  <span>{formattedDraftDate}</span>
                {/if}
              </div>
              {#if footnote.trim()}
                <div class="preview-footnote">{footnote}</div>
              {/if}
            </div>
          </div>

          <p class="preview-note">A faithful preview of the cover sheet — exported title pages will use the selected font.</p>
        </aside>
      </div>

      <footer class="meta-footer">
        <button class="btn-ghost" onclick={handleCancel}>Cancel</button>
        <div class="footer-spacer"></div>
        <button
          class="btn-primary"
          onclick={handleSave}
          disabled={titleInvalid}
          aria-disabled={titleInvalid}
          title={titleInvalid ? 'Title is required' : 'Save metadata'}
        >Save metadata</button>
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
    z-index: var(--modal-z);
  }

  /* Wide two-pane layout matched to Statistics / Export — sized so the
     full form is visible without scrolling on most viewports, and so the
     live title-page preview earns its place beside the form. */
  .meta-card {
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: var(--modal-radius);
    width: 800px;
    max-width: 92vw;
    height: 78vh;
    max-height: 720px;
    box-shadow: var(--modal-shadow);
    animation: modal-in var(--modal-anim-duration) ease-out;
    font-family: var(--ui-font);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  @keyframes modal-in {
    from { opacity: 0; transform: scale(0.97); }
    to   { opacity: 1; transform: scale(1); }
  }

  /* ─── Header ─── */
  .meta-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 22px 28px 18px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .header-text {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .header-eyebrow {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .meta-header h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.2;
  }

  .btn-close {
    flex-shrink: 0;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    font-size: 20px;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-close:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  /* ─── Series strip ─── */
  .series-strip {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
    padding: 12px 28px;
    background: var(--surface-base);
    border-bottom: 1px solid var(--border-subtle);
    font-size: 11.5px;
    color: var(--text-secondary);
  }

  .series-strip-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .series-strip-value {
    color: var(--text-primary);
    font-weight: 600;
  }

  .series-strip-ep {
    color: var(--text-muted);
  }

  /* ─── Two-pane body ─── */
  .meta-body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1.05fr) minmax(0, 1fr);
  }

  .meta-form {
    overflow-y: auto;
    padding: 22px 28px 24px;
  }

  .form-section-heading {
    margin: 0 0 12px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .form-section-heading + .field {
    margin-top: 0;
  }

  .form-section-heading:not(:first-child) {
    margin-top: 24px;
    padding-top: 20px;
    border-top: 1px solid var(--border-subtle);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 14px;
  }

  .field label {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }

  .field-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .field-meta {
    font-size: 10.5px;
    color: var(--text-muted);
    letter-spacing: 0.02em;
  }

  .field-meta.required {
    color: var(--error);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 9.5px;
  }

  .field input,
  .field textarea {
    width: 100%;
    padding: 9px 11px;
    font-size: 12.5px;
    color: var(--text-primary);
    background: var(--surface-base);
    border: 1px solid var(--border-medium);
    border-radius: 7px;
    font-family: var(--ui-font);
    box-sizing: border-box;
    transition: border-color var(--motion-fast, 100ms) ease,
                background var(--motion-fast, 100ms) ease;
  }

  .field input:focus,
  .field textarea:focus {
    outline: none;
    border-color: var(--accent);
    background: var(--surface-float);
  }

  .field input.invalid {
    border-color: var(--error);
  }

  .field textarea {
    resize: vertical;
    line-height: 1.45;
  }

  .field input[type='number'] {
    font-variant-numeric: tabular-nums;
  }

  .field input[type='date'] {
    font-family: var(--ui-font);
  }

  .error-text {
    font-size: 11px;
    color: var(--error);
  }

  .field-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .field-row .field {
    margin-bottom: 14px;
  }

  /* ─── Live title-page preview ─── */
  .meta-preview {
    background: var(--surface-base);
    border-left: 1px solid var(--border-subtle);
    padding: 22px 24px 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
    overflow-y: auto;
  }

  .preview-eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 10.5px;
    font-weight: 600;
    color: var(--text-muted);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  /* The mini title page itself — proportions tuned to feel like an A4
     page card on the desk, not a literal scale. The cream tone borrows
     from --page-bg so light/dark themes both give the writer a faithful
     mental model of the printed cover. */
  .preview-page {
    flex: 1;
    min-height: 0;
    background: var(--page-bg);
    color: var(--text-on-page);
    border-radius: 4px;
    box-shadow:
      inset 0 1px 0 var(--page-edge-highlight),
      0 1px 2px var(--page-shadow-close),
      0 8px 22px var(--page-shadow);
    padding: 28px 30px 26px;
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 24px;
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
    /* Sit on the desk surface even when the wider preview pane is short,
       so the page never collapses to nothing. */
    min-height: 380px;
    position: relative;
    overflow: hidden;
  }

  .preview-reg {
    position: absolute;
    top: 16px;
    right: 18px;
    font-size: 9px;
    color: rgba(0, 0, 0, 0.5);
    letter-spacing: 0.05em;
  }

  .preview-title-block {
    text-align: center;
    margin-top: 36px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .preview-title {
    font-size: 22px;
    font-weight: 700;
    line-height: 1.18;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    word-break: break-word;
  }

  .preview-title.placeholder {
    color: rgba(0, 0, 0, 0.32);
    font-style: italic;
    font-weight: 500;
    text-transform: none;
  }

  .preview-tagline {
    font-size: 11.5px;
    font-style: italic;
    color: rgba(0, 0, 0, 0.65);
  }

  .preview-credits {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: center;
    align-self: center;
    text-align: center;
  }

  .preview-credits.muted .preview-credit-placeholder {
    color: rgba(0, 0, 0, 0.32);
    font-style: italic;
    font-size: 11px;
  }

  .preview-credit {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .preview-credit-label {
    font-size: 9.5px;
    color: rgba(0, 0, 0, 0.55);
    letter-spacing: 0.04em;
    font-style: italic;
  }

  .preview-credit-name {
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .preview-bottom {
    display: flex;
    flex-direction: column;
    gap: 10px;
    font-size: 10.5px;
    color: rgba(0, 0, 0, 0.7);
    text-align: center;
  }

  .preview-contact {
    white-space: pre-line;
    line-height: 1.5;
  }

  .preview-draft {
    display: inline-flex;
    justify-content: center;
    gap: 6px;
    color: rgba(0, 0, 0, 0.55);
    font-size: 10px;
    letter-spacing: 0.05em;
  }

  .preview-draft-sep {
    color: rgba(0, 0, 0, 0.35);
  }

  .preview-footnote {
    font-size: 9.5px;
    color: rgba(0, 0, 0, 0.55);
    font-style: italic;
    line-height: 1.4;
  }

  .preview-note {
    margin: 0;
    font-size: 10.5px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  /* ─── Footer ─── */
  .meta-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 28px;
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
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-ghost:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .btn-primary {
    height: 32px;
    padding: 0 16px;
    border-radius: 6px;
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--text-on-accent);
    font-family: var(--ui-font);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .btn-primary:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
</style>
