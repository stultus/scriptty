<script lang="ts">
  // Custom date picker — replaces the native <input type="date"> in places
  // where the OS popup is heavy / inconsistent across Tauri WebViews and the
  // surrounding chrome wants to feel of-a-piece with the app. Stores ISO
  // strings (YYYY-MM-DD) so the rest of the app sees the same value shape
  // as before, but the writer interacts with a calendar grid instead of
  // OS-default segmented digits.

  let { value = $bindable(''), placeholder = 'Pick a date', onChange } = $props<{
    value: string;
    placeholder?: string;
    /** Optional callback fired when the value changes — useful when the
     *  caller can't pass a bindable (e.g. a date stored on a derived
     *  array element). Receives the new ISO string (or '' for clear). */
    onChange?: (v: string) => void;
  }>();

  let open = $state(false);
  let triggerEl = $state<HTMLButtonElement | null>(null);
  let popoverEl = $state<HTMLDivElement | null>(null);

  /** Open direction — 'down' (default) or 'up' if there isn't room below
   *  the trigger for the calendar. Flipped on open before the popover paints
   *  so the writer never sees a misplaced calendar. */
  let direction = $state<'down' | 'up'>('down');
  /** Approx popover height — used for the flip decision. The actual rendered
   *  height varies (footer changes when a date is set), so we use a generous
   *  upper bound so a near-edge placement still flips correctly. */
  const POPOVER_HEIGHT_GUESS = 320;
  const POPOVER_GAP = 6;

  // The calendar's "viewing" month — independent of `value` so the writer
  // can flip months without committing a date. Seeds from the value if set,
  // today otherwise.
  function initialView(): { year: number; month: number } {
    const seed = parseISO(value) ?? new Date();
    return { year: seed.getFullYear(), month: seed.getMonth() };
  }

  let view = $state(initialView());
  let valueDate = $derived(parseISO(value));

  // Re-seed view when the modal re-opens with a different value (the parent
  // form re-binds when the writer switches episodes etc.). Also decide which
  // direction to open in based on viewport space — we flip the calendar
  // upward when there isn't enough room below the trigger so it never lands
  // off-screen (writer otherwise has to scroll the modal to see it).
  $effect(() => {
    if (!open) return;
    view = initialView();
    if (triggerEl) {
      const r = triggerEl.getBoundingClientRect();
      const spaceBelow = window.innerHeight - r.bottom;
      const spaceAbove = r.top;
      // Flip up if below doesn't fit AND above has more room. Default to
      // down on a tie so the position is predictable.
      direction = spaceBelow < POPOVER_HEIGHT_GUESS + POPOVER_GAP && spaceAbove > spaceBelow
        ? 'up'
        : 'down';
    } else {
      direction = 'down';
    }
  });

  function parseISO(s: string): Date | null {
    if (!s) return null;
    // Match YYYY-MM-DD only — ignores the time component if present.
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(s);
    if (!m) return null;
    const d = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
    return Number.isNaN(d.getTime()) ? null : d;
  }

  function toISO(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const dd = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${dd}`;
  }

  function formatHuman(d: Date): string {
    return d.toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' });
  }

  let displayValue = $derived(valueDate ? formatHuman(valueDate) : '');

  /** Build the 6×7 grid for the current view. Always 42 cells so the popover
   *  doesn't reflow as the user pages months. */
  let grid = $derived.by<Array<{ date: Date; inMonth: boolean }>>(() => {
    const first = new Date(view.year, view.month, 1);
    // Sunday = 0; we render Sun-first so a US screenwriter doesn't trip,
    // and Indian writers don't either (Mon-first feels less universal here).
    const startOffset = first.getDay();
    const cells: Array<{ date: Date; inMonth: boolean }> = [];
    for (let i = 0; i < 42; i++) {
      const d = new Date(view.year, view.month, 1 - startOffset + i);
      cells.push({ date: d, inMonth: d.getMonth() === view.month });
    }
    return cells;
  });

  let monthLabel = $derived(
    new Date(view.year, view.month, 1).toLocaleDateString(undefined, {
      month: 'long',
      year: 'numeric',
    }),
  );

  const today = new Date();
  function isSameDay(a: Date, b: Date): boolean {
    return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
  }

  function pick(d: Date) {
    const next = toISO(d);
    value = next;
    onChange?.(next);
    open = false;
    queueMicrotask(() => triggerEl?.focus());
  }

  function clear() {
    value = '';
    onChange?.('');
    open = false;
    queueMicrotask(() => triggerEl?.focus());
  }

  function pickToday() {
    pick(new Date());
  }

  function prevMonth() {
    const m = view.month - 1;
    if (m < 0) view = { year: view.year - 1, month: 11 };
    else view = { ...view, month: m };
  }

  function nextMonth() {
    const m = view.month + 1;
    if (m > 11) view = { year: view.year + 1, month: 0 };
    else view = { ...view, month: m };
  }

  function handleTriggerKey(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      open = !open;
    } else if (event.key === 'Escape') {
      open = false;
    }
  }

  function handlePopoverKey(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      open = false;
      queueMicrotask(() => triggerEl?.focus());
    }
  }

  // Close on outside click.
  $effect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      const target = e.target as Node;
      if (popoverEl?.contains(target) || triggerEl?.contains(target)) return;
      open = false;
    };
    document.addEventListener('mousedown', onClick);
    return () => document.removeEventListener('mousedown', onClick);
  });

  const weekdays = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'];
</script>

<div class="date-picker">
  <button
    type="button"
    class="dp-trigger"
    class:placeholder={!displayValue}
    bind:this={triggerEl}
    onclick={() => { open = !open; }}
    onkeydown={handleTriggerKey}
    aria-haspopup="dialog"
    aria-expanded={open}
  >
    <svg class="dp-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <rect x="3" y="5" width="18" height="16" rx="2"/>
      <line x1="3" y1="10" x2="21" y2="10"/>
      <line x1="8" y1="3" x2="8" y2="7"/>
      <line x1="16" y1="3" x2="16" y2="7"/>
    </svg>
    <span class="dp-text">{displayValue || placeholder}</span>
    {#if displayValue}
      <!-- Inline clear glyph. Lives inside the trigger button, so we use a
           span with click handling rather than a nested <button> (invalid
           HTML). Clicks are stopped from bubbling into the trigger's
           open/close behavior; the popover's "Clear" link covers the
           keyboard path. -->
      <span
        class="dp-clear"
        role="presentation"
        onclick={(e) => { e.stopPropagation(); clear(); }}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); clear(); } }}
      >×</span>
    {/if}
  </button>

  {#if open}
    <div
      class="dp-popover"
      class:up={direction === 'up'}
      role="dialog"
      aria-label="Pick a date"
      bind:this={popoverEl}
      onkeydown={handlePopoverKey}
      tabindex="-1"
    >
      <div class="dp-head">
        <button type="button" class="dp-nav" onclick={prevMonth} aria-label="Previous month">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M15 18 L9 12 L15 6"/></svg>
        </button>
        <div class="dp-month">{monthLabel}</div>
        <button type="button" class="dp-nav" onclick={nextMonth} aria-label="Next month">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M9 6 L15 12 L9 18"/></svg>
        </button>
      </div>

      <div class="dp-weekdays" aria-hidden="true">
        {#each weekdays as wd}
          <span class="dp-wd">{wd}</span>
        {/each}
      </div>

      <div class="dp-grid" role="grid">
        {#each grid as cell}
          {@const isToday = isSameDay(cell.date, today)}
          {@const isSelected = valueDate ? isSameDay(cell.date, valueDate) : false}
          <button
            type="button"
            class="dp-cell"
            class:out-of-month={!cell.inMonth}
            class:today={isToday}
            class:selected={isSelected}
            onclick={() => pick(cell.date)}
            aria-label={cell.date.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' })}
          >
            {cell.date.getDate()}
          </button>
        {/each}
      </div>

      <div class="dp-footer">
        <button type="button" class="dp-pill" onclick={pickToday}>Today</button>
        <div class="dp-spacer"></div>
        {#if displayValue}
          <button type="button" class="dp-link" onclick={clear}>Clear</button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .date-picker {
    position: relative;
  }

  .dp-trigger {
    width: 100%;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 9px 11px;
    font-size: 12.5px;
    color: var(--text-primary);
    background: var(--surface-base);
    border: 1px solid var(--border-medium);
    border-radius: 7px;
    font-family: var(--ui-font);
    cursor: pointer;
    transition: border-color var(--motion-fast, 100ms) ease,
                background var(--motion-fast, 100ms) ease;
  }

  .dp-trigger:hover {
    border-color: var(--text-muted);
  }

  .dp-trigger[aria-expanded='true'] {
    border-color: var(--accent);
    background: var(--surface-float);
  }

  .dp-icon {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .dp-text {
    flex: 1;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .dp-trigger.placeholder .dp-text {
    color: var(--text-muted);
  }

  .dp-clear {
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: transparent;
    color: var(--text-muted);
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease, color var(--motion-fast, 100ms) ease;
  }

  .dp-clear:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  /* ─── Popover ─── */
  /* Default opens DOWN (top: 100% + gap). When the trigger sits low in the
     viewport, `.up` flips the popover above the trigger so it can't land
     off-screen — the writer otherwise had to scroll the modal to see the
     calendar (the original bug from the screenshot). */
  .dp-popover {
    position: absolute;
    z-index: 20;
    top: calc(100% + 6px);
    left: 0;
    width: 264px;
    padding: 12px;
    background: var(--surface-float);
    border: 1px solid var(--border-medium);
    border-radius: 10px;
    box-shadow:
      0 12px 32px var(--shadow-heavy),
      0 2px 8px var(--shadow-soft);
    animation: dp-in 120ms ease-out;
    font-family: var(--ui-font);
  }

  .dp-popover.up {
    top: auto;
    bottom: calc(100% + 6px);
    animation: dp-in-up 120ms ease-out;
  }

  @keyframes dp-in {
    from { opacity: 0; transform: translateY(-4px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  @keyframes dp-in-up {
    from { opacity: 0; transform: translateY(4px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .dp-head {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 10px;
  }

  .dp-month {
    flex: 1;
    text-align: center;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.02em;
  }

  .dp-nav {
    width: 26px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease, color var(--motion-fast, 100ms) ease;
  }

  .dp-nav:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .dp-weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
    margin-bottom: 6px;
  }

  .dp-wd {
    text-align: center;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .dp-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
  }

  .dp-cell {
    aspect-ratio: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--ui-font);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease,
                color var(--motion-fast, 100ms) ease;
  }

  .dp-cell:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .dp-cell.out-of-month {
    color: var(--text-muted);
    opacity: 0.45;
  }

  .dp-cell.today {
    color: var(--accent);
    font-weight: 700;
    box-shadow: inset 0 0 0 1px var(--accent-muted);
  }

  .dp-cell.selected {
    background: var(--accent);
    color: var(--text-on-accent);
    font-weight: 600;
    box-shadow: none;
  }

  .dp-cell.selected:hover {
    background: var(--accent-hover);
    color: var(--text-on-accent);
  }

  .dp-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--border-subtle);
  }

  .dp-spacer {
    flex: 1;
  }

  .dp-pill {
    height: 24px;
    padding: 0 10px;
    border-radius: 12px;
    border: 1px solid var(--border-medium);
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--ui-font);
    font-size: 11px;
    cursor: pointer;
    transition: background var(--motion-fast, 100ms) ease, color var(--motion-fast, 100ms) ease;
  }

  .dp-pill:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .dp-link {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-family: var(--ui-font);
    font-size: 11px;
    cursor: pointer;
    padding: 4px 6px;
  }

  .dp-link:hover {
    color: var(--accent);
  }
</style>
