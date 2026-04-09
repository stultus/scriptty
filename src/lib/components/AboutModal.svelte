<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let { open = $bindable(false) } = $props<{ open: boolean }>();

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

  function openExternal(event: MouseEvent, url: string) {
    event.preventDefault();
    event.stopPropagation();
    invoke('open_external_url', { url });
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="modal-backdrop" onclick={handleBackdropClick} onkeydown={handleKeydown} role="dialog" aria-modal="true" tabindex="-1">
    <div class="modal-card">
      <div class="logo-area">
        <img src="/app-icon.png" alt="Scriptty" class="logo-img" />
      </div>

      <h1 class="app-name">Scriptty</h1>
      <p class="version">Version 0.5.0</p>
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
        <p class="credits-name">Abraham Joseph (Abrooz)</p>
        <p class="credits-name">Aashiq Abu (Filmmaker)</p>
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
