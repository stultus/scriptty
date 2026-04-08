<script lang="ts">
  import { documentStore } from '$lib/stores/documentStore.svelte';
  import { themeStore } from '$lib/stores/themeStore.svelte';
  import { InputModeManager } from '$lib/editor/input/InputModeManager';

  let { open = $bindable(false) } = $props<{ open: boolean }>();

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
        <button class="btn-close" onclick={() => { open = false; }}>&times;</button>
      </div>

      <div class="setting-row">
        <div class="setting-name-group">
          <span class="setting-name">Language</span>
          <span class="setting-hint">⌃Space</span>
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
        <div class="setting-row nested">
          <span class="setting-name">Keyboard</span>
          
          <div class="custom-select-container">
            <button 
              class="scheme-select" 
              onclick={() => schemeDropdownOpen = !schemeDropdownOpen}
            >
              {currentScheme === 'mozhi' ? 'Mozhi Phonetic' : currentScheme === 'inscript2' ? 'Inscript Standard' : 'Inscript Legacy'}
            </button>

            {#if schemeDropdownOpen}
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <div class="dropdown-backdrop" role="presentation" onclick={(e) => { e.stopPropagation(); schemeDropdownOpen = false; }} onkeydown={(e) => { if (e.key === 'Escape') schemeDropdownOpen = false; }}></div>
              <div class="custom-options">
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

      <div class="setting-row">
        <span class="setting-name">Editor Font</span>
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
        <span class="setting-name">Scene Start #</span>
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
        <span class="setting-name">Theme</span>
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
    bottom: 36px;
    left: 16px;
    background: var(--surface-float);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 6px 14px;
    width: 250px;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2), 0 1px 4px rgba(0, 0, 0, 0.1);
    animation: menu-in 100ms ease-out;
    font-family: system-ui, -apple-system, sans-serif;
    z-index: 1000;
  }

  @keyframes menu-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 13px;
    color: var(--text-primary);
    font-weight: 600;
  }

  .btn-close {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
  }

  .btn-close:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 0;
  }

  .setting-row.nested {
    padding-left: 12px;
  }

  .setting-name-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .setting-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .setting-hint {
    font-size: 10px;
    color: var(--text-muted);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    background: var(--surface-base);
    padding: 2px 4px;
    border-radius: 3px;
    border: 1px solid var(--border-subtle);
  }

  .segmented {
    display: flex;
    background: var(--surface-base);
    border-radius: 6px;
    padding: 2px;
    gap: 1px;
    border: 1px solid var(--border-subtle);
    width: 140px;
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
    box-shadow: 0 1px 2px rgba(0,0,0,0.1);
  }

  .scene-start-input {
    width: 60px;
    background: var(--surface-base);
    color: var(--text-primary);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 12px;
    font-family: inherit;
    text-align: center;
    outline: none;
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
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
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
