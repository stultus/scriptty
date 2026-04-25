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
      <!-- Editorial masthead — same vocabulary as the WelcomeScreen
           and the SceneCardsView hero. Eyebrow + tracked-caps title
           + italic Manjari tagline + asterism divider. -->
      <img src="/app-icon.png" alt="" class="logo-img" aria-hidden="true" />

      <div class="masthead-eyebrow" aria-hidden="true">
        <span class="eyebrow-rule"></span>
        <span class="eyebrow-text">{version ? `Version ${version}` : 'About'}</span>
        <span class="eyebrow-rule"></span>
      </div>

      <h1 class="app-name">SCRIPTTY</h1>
      <p class="tagline">Write in the language you dream in.</p>

      <div class="asterism" aria-hidden="true">· · ·</div>

      <div class="credits-section">
        <h3 class="credits-heading">Developers</h3>
        <p class="credits-name">Hrishikesh Bhaskaran (stultus)</p>
        <p class="credits-contact"><a href="mailto:hello@stultus.in" class="credits-link" onclick={(e) => openExternal(e, 'mailto:hello@stultus.in')}>hello@stultus.in</a> &middot; <a href="https://stultus.in" class="credits-link" onclick={(e) => openExternal(e, 'https://stultus.in')}>stultus.in</a></p>
        <p class="credits-name with-gap">Hiran Venugopalan</p>
        <p class="credits-contact"><a href="mailto:hiran.v@gmail.com" class="credits-link" onclick={(e) => openExternal(e, 'mailto:hiran.v@gmail.com')}>hiran.v@gmail.com</a> &middot; <a href="https://hiran.in" class="credits-link" onclick={(e) => openExternal(e, 'https://hiran.in')}>hiran.in</a></p>
      </div>

      <div class="credits-section">
        <h3 class="credits-heading">Inputs &amp; Feedback</h3>
        <p class="credits-name">Abraham Joseph</p>
        <p class="credits-name">Aashiq Abu (Filmmaker)</p>
        <p class="credits-name">Sijith Vijayakumar</p>
      </div>

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

  /* ─── Editorial masthead — centered frontispiece ─── */
  .masthead-eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 14px;
  }

  .eyebrow-rule {
    display: inline-block;
    width: 28px;
    height: 1px;
    background: var(--border-medium);
  }

  .eyebrow-text {
    font-family: var(--ui-font);
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  /* SCRIPTTY in Courier Prime — the screenplay's own typeface, so
     the wordmark reads as a printed cover-sheet title. Same
     treatment as the Welcome screen. */
  .app-name {
    margin: 0;
    font-family: var(--editor-font-en), ui-monospace, monospace;
    font-size: 36px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-primary);
    line-height: 1;
  }

  .tagline {
    margin: 8px 0 0;
    font-family: 'Manjari', var(--ui-font);
    font-size: 13.5px;
    font-style: italic;
    color: var(--text-secondary);
    line-height: 1.4;
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
    margin-bottom: 18px;
    text-align: left;
    align-self: stretch;
  }

  /* Eyebrow style for credits headings — matches the masthead
     vocabulary so they read as section subheads in the same system. */
  .credits-heading {
    font-family: var(--ui-font);
    font-size: 9.5px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.18em;
    margin: 0 0 8px;
  }

  .credits-name {
    font-family: var(--ui-font);
    font-size: 13px;
    color: var(--text-primary);
    margin: 2px 0;
  }

  .credits-name.with-gap {
    margin-top: 10px;
  }

  .credits-contact {
    font-family: var(--ui-font);
    font-size: 11.5px;
    color: var(--text-secondary);
    margin: 2px 0;
    font-style: italic;
  }

  .credits-link {
    color: var(--accent);
    text-decoration: none;
  }

  .credits-link:hover {
    text-decoration: underline;
  }

  .license {
    margin: 18px 0 0;
    font-family: var(--ui-font);
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--text-muted);
    padding-top: 18px;
    border-top: 1px solid var(--border-subtle);
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
