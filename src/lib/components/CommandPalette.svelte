<script lang="ts">
  import { focusTrap } from '$lib/actions/focusTrap';
  import { tick } from 'svelte';

  // The command palette is the single entry point for infrequent actions —
  // file ops, view toggles, settings. Actions live here as data so they
  // can be filtered by label or keywords; the parent wires each callback
  // to the same handler the keyboard shortcut or menu event already uses.

  export interface Command {
    id: string;
    label: string;
    group: string;
    hint?: string;
    keywords?: string;
    action: () => void;
  }

  let {
    open = $bindable(false),
    commands,
  }: {
    open: boolean;
    commands: Command[];
  } = $props();

  let query = $state('');
  let selectedIdx = $state(0);
  let inputEl: HTMLInputElement | undefined = $state();
  let listEl: HTMLDivElement | undefined = $state();

  // Track the last mouse position so we can distinguish real cursor motion
  // from pseudo-hover events that fire when the list scrolls under a
  // stationary cursor. Without this, pressing ArrowDown would scroll the
  // list, trigger mouseenter on whichever row slid under the cursor, and
  // snap the selection back — fighting the keyboard.
  let lastPointer = { x: -1, y: -1 };

  let filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return commands;
    return commands.filter((c) => {
      const hay = `${c.label} ${c.group} ${c.keywords ?? ''}`.toLowerCase();
      return q.split(/\s+/).every((term) => hay.includes(term));
    });
  });

  // Group consecutive commands from the same group so the list reads as
  // tidy sections — but only when there's no active filter (a filtered
  // list is small and the grouping just adds visual noise).
  let grouped = $derived.by<Array<{ group: string; items: Command[] }>>(() => {
    if (query.trim()) return [{ group: '', items: filtered }];
    const out: Array<{ group: string; items: Command[] }> = [];
    for (const cmd of filtered) {
      const last = out[out.length - 1];
      if (last && last.group === cmd.group) {
        last.items.push(cmd);
      } else {
        out.push({ group: cmd.group, items: [cmd] });
      }
    }
    return out;
  });

  $effect(() => {
    // Reset state when the palette opens so each invocation feels fresh.
    if (open) {
      query = '';
      selectedIdx = 0;
      tick().then(() => inputEl?.focus());
    }
  });

  // Clamp the selection whenever the filtered list shrinks, otherwise the
  // highlighted row can point past the end and Enter becomes a no-op.
  $effect(() => {
    if (selectedIdx >= filtered.length) selectedIdx = Math.max(0, filtered.length - 1);
  });

  function run(cmd: Command) {
    open = false;
    // Defer the action so the palette's focus-trap can restore focus to
    // whatever was focused before — otherwise actions that open another
    // modal race against the restore and the editor steals focus back.
    queueMicrotask(() => cmd.action());
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      open = false;
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (filtered.length === 0) return;
      selectedIdx = (selectedIdx + 1) % filtered.length;
      scrollSelectedIntoView();
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (filtered.length === 0) return;
      selectedIdx = (selectedIdx - 1 + filtered.length) % filtered.length;
      scrollSelectedIntoView();
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      const cmd = filtered[selectedIdx];
      if (cmd) run(cmd);
    }
  }

  function scrollSelectedIntoView() {
    tick().then(() => {
      const el = listEl?.querySelector<HTMLButtonElement>(`[data-idx="${selectedIdx}"]`);
      el?.scrollIntoView({ block: 'nearest' });
    });
  }
</script>

{#if open}
  <button
    type="button"
    class="backdrop"
    onclick={() => { open = false; }}
    aria-label="Close command palette"
  ></button>
  <div
    class="palette"
    role="dialog"
    aria-modal="true"
    aria-label="Command palette"
    tabindex="-1"
    use:focusTrap
    onkeydown={handleKeydown}
  >
    <input
      bind:this={inputEl}
      bind:value={query}
      type="text"
      class="search"
      placeholder="Type a command…"
      aria-label="Search commands"
      autocomplete="off"
      spellcheck="false"
    />
    <div class="list" bind:this={listEl} role="listbox">
      {#if filtered.length === 0}
        <div class="empty">No matching commands</div>
      {:else}
        {#each grouped as section (section.group)}
          {#if section.group}
            <div class="group-label">{section.group}</div>
          {/if}
          {#each section.items as cmd (cmd.id)}
            {@const idx = filtered.indexOf(cmd)}
            <button
              type="button"
              class="item"
              class:selected={idx === selectedIdx}
              data-idx={idx}
              onclick={() => run(cmd)}
              onmousemove={(e) => {
                if (e.clientX === lastPointer.x && e.clientY === lastPointer.y) return;
                lastPointer = { x: e.clientX, y: e.clientY };
                selectedIdx = idx;
              }}
              role="option"
              aria-selected={idx === selectedIdx}
            >
              <span class="label">{cmd.label}</span>
              {#if cmd.hint}
                <span class="hint">{cmd.hint}</span>
              {/if}
            </button>
          {/each}
        {/each}
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--backdrop, rgba(0, 0, 0, 0.38));
    backdrop-filter: blur(2px);
    z-index: 400;
    border: none;
    padding: 0;
    cursor: default;
  }

  /* Float high on the screen so the search input sits in the reader's
     natural sightline — a centered palette feels sluggish here. */
  .palette {
    position: fixed;
    top: 18vh;
    left: 50%;
    transform: translateX(-50%);
    width: min(560px, calc(100vw - 48px));
    max-height: 64vh;
    display: flex;
    flex-direction: column;
    background: var(--surface-elevated);
    border: 1px solid var(--border-medium);
    border-radius: 10px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35);
    z-index: 401;
    overflow: hidden;
    font-family: var(--ui-font);
  }

  .search {
    flex-shrink: 0;
    height: 48px;
    padding: 0 18px;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-primary);
    font-family: var(--ui-font);
    font-size: 15px;
    outline: none;
  }

  .search::placeholder {
    color: var(--text-muted);
  }

  .list {
    flex: 1;
    overflow-y: auto;
    padding: 6px 0;
  }

  .group-label {
    padding: 10px 18px 4px;
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    font-weight: 600;
  }

  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    padding: 8px 18px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-family: var(--ui-font);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    transition: background var(--motion-fast) var(--motion-easing),
                color var(--motion-fast) var(--motion-easing);
  }

  .item.selected {
    background: var(--accent-muted);
    color: var(--text-primary);
  }

  .label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hint {
    flex-shrink: 0;
    color: var(--text-muted);
    font-size: 11px;
    letter-spacing: 0.04em;
    font-variant-numeric: tabular-nums;
  }

  .item.selected .hint {
    color: var(--text-secondary);
  }

  .empty {
    padding: 24px 18px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }
</style>
