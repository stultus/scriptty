<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { themeStore } from '$lib/stores/themeStore.svelte';
  import { InputModeManager } from '$lib/editor/input/InputModeManager';

  let { open = $bindable(false), showAnnotations = $bindable(true) } = $props<{
    open: boolean;
    showAnnotations?: boolean;
  }>();

  const inputManager = InputModeManager.getInstance();

  // We sync local state when the modal opens, just in case
  // it was changed via keyboard shortcuts (like Ctrl+Space).
  let currentMode = $state<'ENGLISH' | 'MALAYALAM'>(inputManager.isMalayalam ? 'MALAYALAM' : 'ENGLISH');
  let currentScheme = $state<'inscript1' | 'inscript2' | 'mozhi'>(inputManager.scheme);
  let schemeDropdownOpen = $state(false);

  $effect(() => {
    if (open) {
      currentMode = inputManager.isMalayalam ? 'MALAYALAM' : 'ENGLISH';
      currentScheme = inputManager.scheme;
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

  function setLanguageMode(mode: 'ENGLISH' | 'MALAYALAM') {
    if (mode === 'MALAYALAM' && !inputManager.isMalayalam) {
      inputManager.toggle();
    } else if (mode === 'ENGLISH' && inputManager.isMalayalam) {
      inputManager.toggle();
    }
    currentMode = mode;
  }

  function setScheme(scheme: 'inscript1' | 'inscript2' | 'mozhi') {
    inputManager.setScheme(scheme);
    currentScheme = scheme;
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="modal-backdrop" onclick={handleBackdropClick} onkeydown={handleKeydown} role="dialog" aria-modal="true" tabindex="-1">
    <div class="modal-card">
      <div class="modal-header">
        <h2>Settings</h2>
        <button class="btn-close" onclick={() => { open = false; }} aria-label="Close settings">&times;</button>
      </div>

      <!-- ── Writing ─────────────────────────────────────────── -->
      <div class="section">
        <div class="section-title">Writing</div>

        <div class="setting-row">
          <div class="setting-label">
            <span class="setting-name">Language</span>
            <span class="setting-desc">Toggle with <kbd>⌃Space</kbd></span>
          </div>
          <div class="segmented">
            <button
              class="segmented-item"
              class:active={currentMode === 'ENGLISH'}
              onclick={() => setLanguageMode('ENGLISH')}
            >English</button>
            <button
              class="segmented-item"
              class:active={currentMode === 'MALAYALAM'}
              onclick={() => setLanguageMode('MALAYALAM')}
            >മലയാളം</button>
          </div>
        </div>

        {#if currentMode === 'MALAYALAM'}
          <div class="setting-row">
            <div class="setting-label">
              <span class="setting-name">Keyboard</span>
              <span class="setting-desc">Malayalam input scheme</span>
            </div>

            <div class="custom-select-container">
              <button
                class="scheme-select"
                onclick={() => schemeDropdownOpen = !schemeDropdownOpen}
                aria-haspopup="listbox"
                aria-expanded={schemeDropdownOpen}
              >
                {currentScheme === 'mozhi' ? 'Mozhi Phonetic' : currentScheme === 'inscript2' ? 'Inscript Standard' : 'Inscript Legacy'}
              </button>

              {#if schemeDropdownOpen}
                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                <div class="dropdown-backdrop" role="presentation" onclick={(e) => { e.stopPropagation(); schemeDropdownOpen = false; }} onkeydown={(e) => { if (e.key === 'Escape') schemeDropdownOpen = false; }}></div>
                <div class="custom-options" role="listbox">
                  <button
                    class="custom-option"
                    class:selected={currentScheme === 'mozhi'}
                    onclick={() => { setScheme('mozhi'); schemeDropdownOpen = false; }}
                  >Mozhi Phonetic</button>
                  <button
                    class="custom-option"
                    class:selected={currentScheme === 'inscript2'}
                    onclick={() => { setScheme('inscript2'); schemeDropdownOpen = false; }}
                  >Inscript Standard</button>
                  <button
                    class="custom-option"
                    class:selected={currentScheme === 'inscript1'}
                    onclick={() => { setScheme('inscript1'); schemeDropdownOpen = false; }}
                  >Inscript Legacy</button>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>

      <!-- ── Editor ──────────────────────────────────────────── -->
      <div class="section">
        <div class="section-title">Editor</div>

        <div class="setting-row">
          <div class="setting-label">
            <span class="setting-name">Font</span>
            <span class="setting-desc">Used on screen and in exports</span>
          </div>
          <div class="segmented">
            <button
              class="segmented-item"
              class:active={documentStore.currentFont === 'noto-sans-malayalam'}
              onclick={() => documentStore.setFont('noto-sans-malayalam')}
            >Noto</button>
            <button
              class="segmented-item"
              class:active={documentStore.currentFont === 'manjari'}
              onclick={() => documentStore.setFont('manjari')}
            >Manjari</button>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <span class="setting-name">First scene number</span>
            <span class="setting-desc">Handy when co-writing a range</span>
          </div>
          <input
            class="scene-start-input"
            type="number"
            min="1"
            value={documentStore.document?.settings.scene_number_start ?? 1}
            onchange={(e: Event) => {
              const val = parseInt((e.target as HTMLInputElement).value, 10);
              if (documentStore.document && val >= 1) {
                documentStore.document.settings.scene_number_start = val;
                documentStore.markDirty();
              }
            }}
          />
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <span class="setting-name">Characters under scene heading</span>
            <span class="setting-desc">Auto-list speaking characters</span>
          </div>
          <div class="segmented">
            <button
              class="segmented-item"
              class:active={documentStore.document?.settings.show_characters_below_header === true}
              onclick={() => {
                if (documentStore.document) {
                  documentStore.document.settings.show_characters_below_header = true;
                  documentStore.markDirty();
                }
              }}
            >Show</button>
            <button
              class="segmented-item"
              class:active={!documentStore.document?.settings.show_characters_below_header}
              onclick={() => {
                if (documentStore.document) {
                  documentStore.document.settings.show_characters_below_header = false;
                  documentStore.markDirty();
                }
              }}
            >Hide</button>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <span class="setting-name">Scene annotations</span>
            <span class="setting-desc">Gutter beside the page</span>
          </div>
          <div class="segmented">
            <button
              class="segmented-item"
              class:active={showAnnotations}
              onclick={() => { showAnnotations = true; localStorage.setItem('scriptty-annotations', 'true'); }}
            >Show</button>
            <button
              class="segmented-item"
              class:active={!showAnnotations}
              onclick={() => { showAnnotations = false; localStorage.setItem('scriptty-annotations', 'false'); }}
            >Hide</button>
          </div>
        </div>
      </div>

      <!-- ── Appearance ──────────────────────────────────────── -->
      <div class="section">
        <div class="section-title">Appearance</div>

        <div class="setting-row">
          <div class="setting-label">
            <span class="setting-name">Theme</span>
            <span class="setting-desc">Applies across the app</span>
          </div>
          <div class="segmented">
            <button
              class="segmented-item"
              class:active={!themeStore.isDark}
              onclick={() => { if (themeStore.isDark) themeStore.toggle(); }}
            >Light</button>
            <button
              class="segmented-item"
              class:active={themeStore.isDark}
              onclick={() => { if (!themeStore.isDark) themeStore.toggle(); }}
            >Dark</button>
          </div>
        </div>
      </div>

    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 999;
    /* Invisible backdrop just for capturing outside clicks */
  }

  .modal-card {
    position: absolute;
    /* Clamp both axes to viewport — on short windows the 48px safe area at
       top + 40px above the gear stays respected; on narrow windows the card
       shrinks instead of clipping off-screen. */
    bottom: 40px;
    left: 16px;
    max-width: min(320px, calc(100vw - 32px));
    width: 320px;
    max-height: calc(100vh - 88px);
    background: var(--surface-float);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    padding: 14px 16px 12px;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
    box-shadow: 0 12px 32px var(--shadow-heavy, rgba(0, 0, 0, 0.2)),
                0 2px 8px var(--shadow-soft);
    animation: menu-in 120ms ease-out;
    font-family: system-ui, -apple-system, sans-serif;
    z-index: 1000;
  }

  /* Subtle fade at the bottom of the card hints there is more content
     below when the settings list is taller than the viewport. The
     pseudo-element is positioned fixed to the card's bottom so it stays
     visible while the user scrolls internal content. */
  .modal-card::after {
    content: "";
    position: sticky;
    display: block;
    bottom: -14px;
    left: 0;
    right: 0;
    height: 14px;
    margin: 0 -16px -12px -16px;
    background: linear-gradient(to top, var(--surface-float) 20%, transparent);
    pointer-events: none;
  }

  .modal-card::-webkit-scrollbar {
    width: 8px;
  }

  .modal-card::-webkit-scrollbar-thumb {
    background: var(--border-medium);
    border-radius: 4px;
    border: 2px solid var(--surface-float);
  }

  .modal-card::-webkit-scrollbar-thumb:hover {
    background: var(--text-muted);
  }

  .modal-card::-webkit-scrollbar {
    width: 8px;
  }

  .modal-card::-webkit-scrollbar-thumb {
    background: var(--border-medium);
    border-radius: 4px;
    border: 2px solid var(--surface-float);
  }

  .modal-card::-webkit-scrollbar-thumb:hover {
    background: var(--text-muted);
  }

  @keyframes menu-in {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: -2px -4px 10px -4px;
    padding: 0 4px 10px 4px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 13px;
    color: var(--text-primary);
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .btn-close {
    width: 26px;
    height: 26px;
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
    transition: background 100ms ease, color 100ms ease;
  }

  .btn-close:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .section {
    display: flex;
    flex-direction: column;
  }

  .section + .section {
    margin-top: 6px;
    padding-top: 10px;
    border-top: 1px solid var(--border-subtle);
  }

  .section-title {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    margin: 2px 0 6px 0;
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 6px 0;
  }

  .setting-label {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }

  .setting-name {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1.3;
  }

  .setting-desc {
    font-size: 10.5px;
    color: var(--text-muted);
    line-height: 1.3;
  }

  .setting-desc kbd {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 10px;
    background: var(--surface-base);
    border: 1px solid var(--border-subtle);
    border-radius: 3px;
    padding: 1px 4px;
    color: var(--text-secondary);
  }

  .segmented {
    display: flex;
    background: var(--surface-base);
    border-radius: 6px;
    padding: 2px;
    gap: 1px;
    border: 1px solid var(--border-subtle);
    width: 140px;
    flex-shrink: 0;
  }

  .segmented-item {
    flex: 1;
    text-align: center;
    padding: 4px 0;
    border-radius: 4px;
    border: none;
    font-size: 11px;
    font-weight: 500;
    font-family: system-ui, -apple-system, sans-serif;
    color: var(--text-muted);
    background: transparent;
    cursor: pointer;
    transition: background 100ms, color 100ms;
  }

  .segmented-item:hover {
    color: var(--text-secondary);
  }

  .segmented-item.active {
    background: var(--surface-elevated);
    color: var(--text-primary);
    box-shadow: 0 1px 2px var(--shadow-soft);
  }

  .scene-start-input {
    width: 72px;
    flex-shrink: 0;
    background: var(--surface-base);
    color: var(--text-primary);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    padding: 5px 8px;
    font-size: 12px;
    font-family: inherit;
    text-align: center;
    outline: none;
    transition: border-color 100ms ease;
  }

  .scene-start-input:focus {
    border-color: var(--accent);
  }

  .scheme-select {
    appearance: none;
    background: var(--surface-base) url("data:image/svg+xml;charset=UTF-8,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%23888' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E") no-repeat right 6px center / 12px;
    color: var(--text-primary);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    padding: 4px 24px 4px 8px;
    font-size: 12px;
    font-weight: 500;
    font-family: inherit;
    outline: none;
    cursor: pointer;
    width: 140px;
    text-align: left;
  }

  .custom-select-container {
    position: relative;
  }

  .dropdown-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
  }

  .custom-options {
    position: absolute;
    bottom: calc(100% + 4px);
    right: 0;
    width: 160px;
    background: var(--surface-elevated);
    border: 1px solid var(--border-medium);
    border-radius: 6px;
    box-shadow: 0 4px 16px var(--shadow-soft);
    padding: 4px;
    z-index: 1001;
    display: flex;
    flex-direction: column;
    gap: 2px;
    animation: menu-up 100ms ease-out;
    transform-origin: bottom center;
  }

  @keyframes menu-up {
    from { opacity: 0; transform: translateY(4px) scale(0.98); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }

  .custom-option {
    background: transparent;
    border: none;
    padding: 6px 10px;
    text-align: left;
    font-size: 12px;
    font-family: system-ui, -apple-system, sans-serif;
    color: var(--text-secondary);
    border-radius: 4px;
    cursor: pointer;
    transition: background 100ms, color 100ms;
  }

  .custom-option:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .custom-option.selected {
    background: var(--accent-muted);
    color: var(--accent);
    font-weight: 500;
  }
</style>
