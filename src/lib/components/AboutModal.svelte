<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getVersion } from '@tauri-apps/api/app';
  import { onMount } from 'svelte';
  import { focusTrap } from '$lib/actions/focusTrap';

  let { open = $bindable(false) } = $props<{ open: boolean }>();

  let version = $state('');
  onMount(async () => {
    try {
      version = await getVersion();
    } catch {
      version = '';
    }
  });

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

  async function openExternal(event: MouseEvent, url: string) {
    event.preventDefault();
    event.stopPropagation();
    try {
      await invoke('open_external_url', { url });
    } catch (err) {
      // If the OS blocks the open (sandbox rejection, missing xdg-open, etc.)
      // at least leave a breadcrumb in the console instead of looking frozen.
      console.error('Failed to open external URL', url, err);
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="modal-backdrop" onclick={handleBackdropClick} onkeydown={handleKeydown} role="dialog" aria-modal="true" tabindex="-1">
    <div class="modal-card" use:focusTrap>
      <!-- Editorial masthead — eyebrow + display wordmark + italic
           tagline + asterism. Mirrors the magazine-cover hero on
           stultus.in/scriptty so the app's About reads as the same
           publication's colophon. -->
      <img src="/app-icon.png" alt="" class="logo-img" aria-hidden="true" />

      <div class="mh-eyebrow is-centered" aria-hidden="true">
        <span class="mh-rule"></span>
        <span>{version ? `Version ${version}` : 'About'}</span>
        <span class="mh-rule"></span>
      </div>

      <h1 class="app-display">Scriptty<span class="dot">.</span></h1>
      <p class="tagline">Write in the language you <em>dream</em> in.</p>

      <div class="asterism" aria-hidden="true">· · ·</div>

      <div class="credits-section">
        <h3 class="credits-heading">№ 01 · Developers</h3>
        <p class="credits-name">Hrishikesh Bhaskaran <span class="credits-handle">(stultus)</span></p>
        <p class="credits-contact"><a href="mailto:hello@stultus.in" class="credits-link" onclick={(e) => openExternal(e, 'mailto:hello@stultus.in')}>hello@stultus.in</a> &middot; <a href="https://stultus.in" class="credits-link" onclick={(e) => openExternal(e, 'https://stultus.in')}>stultus.in</a></p>
        <p class="credits-name with-gap">Hiran Venugopalan</p>
        <p class="credits-contact"><a href="mailto:hiran.v@gmail.com" class="credits-link" onclick={(e) => openExternal(e, 'mailto:hiran.v@gmail.com')}>hiran.v@gmail.com</a> &middot; <a href="https://hiran.in" class="credits-link" onclick={(e) => openExternal(e, 'https://hiran.in')}>hiran.in</a></p>
        <p class="credits-name with-gap">Abraham Joseph</p>
        <p class="credits-contact"><a href="mailto:dreamingnomad@gmail.com" class="credits-link" onclick={(e) => openExternal(e, 'mailto:dreamingnomad@gmail.com')}>dreamingnomad@gmail.com</a></p>
      </div>

      <div class="credits-section">
        <h3 class="credits-heading">№ 02 · Inputs &amp; Feedback</h3>
        <p class="credits-name">Aashiq Abu <span class="credits-handle">(Filmmaker)</span></p>
        <p class="credits-name">Sijith Vijayakumar</p>
      </div>

      <p class="colophon">Set in <em>Courier Prime</em>, <em>Manjari</em>, <em>Noto Sans Malayalam</em>, and <em>Fraunces</em>.</p>
      <p class="license">MIT License &middot; &copy; 2026 Hrishikesh B.</p>

      <div class="modal-footer">
        <button class="btn-ghost" onclick={() => { open = false; }}>Close</button>
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
    z-index: var(--modal-z);
  }

  .modal-card {
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: var(--modal-radius);
    padding: 36px var(--modal-padding) 28px;
    width: var(--modal-w-base);
    max-width: 90vw;
    box-shadow: var(--modal-shadow);
    animation: modal-in var(--modal-anim-duration) ease-out;
    /* Classical frontispiece: centered masthead (logo + eyebrow +
       title + tagline + asterism), then left-aligned reading
       content (credits sections). */
    text-align: center;
    font-family: var(--ui-font);
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  @keyframes modal-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }

  .logo-img {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    margin-bottom: 18px;
    opacity: 0.92;
  }

  /* mh-eyebrow utility (from +layout.svelte) handles the tracked-caps +
     flanking rules. Just space it from the wordmark below. */
  .modal-card :global(.mh-eyebrow) {
    margin-bottom: 14px;
  }

  /* The display wordmark — Fraunces SemiBold, mixed case, period in
     marker color. Same vocabulary as the title-bar wordmark and the
     marketing-site hero. Scaled larger here because the About modal
     is the ceremonial "cover" surface. */
  .app-display {
    margin: 0;
    font-family: var(--display-font);
    font-size: 44px;
    font-weight: 600;
    letter-spacing: -0.015em;
    color: var(--text-primary);
    line-height: 0.95;
  }

  .app-display .dot {
    color: var(--marker-color);
  }

  /* Tagline now uses the display font in italic so it reads as a
     deck under the wordmark — magazine-issue voice rather than a
     UI subtitle. The emphasised word picks up the accent. */
  .tagline {
    margin: 14px 0 0;
    font-family: var(--display-font);
    font-size: 15.5px;
    font-style: italic;
    font-weight: 400;
    color: var(--text-secondary);
    line-height: 1.4;
    letter-spacing: 0.005em;
  }

  .tagline em {
    font-style: italic;
    color: var(--accent);
    font-weight: 500;
  }

  /* Horizontal masthead break — same vocabulary as the welcome
     and cards-hero. Replaces the prior vertical asterism. */
  .asterism {
    position: relative;
    width: 100%;
    height: 1px;
    margin: 24px 0;
    background: linear-gradient(
      to right,
      transparent 0,
      var(--border-medium) 8%,
      var(--border-medium) 46%,
      transparent 47.5%,
      transparent 52.5%,
      var(--border-medium) 54%,
      var(--border-medium) 92%,
      transparent 100%);
    color: transparent;
    font-size: 0;
  }

  .asterism::before {
    content: '·';
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 14px;
    height: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 14px;
    line-height: 1;
    background: var(--surface-float);
  }

  /* Credits sections — left-aligned reading content beneath the
     centered masthead. Classical magazine layout: display centered,
     body flush-left. */
  .credits-section {
    margin-bottom: 22px;
    text-align: left;
    align-self: stretch;
  }

  .credits-section:last-of-type {
    margin-bottom: 0;
  }

  /* Department-style credits heading — Courier numeral in marker color.
     No hairline rule; the marker-color Courier is loud enough to mark
     a section break on its own, and the asterism above is doing the
     structural-divider work for the whole credits block. */
  .credits-heading {
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 10px;
    font-weight: 700;
    color: var(--marker-color);
    text-transform: uppercase;
    letter-spacing: 0.18em;
    margin: 0 0 10px;
  }

  .credits-name {
    font-family: var(--display-font);
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 6px 0 2px;
    line-height: 1.2;
    letter-spacing: -0.005em;
  }

  .credits-name .credits-handle {
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 10.5px;
    font-weight: 400;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    margin-left: 4px;
    text-transform: uppercase;
  }

  .credits-name.with-gap {
    margin-top: 14px;
  }

  .credits-contact {
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 10.5px;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin: 0 0 4px;
    font-style: normal;
    text-transform: lowercase;
  }

  .credits-link {
    color: var(--text-secondary);
    text-decoration: none;
    transition: color 120ms ease;
  }

  .credits-link:hover {
    color: var(--accent);
    text-decoration: underline;
  }

  /* Colophon — the printer's mark at the back of a magazine, listing
     the typefaces used. Italic display-font, quiet color. No hairline
     rule above; the generous top-margin and shift to italic display
     type already mark the change of voice from credits to colophon. */
  .colophon {
    margin: 32px 0 0;
    font-family: var(--display-font);
    font-size: 12.5px;
    font-style: italic;
    line-height: 1.5;
    color: var(--text-muted);
    align-self: stretch;
    text-align: center;
    letter-spacing: 0.005em;
  }

  .colophon em {
    font-style: italic;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .license {
    margin: 10px 0 0;
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--text-quiet, var(--text-muted));
    align-self: stretch;
    text-align: center;
  }

  .modal-footer {
    margin-top: 18px;
    align-self: stretch;
    display: flex;
    justify-content: center;
  }

  .btn-ghost {
    height: 28px;
    padding: 0 16px;
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
</style>
