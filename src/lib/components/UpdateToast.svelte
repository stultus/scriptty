<script lang="ts">
  import { updateStore } from '$lib/stores/updateStore.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';

  async function getUpdate() {
    try {
      await openUrl('https://stultus.in/scriptty');
    } catch (err) {
      console.error('Could not open download page', err);
    }
    updateStore.dismiss();
  }

  function later() {
    updateStore.dismiss();
  }
</script>

{#if updateStore.available}
  <div class="update-toast" role="status" aria-live="polite">
    <div class="update-body">
      <span class="update-icon" aria-hidden="true">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
        </svg>
      </span>
      <div class="update-text">
        <strong>Scriptty {updateStore.available.latestVersion} is available</strong>
        <span class="update-sub">You're on {updateStore.available.currentVersion}.</span>
      </div>
    </div>
    <div class="update-actions">
      <button type="button" class="later" onclick={later}>Later</button>
      <button type="button" class="primary" onclick={getUpdate}>Get update</button>
    </div>
  </div>
{/if}

<style>
  .update-toast {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 900;
    width: 320px;
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 10px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.28);
    padding: 14px 16px;
    animation: update-slide-in 220ms cubic-bezier(0.2, 0.7, 0.3, 1);
    font-family: inherit;
  }

  @keyframes update-slide-in {
    from {
      transform: translateY(14px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  .update-body {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    margin-bottom: 12px;
  }

  .update-icon {
    color: var(--accent);
    flex-shrink: 0;
    margin-top: 1px;
  }

  .update-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .update-text strong {
    font-size: 13.5px;
    color: var(--text-primary);
    font-weight: 600;
    letter-spacing: -0.005em;
  }

  .update-sub {
    font-size: 12px;
    color: var(--text-muted);
  }

  .update-actions {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
  }

  .update-actions button {
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 12.5px;
    font-weight: 500;
    border: 1px solid transparent;
    cursor: pointer;
    font-family: inherit;
    transition: background-color 120ms, color 120ms;
  }

  .update-actions .later {
    background: transparent;
    color: var(--text-secondary);
  }

  .update-actions .later:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .update-actions .primary {
    background: var(--accent);
    color: #ffffff;
  }

  .update-actions .primary:hover {
    background: var(--accent-hover);
  }

  .update-actions button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
