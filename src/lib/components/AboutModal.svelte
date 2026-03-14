<script lang="ts">
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
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="modal-backdrop" onclick={handleBackdropClick} onkeydown={handleKeydown} role="dialog" aria-modal="true" tabindex="-1">
    <div class="modal-card">
      <div class="logo-area">
        <img src="/app-icon.png" alt="Scriptty" class="logo-img" />
      </div>

      <h1 class="app-name">Scriptty</h1>
      <p class="version">Version 0.3.0</p>
      <p class="tagline">Offline screenwriting for Malayalam<br />and English writers</p>

      <hr class="divider" />

      <div class="credits-section">
        <h3 class="credits-heading">Developer</h3>
        <p class="credits-name">Hrishikesh Bhaskaran (stultus)</p>
        <p class="credits-contact">hello@stultus.in &middot; stultus.in</p>
      </div>

      <div class="credits-section">
        <h3 class="credits-heading">Inputs &amp; Feedback</h3>
        <p class="credits-name">Abraham Joseph (Abrooz)</p>
        <p class="credits-name">Aswin Raveendran</p>
        <p class="credits-name">Sanjay Krishna</p>
        <p class="credits-name">Aashiq Abu (director)</p>
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
    background: rgba(0, 0, 0, 0.6);
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
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
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
