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
  <!-- Editorial press-release vocabulary, scaled to a 320px toast.
       Centered mh-eyebrow at the top, Fraunces version line with the
       terracotta period that mirrors the brand wordmark, italic display
       deck, then the action pair. Slides up from below — same motion
       as before, the toast is informational not urgent. -->
  <div class="update-toast" role="status" aria-live="polite">
    <div class="mh-eyebrow is-centered" aria-hidden="true">
      <span class="mh-rule"></span>
      <span>An update</span>
      <span class="mh-rule"></span>
    </div>

    <h2 class="update-title">
      Scriptty {updateStore.available.latestVersion}<span class="dot">.</span>
    </h2>

    <p class="update-deck">
      <em>Out now.</em>
      <span class="from">You're on {updateStore.available.currentVersion}.</span>
    </p>

    <div class="update-actions">
      <button type="button" class="later" onclick={later}>Later</button>
      <button type="button" class="primary" onclick={getUpdate}>
        Get update
        <span class="arrow" aria-hidden="true">→</span>
      </button>
    </div>
  </div>
{/if}

<style>
  .update-toast {
    position: fixed;
    bottom: 22px;
    right: 22px;
    z-index: 900;
    width: 320px;
    padding: 18px 22px 16px;
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 10px;
    box-shadow: 0 14px 36px var(--shadow-heavy),
                0 2px 6px var(--shadow-soft);
    animation: update-slide-in var(--motion-slow) cubic-bezier(0.2, 0.7, 0.3, 1);
    text-align: center;
    /* Subtle paper grain — same texture as the screenplay page. Anchors
       the toast in the same physical-paper world the rest of the app
       lives in, so it doesn't read as alien chrome. */
    background-image: var(--page-grain);
    background-repeat: repeat;
    background-size: 240px 240px;
  }

  /* Lighter slide travel (8px) — the toast is informational, not
     urgent. Smaller motion arc reads as a nudge rather than an alert. */
  @keyframes update-slide-in {
    from {
      transform: translateY(10px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  /* mh-eyebrow utility (defined in +layout.svelte) handles the
     tracked-caps + flanking rules. Just space it from the title below. */
  .update-toast :global(.mh-eyebrow) {
    margin-bottom: 10px;
  }

  /* The version line — Fraunces SemiBold, Scriptty wordmark-style with
     the terracotta period. Mirrors the title-bar wordmark composition
     so the user reads "another Scriptty edition" at a glance. */
  .update-title {
    margin: 0;
    font-family: var(--display-font);
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.015em;
    line-height: 1;
    color: var(--text-primary);
  }

  .update-title .dot {
    color: var(--marker-color);
  }

  /* Deck under the version line — display italic for the "Out now" beat,
     muted body for the "you're on X" reference. Read as one breathing
     editorial line. */
  .update-deck {
    margin: 10px 0 16px;
    font-family: var(--display-font);
    font-size: 13.5px;
    line-height: 1.4;
    color: var(--text-muted);
  }

  .update-deck em {
    font-style: italic;
    font-weight: 500;
    color: var(--accent);
    margin-right: 4px;
  }

  .update-deck .from {
    font-style: normal;
  }

  /* Action row — right-aligned per editorial-card convention. */
  .update-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .update-actions button {
    padding: 7px 14px;
    border-radius: 6px;
    font-size: 12.5px;
    font-weight: 500;
    border: 1px solid transparent;
    cursor: pointer;
    font-family: var(--ui-font);
    transition: background-color var(--motion-fast, 120ms) ease,
                color var(--motion-fast, 120ms) ease;
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
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--accent);
    color: var(--text-on-accent);
  }

  .update-actions .primary:hover {
    background: var(--accent-hover);
  }

  /* Right-arrow ornament — subtle motion-forward cue. Slight rightward
     nudge on hover so the action feels eager. */
  .update-actions .primary .arrow {
    display: inline-block;
    transition: transform var(--motion-fast, 120ms) ease;
  }

  .update-actions .primary:hover .arrow {
    transform: translateX(2px);
  }

  .update-actions button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
