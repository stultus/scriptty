<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { focusTrap } from '$lib/actions/focusTrap';
  import DatePicker from './DatePicker.svelte';

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

  /** Title input ref — used to auto-focus the field on open of an
   *  untitled doc (#143) so the writer can start typing the most
   *  important piece of metadata immediately. */
  let titleInputEl = $state<HTMLInputElement | null>(null);

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
    // For untitled docs, focus the Title field so the writer can start
    // typing the most important piece of metadata immediately. For docs
    // that already have a title, leave focus on the focusTrap default
    // (close button) so Escape feels predictable for "I just opened to
    // peek." (#143)
    if (!title.trim()) {
      queueMicrotask(() => titleInputEl?.focus());
    }
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
        <section class="meta-form" aria-label="Metadata fields">
          <!-- Identity section — no heading. The first group is implicit
               (the modal's own title already says "Metadata · Title page"),
               so a redundant "Identity" label was just stealing vertical
               room a 13" laptop didn't have. Production gets the only
               heading because it's a clear shift in intent. -->
          <div class="form-group">
            <div class="field">
              <label for="meta-title">
                <span class="field-name">Title</span>
                <span class="field-meta required">required</span>
              </label>
              <input
                id="meta-title"
                type="text"
                bind:this={titleInputEl}
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
          </div>

          <div class="form-group">
            <h3 class="form-section-heading">Production</h3>

            <div class="field">
              <label for="meta-contact">
                <span class="field-name">Contact</span>
                <span class="field-meta">Address · phone · email</span>
              </label>
              <textarea id="meta-contact" rows="2" bind:value={contact} placeholder="Bottom-left of the title page"></textarea>
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
                <DatePicker bind:value={draftDate} placeholder="Pick a date" />
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

          <!-- Faithful preview of the Typst-rendered title page (see
               generate_title_page_markup in pdf.rs). Layout mirrors the
               printed cover:
                 - Title centered, tagline italic below
                 - Credits centered: small italic label ABOVE each name
                 - Bottom-LEFT block: contact, "Draft N — DATE", "Reg: ..."
                 - Footnote: italic, centered, very bottom of the page
               Aspect ratio matches A4 portrait so what the writer sees on
               screen reads as the same composition Typst produces. -->
          <div class="preview-page" aria-hidden="false">
            <div class="preview-center">
              <div class="preview-title" class:placeholder={!title.trim()}>
                {title.trim() || 'Untitled'}
              </div>
              {#if tagline.trim()}
                <div class="preview-tagline">{tagline}</div>
              {/if}
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
                <div class="preview-credit-placeholder">— add an author or director —</div>
              {/if}
            </div>

            <div class="preview-bottom-left">
              {#if contact.trim()}
                <div class="preview-contact">{contact}</div>
              {/if}
              {#if draftNumber > 0 || formattedDraftDate}
                <div class="preview-draft">
                  {#if draftNumber > 0}<span>Draft {draftNumber}</span>{/if}
                  {#if draftNumber > 0 && formattedDraftDate}<span class="preview-em-dash"> — </span>{/if}
                  {#if formattedDraftDate}<span>{formattedDraftDate}</span>{/if}
                </div>
              {/if}
              {#if registrationNumber.trim()}
                <div class="preview-reg">Reg: {registrationNumber}</div>
              {/if}
            </div>

            {#if footnote.trim()}
              <div class="preview-footnote">{footnote}</div>
            {/if}
          </div>

          <p class="preview-note">Mirrors the printed cover sheet — exports use the selected document font.</p>
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

  /* Wide two-pane layout matched to Statistics / Export. Big enough that
     the form fits without scrolling on standard viewports — the writer
     sees every metadata field at once alongside the live preview. */
  .meta-card {
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: var(--modal-radius);
    width: 1080px;
    max-width: 96vw;
    height: 86vh;
    max-height: 820px;
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
  /* The two columns each scroll independently if needed — but at 1080px
     wide × 86vh tall the form should fit without overflow on any
     reasonable viewport. */
  .meta-body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1.05fr) minmax(0, 1fr);
  }

  .meta-form {
    min-height: 0;
    overflow-y: auto;
    padding: 20px 28px 22px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  /* Scrollbar styled so it visually disappears unless actively scrolling.
     The modal was sized to fit on a normal viewport without needing it,
     but if the writer's screen is short or the form gets new fields, this
     is the gentle fallback. */
  .meta-form::-webkit-scrollbar { width: 6px; }
  .meta-form::-webkit-scrollbar-track { background: transparent; }
  .meta-form::-webkit-scrollbar-thumb {
    background: var(--border-medium);
    border-radius: 3px;
  }

  /* Section group — heading + fields cluster, with a soft divider between
     groups instead of the noisy top-border-on-every-heading. */
  .form-section-heading {
    margin: 0 0 12px;
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--text-muted);
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }

  .form-section-heading::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--border-subtle);
  }

  .form-section-heading + .field {
    margin-top: 0;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
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
    padding: 8px 11px;
    font-size: 12.5px;
    color: var(--text-primary);
    background: var(--surface-base);
    border: 1px solid var(--border-medium);
    border-radius: 7px;
    font-family: var(--ui-font);
    box-sizing: border-box;
    transition: border-color var(--motion-fast, 100ms) ease,
                background var(--motion-fast, 100ms) ease,
                box-shadow var(--motion-fast, 100ms) ease;
  }

  .field input:focus,
  .field textarea:focus {
    outline: none;
    border-color: var(--accent);
    background: var(--surface-float);
    box-shadow: 0 0 0 3px var(--accent-muted);
  }

  .field input.invalid {
    border-color: var(--error);
  }

  .field textarea {
    resize: vertical;
    line-height: 1.45;
    min-height: 64px;
  }

  /* The Footnote textarea is the shortest in the form — keep it small so
     the whole form fits without scrolling on a 13" laptop viewport. */
  #meta-footnote {
    min-height: 52px;
  }

  /* Contact textarea slightly taller than other prose — it commonly
     holds 3-4 short address lines. */
  #meta-contact {
    min-height: 70px;
  }

  .field input[type='number'] {
    font-variant-numeric: tabular-nums;
  }

  .error-text {
    font-size: 11px;
    color: var(--error);
  }

  .field-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }

  /* Section block — wraps a heading + several fields so the gap rule on
     .meta-form spaces sections, not individual fields. */
  .form-group {
    display: flex;
    flex-direction: column;
    gap: 11px;
  }

  /* ─── Live title-page preview ─── */
  /* The preview pane is the visual reward for the form. Pinned-cork-board
     vibe: a cool gray "desk" with the cream A4 page card floating on it.
     Subtle radial vignette behind the page makes it feel placed rather
     than pasted. */
  .meta-preview {
    background:
      radial-gradient(ellipse 80% 60% at 50% 40%, var(--surface-base), var(--surface-elevated) 80%);
    border-left: 1px solid var(--border-subtle);
    padding: 28px 32px 32px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .preview-eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 10.5px;
    font-weight: 700;
    color: var(--text-muted);
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  /* The mini title page — A4 portrait aspect, faithful to what
     generate_title_page_markup() produces in pdf.rs. */
  .preview-page {
    flex: 1;
    min-height: 0;
    aspect-ratio: 1 / 1.414;     /* A4 portrait */
    max-width: 100%;
    margin: 0 auto;
    align-self: center;
    background: var(--page-bg);
    color: var(--text-on-page);
    background-image: var(--page-grain);
    background-size: 240px 240px;
    border-radius: 3px;
    box-shadow:
      inset 0 1px 0 var(--page-edge-highlight),
      0 1px 2px var(--page-shadow-close),
      0 16px 36px var(--page-shadow);
    padding: 12% 14% 9% 14%;     /* mirrors the Typst 3cm-ish margins */
    display: grid;
    grid-template-rows: 1fr auto;
    position: relative;
    overflow: hidden;
    font-family: var(--editor-font-en), var(--editor-font-ml), ui-monospace, monospace;
  }

  /* ── Centered title + tagline + credits ── */
  .preview-center {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 14px;
    padding-top: 4%;
  }

  .preview-title {
    font-size: clamp(13px, 2vw, 22px);
    font-weight: 700;
    line-height: 1.16;
    letter-spacing: 0.02em;
    word-break: break-word;
    color: var(--text-on-page);
  }

  .preview-title.placeholder {
    color: rgba(0, 0, 0, 0.3);
    font-weight: 500;
    font-style: italic;
  }

  .preview-tagline {
    font-size: clamp(10px, 1.05vw, 12px);
    font-style: italic;
    color: rgba(0, 0, 0, 0.55);
    line-height: 1.4;
    max-width: 80%;
  }

  .preview-credits {
    display: flex;
    flex-direction: column;
    gap: 14px;
    align-items: center;
    margin-top: 18px;
  }

  .preview-credit {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  /* Label small, light, italic — matches Typst's `text(size: 11pt, fill: luma(100))` */
  .preview-credit-label {
    font-size: clamp(9px, 0.85vw, 10.5px);
    color: rgba(0, 0, 0, 0.5);
    font-style: italic;
    letter-spacing: 0.03em;
  }

  /* Name larger and prominent — matches Typst's `text(size: 16pt)` */
  .preview-credit-name {
    font-size: clamp(11px, 1.4vw, 15px);
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  .preview-credit-placeholder {
    margin-top: 12px;
    font-size: 10.5px;
    font-style: italic;
    color: rgba(0, 0, 0, 0.28);
    letter-spacing: 0.04em;
  }

  /* ── Bottom-LEFT block: contact, draft, registration ──
     Matches the Typst `#align(left) #v(1fr)` block — pinned to the
     bottom-left corner, sits on top of the body grid via grid-row.  */
  .preview-bottom-left {
    align-self: end;
    text-align: left;
    color: rgba(0, 0, 0, 0.7);
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: clamp(8.5px, 0.85vw, 10px);
    line-height: 1.45;
  }

  .preview-contact {
    white-space: pre-line;
  }

  .preview-draft {
    color: rgba(0, 0, 0, 0.55);
  }

  .preview-em-dash {
    color: rgba(0, 0, 0, 0.35);
  }

  .preview-reg {
    color: rgba(0, 0, 0, 0.45);
  }

  /* Footnote — italic, centered, very bottom of the page (matches Typst's
     `#place(bottom + center)` for the footnote block). */
  .preview-footnote {
    position: absolute;
    bottom: 6%;
    left: 14%;
    right: 14%;
    text-align: center;
    font-style: italic;
    color: rgba(0, 0, 0, 0.5);
    font-size: clamp(8px, 0.75vw, 9.5px);
    line-height: 1.4;
  }

  .preview-note {
    margin: 0;
    text-align: center;
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
