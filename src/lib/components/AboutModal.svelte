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
      <div class="logo-area">
        <img src="/app-icon.png" alt="Scriptty" class="logo-img" />
      </div>

      <h1 class="app-name">Scriptty</h1>
      {#if version}<p class="version">Version {version}</p>{/if}
      <p class="tagline">Write in the language you dream in.</p>

      <hr class="divider" />

      <div class="credits-section">
        <h3 class="credits-heading">Developers</h3>
        <p class="credits-name">Hrishikesh Bhaskaran (stultus)</p>
        <p class="credits-contact"><a href="mailto:hello@stultus.in" class="credits-link" onclick={(e) => openExternal(e, 'mailto:hello@stultus.in')}>hello@stultus.in</a> &middot; <a href="https://stultus.in" class="credits-link" onclick={(e) => openExternal(e, 'https://stultus.in')}>stultus.in</a></p>
        <p class="credits-name" style="margin-top: 10px;">Hiran Venugopalan</p>
        <p class="credits-contact"><a href="mailto:hiran.v@gmail.com" class="credits-link" onclick={(e) => openExternal(e, 'mailto:hiran.v@gmail.com')}>hiran.v@gmail.com</a> &middot; <a href="https://hiran.in" class="credits-link" onclick={(e) => openExternal(e, 'https://hiran.in')}>hiran.in</a></p>
      </div>

      <div class="credits-section">
        <h3 class="credits-heading">Inputs &amp; Feedback</h3>
        <p class="credits-name">Abraham Joseph</p>
        <p class="credits-name">Aashiq Abu (Filmmaker)</p>
        <p class="credits-name">Sijith Vijayakumar</p>
      </div>

      <hr class="divider" />

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
    padding: var(--modal-padding);
    width: var(--modal-w-base);
    max-width: 90vw;
    box-shadow: var(--modal-shadow);
    animation: modal-in var(--modal-anim-duration) ease-out;
    text-align: center;
    font-family: system-ui, -apple-system, sans-serif;
  }

  @keyframes modal-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }

  .logo-area {
    margin: 0 auto 16px;
  }

  .logo-img {
    width: 80px;
    height: 80px;
  }

  .app-name {
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 4px;
  }

  .version {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0 0 12px;
  }

  .tagline {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  .divider {
    border: none;
    border-top: 1px solid var(--border-subtle);
    margin: 20px 0;
  }

  .credits-section {
    margin-bottom: 16px;
  }

  .credits-heading {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0 0 6px;
  }

  .credits-name {
    font-size: 13px;
    color: var(--text-primary);
    margin: 2px 0;
  }

  .credits-contact {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 2px 0;
  }

  .credits-link {
    color: var(--accent);
    text-decoration: none;
  }

  .credits-link:hover {
    text-decoration: underline;
  }

  .license {
    font-size: 11px;
    color: var(--text-muted);
    margin: 0;
  }

  .modal-footer {
    margin-top: 20px;
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
