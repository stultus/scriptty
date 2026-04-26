<script lang="ts">
  import { focusTrap } from '$lib/actions/focusTrap';
  import { getVersion } from '@tauri-apps/api/app';
  import { onMount } from 'svelte';

  let { open = $bindable(false), onShowAbout }: {
    open: boolean;
    onShowAbout?: () => void;
  } = $props();

  let version = $state('');
  onMount(async () => {
    try {
      version = await getVersion();
    } catch {
      version = '';
    }
  });

  // TOC entries. Order matches the on-screen order of sections in the
  // right pane, and drives both the sidebar links and the active-section
  // highlight (via IntersectionObserver below).
  const sections = [
    { id: 'getting-started', title: 'Getting Started' },
    { id: 'series', title: 'Web Series' },
    { id: 'writing', title: 'Writing in the Editor' },
    { id: 'paste-to-script', title: 'Paste to Screenplay' },
    { id: 'malayalam', title: 'Malayalam Input' },
    { id: 'autocomplete', title: 'Character Autocomplete' },
    { id: 'formatting', title: 'Text Formatting' },
    { id: 'find', title: 'Find and Replace' },
    { id: 'scene-nav', title: 'Scene Navigator' },
    { id: 'story-view', title: 'Story View' },
    { id: 'views', title: 'Views' },
    { id: 'annotations', title: 'Scene Annotations' },
    { id: 'cards', title: 'Scene Cards' },
    { id: 'production', title: 'Production Planning' },
    { id: 'stats', title: 'Script Statistics' },
    { id: 'export', title: 'Exporting to PDF' },
    { id: 'fonts', title: 'Fonts' },
    { id: 'saving', title: 'Saving & Autosave' },
    { id: 'shortcuts', title: 'Keyboard Shortcuts' },
  ];

  let activeId = $state(sections[0].id);
  let contentEl: HTMLDivElement | undefined = $state();

  function scrollToSection(id: string) {
    const el = contentEl?.querySelector(`#${id}`) as HTMLElement | null;
    if (!el || !contentEl) return;
    // offsetTop is relative to the nearest positioned ancestor, which may
    // not be the scroll container — compute the delta via bounding rects
    // so the section top lands at the top of the content pane.
    const top =
      el.getBoundingClientRect().top -
      contentEl.getBoundingClientRect().top +
      contentEl.scrollTop -
      12;
    contentEl.scrollTo({ top, behavior: 'smooth' });
    activeId = id;
  }

  // Track which section is currently the topmost visible one so the
  // sidebar link stays in sync as the user scrolls the content pane.
  $effect(() => {
    if (!open || !contentEl) return;
    const observer = new IntersectionObserver(
      (entries) => {
        // Pick the first entry that is intersecting — because rootMargin
        // pushes the top of the observation band down, the "active"
        // section is naturally the one just crossing into view.
        const visible = entries
          .filter((e) => e.isIntersecting)
          .sort((a, b) => a.target.getBoundingClientRect().top - b.target.getBoundingClientRect().top);
        if (visible.length > 0) {
          activeId = (visible[0].target as HTMLElement).id;
        }
      },
      { root: contentEl, rootMargin: '0px 0px -70% 0px', threshold: 0 }
    );
    contentEl.querySelectorAll<HTMLElement>('.help-section').forEach((s) => observer.observe(s));
    return () => observer.disconnect();
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

  function handleAboutClick() {
    open = false;
    onShowAbout?.();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="modal-backdrop" onclick={handleBackdropClick} onkeydown={handleKeydown} role="dialog" aria-modal="true" aria-labelledby="help-title" tabindex="-1">
    <div class="modal-card" use:focusTrap>
      <div class="modal-header">
        <div class="modal-header-text">
          <div class="mh-eyebrow" aria-hidden="true">
            <span class="mh-rule"></span>
            <span>The Manual</span>
            <span class="mh-rule"></span>
          </div>
          <h2 id="help-title" class="mh-title help-title">How to Use Scriptty</h2>
        </div>
        <button class="btn-close" onclick={() => { open = false; }} aria-label="Close help">&times;</button>
      </div>

      <div class="modal-body">
        <!-- Sidebar TOC: fixed width, scrolls independently of content. -->
        <nav class="help-toc" aria-label="Help sections">
          <ul class="toc-list">
            {#each sections as section (section.id)}
              <li>
                <button
                  class="toc-link"
                  class:active={activeId === section.id}
                  onclick={() => scrollToSection(section.id)}
                >{section.title}</button>
              </li>
            {/each}
          </ul>

          <div class="toc-footer">
            <button class="about-link" onclick={handleAboutClick}>About Scriptty</button>
            {#if version}<span class="toc-footer-version">v{version}</span>{/if}
          </div>
        </nav>

        <!-- Scrolling content pane. Each section has an id matching the TOC. -->
        <div class="help-content" bind:this={contentEl}>

          <section class="help-section" id="getting-started">
            <h3>Getting Started</h3>
            <p>Scriptty is an offline screenwriting app for Malayalam and English writers. Your work is saved locally as <code>.screenplay</code> files — no cloud, no account needed.</p>
            <ol>
              <li>On the Welcome screen, pick <strong>New Film</strong> for a single screenplay or <strong>New Series</strong> for a multi-episode project. You can also trigger these later with <kbd>Cmd+N</kbd> (film) and <kbd>Cmd+Shift+N</kbd> (series).</li>
              <li>Go to <strong>File → Metadata</strong> to set your title, writer, director, draft info, and title-page extras (tagline, registration number, footnote).</li>
              <li>Start writing — the editor begins in <em>Action</em> mode. Press <kbd>Tab</kbd> to switch to a Character name, then <kbd>Enter</kbd> to write Dialogue.</li>
              <li>Use <kbd>Cmd+B</kbd>, <kbd>Cmd+I</kbd>, <kbd>Cmd+U</kbd> or the <strong>B I U</strong> buttons in the toolbar for bold, italic, and underline formatting.</li>
              <li>Press <kbd>Cmd+S</kbd> to save your work.</li>
            </ol>
          </section>

          <section class="help-section" id="series">
            <h3>Web Series</h3>
            <p>Scriptty supports multi-episode projects in a single <code>.screenplay</code> file. Each episode has its own screenplay content, metadata, story sections, and scene cards — but shares one series title, font, and input settings.</p>
            <ol>
              <li>Pick <strong>New Series</strong> on the Welcome screen (or press <kbd>Cmd+Shift+N</kbd>). Enter a series title to get started — Scriptty creates Episode 1 for you.</li>
              <li>Open the Scene Navigator (<kbd>Cmd+\</kbd>). Series projects show an <strong>Episodes</strong> list above the scenes. Click an episode to switch; use the <strong>+</strong> control to add new episodes.</li>
              <li>Open <strong>File → Metadata</strong> while an episode is active to edit that episode's title, credits, and draft info. The <strong>Episode Title</strong> field is the short label shown in the navigator and on the per-episode header in combined exports.</li>
              <li>Each episode has its own Story view, Scene Cards, and annotations — switching episodes switches every view in the app.</li>
            </ol>
            <p><strong>Exporting a series:</strong> in the Export modal, pick a scope — <strong>Active episode</strong> (just the one you're editing) or <strong>Entire series</strong>. Series exports start each episode on a new page with a centered <em>EPISODE N: Title</em> header, and restart scene numbering from 1 for each episode.</p>
            <p class="hint">Renaming an episode: click the episode in the Scene Navigator to activate it, then edit <strong>Episode Title</strong> in the Metadata modal.</p>
          </section>

          <section class="help-section" id="writing">
            <h3>Writing in the Editor</h3>
            <p>The editor uses Hollywood single-column format. There are six element types, and you move between them with <kbd>Tab</kbd> and <kbd>Enter</kbd>:</p>

            <table class="shortcut-table">
              <thead>
                <tr><th>You're in</th><th>Key</th><th>Creates</th></tr>
              </thead>
              <tbody>
                <tr><td>Scene Heading</td><td><kbd>Enter</kbd></td><td>Action</td></tr>
                <tr><td>Action</td><td><kbd>Enter</kbd></td><td>Action (new paragraph)</td></tr>
                <tr><td>Action</td><td><kbd>Tab</kbd></td><td>Character</td></tr>
                <tr><td>Character</td><td><kbd>Enter</kbd></td><td>Dialogue</td></tr>
                <tr><td>Dialogue</td><td><kbd>Enter</kbd></td><td>Character</td></tr>
                <tr><td>Dialogue</td><td><kbd>Tab</kbd></td><td>Parenthetical</td></tr>
                <tr><td>Parenthetical</td><td><kbd>Enter</kbd></td><td>Dialogue</td></tr>
                <tr><td>Parenthetical</td><td><kbd>Tab</kbd></td><td>Character</td></tr>
                <tr><td>Any element</td><td><kbd>Cmd+Shift+T</kbd></td><td>Transition</td></tr>
                <tr><td>Transition</td><td><kbd>Enter</kbd></td><td>Action</td></tr>
              </tbody>
            </table>

            <p><kbd>Shift+Enter</kbd> creates a new Scene Heading from anywhere.<br />
            <kbd>Shift+Tab</kbd> converts the current element back to Action (or Scene Heading if already Action, Dialogue if in Parenthetical).</p>

            <p><strong>Jump to element type:</strong> <kbd>Cmd+1</kbd> Scene Heading · <kbd>Cmd+2</kbd> Action · <kbd>Cmd+3</kbd> Character · <kbd>Cmd+4</kbd> Dialogue. Converts the current block in place.</p>
            <p class="hint">On Linux, if your desktop already uses <kbd>Ctrl+1..4</kbd> for workspace switching, the same actions are also bound to <kbd>Ctrl+Alt+1..4</kbd>.</p>

            <p><strong>Dialogue loop:</strong> Character → Dialogue → Character flows continuously. Press <kbd>Enter</kbd> on an empty Character to break out to Action. Press <kbd>Enter</kbd> on an empty Dialogue to return to the Character above.</p>
            <p><strong>Paragraph splitting:</strong> Press <kbd>Enter</kbd> in the middle of an Action element to split it into two paragraphs.</p>
            <p><strong>Transitions:</strong> Use <kbd>Cmd+Shift+T</kbd> to turn the current block into a Transition ("CUT TO:", "SMASH CUT TO:", "MATCH CUT:"). Pressing <kbd>Enter</kbd> after a Transition continues with Action — useful for mid-scene transitions and montage beats. Press <kbd>Shift+Enter</kbd> instead when the Transition ends the scene.</p>
            <p>Scene headings and character names are automatically uppercased as you type.</p>
            <p>Parentheticals are automatically wrapped in parentheses — just type the direction (e.g., "whispering") and the app adds <strong>(</strong> and <strong>)</strong> for you.</p>
          </section>

          <section class="help-section" id="paste-to-script">
            <h3>Paste to Screenplay</h3>
            <p>Drop a plain-text draft into the editor and Scriptty will convert it into proper screenplay elements automatically. From the Welcome screen pick <strong>Paste Script</strong>, or use <strong>File → Import → Paste Plain Text</strong>.</p>
            <p>The detector recognises Hollywood-style scene headings (<em>INT.</em>/<em>EXT.</em>), all-caps character cues, transitions ending in <em>TO:</em>, and bracketed parentheticals — plus a Malayalam-aware path that catches character cues when the cue line is the only line written entirely in Malayalam vowel-and-consonant patterns. Anything else becomes Action.</p>
            <p class="hint">The result is a regular screenplay you can edit normally — you can re-run the import on a different draft any time.</p>
          </section>

          <section class="help-section" id="malayalam">
            <h3>Malayalam Input</h3>
            <p>Scriptty has built-in Malayalam input — no OS keyboard setup required.</p>
            <ol>
              <li>Press <kbd>Ctrl+Space</kbd> to toggle between English and Malayalam.</li>
              <li>The status bar shows <strong>ENG</strong> or <strong class="accent">MAL</strong> to indicate the current mode.</li>
              <li>When in Malayalam mode, choose your input scheme from Settings (<kbd>⌘K</kbd> → Settings):
                <ul>
                  <li><strong>Mozhi</strong> — type phonetically in English (e.g., "namaskkaaram" → നമസ്ക്കാരം)</li>
                  <li><strong>Inscript 2</strong> — standard Indian keyboard layout</li>
                  <li><strong>Inscript 1</strong> — legacy Indian keyboard layout</li>
                </ul>
              </li>
            </ol>
            <p>You can mix Malayalam and English freely on the same line (e.g., "രമേഷ് Flat ലേക്ക് നടന്നു").</p>
          </section>

          <section class="help-section" id="autocomplete">
            <h3>Character Autocomplete</h3>
            <p>When typing a character name, Scriptty suggests names already used in your screenplay after 2 characters. Use <kbd>↑</kbd> / <kbd>↓</kbd> to navigate, <kbd>Enter</kbd> or <kbd>Tab</kbd> to accept, <kbd>Escape</kbd> to dismiss.</p>
            <p><strong>Malayalam names:</strong> matching ignores vowel signs and virama, so typing the consonant skeleton (e.g., <code>ram</code> → രമ) will match <strong>രാമൻ</strong>, <strong>രമേഷ്</strong>, and any other name built from those consonants. Partial middle matches also show up as suggestions — you don't need to start from the first consonant.</p>
          </section>

          <section class="help-section" id="formatting">
            <h3>Text Formatting</h3>
            <p>Use the <strong>B I U</strong> buttons in the toolbar or keyboard shortcuts to format text:</p>
            <ul>
              <li><kbd>Cmd+B</kbd> — <strong>Bold</strong></li>
              <li><kbd>Cmd+I</kbd> — <em>Italic</em></li>
              <li><kbd>Cmd+U</kbd> — <u>Underline</u></li>
            </ul>
            <p>Select text first to apply formatting, or toggle the format before typing. The buttons highlight when the format is active at the cursor. Formatting is preserved in PDF and Fountain exports.</p>
          </section>

          <section class="help-section" id="find">
            <h3>Find and Replace</h3>
            <p>Press <kbd>Cmd+F</kbd> to open the Find bar, or <kbd>Cmd+Shift+H</kbd> to open Find and Replace. Matches are highlighted in yellow, with the current match in orange.</p>
            <ul>
              <li><kbd>Enter</kbd> or the arrow buttons to move between matches</li>
              <li>Toggle case sensitivity with the <strong>Aa</strong> button</li>
              <li>Replace one match at a time, or use <strong>All</strong> to replace every match in one step (undoable with <kbd>Cmd+Z</kbd>)</li>
            </ul>
          </section>

          <section class="help-section" id="scene-nav">
            <h3>Scene Navigator</h3>
            <p>Press <kbd>Cmd+\</kbd> to toggle the left panel. It shows all your scene headings with auto-numbered labels.</p>
            <p>Click any scene to jump directly to it in the editor. Drag the six-dot handle that appears on hover to reorder scenes — the entire scene (heading + all content until the next scene) moves as a single block, undoable with <kbd>Cmd+Z</kbd>.</p>
            <p><strong>In a series:</strong> the panel shows an <strong>Episodes</strong> list above the scenes. Click an episode to switch to it (this also switches Story view and Cards view). Use the <strong>+</strong> to add a new episode.</p>
          </section>

          <section class="help-section" id="story-view">
            <h3>Story View</h3>
            <p>Press <kbd>Cmd+Shift+L</kbd> or click the <strong>Story</strong> tab in the title bar to enter Story view — all four story sections (Idea, Synopsis, Treatment, Narrative) in one full-page workspace.</p>
            <p>Malayalam input is supported (<kbd>Ctrl+Space</kbd> to toggle). A word count is shown in the status bar.</p>
          </section>

          <section class="help-section" id="views">
            <h3>Views</h3>
            <p>Scriptty has three views, switchable from the tab bar in the title bar:</p>
            <ul>
              <li><strong>Writing</strong> — the screenplay editor with scene annotations in the right margin</li>
              <li><strong>Cards</strong> — a grid of scene cards for planning and production breakdown</li>
              <li><strong>Story</strong> — a combined view for Idea, Synopsis, Treatment, and Narrative writing</li>
            </ul>
          </section>

          <section class="help-section" id="annotations">
            <h3>Scene Annotations</h3>
            <p>In Writing view, scene descriptions and notes appear in the right margin alongside each scene heading. These annotations are editable and saved with your <code>.screenplay</code> file.</p>
            <ul>
              <li><kbd>Cmd+Shift+D</kbd> — add or edit the annotation for the current scene</li>
              <li><kbd>Cmd+Shift+A</kbd> — toggle annotation visibility</li>
            </ul>
            <p>When an annotation is taller than its scene, the editor automatically adds space to keep everything aligned. You can also toggle annotations on or off from Settings.</p>
          </section>

          <section class="help-section" id="cards">
            <h3>Scene Cards</h3>
            <p>The Cards view (<kbd>Cmd+Shift+K</kbd>) is laid out as a typeset shooting-script tear sheet — each scene is a card with a zero-padded number chip, INT/EXT glyph, slug, cast list, description, shoot notes, and an optional production-prep group.</p>
            <p>Click <strong>Add Scene</strong> to create a new scene. Drag the scene number chip to reorder — this reorders the scenes in your screenplay. The number chip and gutter numeral pick up a warm or cool tint based on time-of-day (DAY scenes warm, NIGHT scenes cool). Click the heading to rename a scene inline; use the per-card <strong>Duplicate</strong> action to clone a scene's structure into a new one.</p>
            <p><strong>Compact mode</strong> in the Cards view collapses each card to slug + cast for an at-a-glance overview; <strong>Group by location</strong> clusters cards by their location group so co-located scenes sit together. Both toggles live in the Cards view header.</p>
            <p>Malayalam input is supported in card textareas (<kbd>Ctrl+Space</kbd> to toggle). Series projects also get an <strong>Episode breakout view</strong> — a top-level card per episode with a scene preview list; click into an episode to drill down to its scene cards.</p>
          </section>

          <section class="help-section" id="production">
            <h3>Production Planning</h3>
            <p>Each scene card carries optional production-prep fields the AD team uses on set:</p>
            <ul>
              <li><strong>Extras</strong> — comma-separated background / silent characters that don't appear as speakers. Merged with the auto-detected speaker list everywhere a cast list is shown.</li>
              <li><strong>Location group</strong> — a free-text tag (e.g. <code>STAGE-3</code>, <code>VILLAGE-EAST</code>) that clusters scenes shot at the same physical place. The Cards view's <strong>Group by location</strong> toggle uses this.</li>
              <li><strong>Shoot date</strong> — pick a date with the calendar popover. Scheduled scenes appear in the <strong>Daily Shoot List</strong> export and the <strong>Schedule</strong> tab of the statistics panel.</li>
            </ul>
            <p><strong>Daily Shoot List</strong> — turn this on in the Export modal to emit a per-day production report: scenes grouped by shoot date, sub-grouped by location group, with per-scene page-eighths (the industry-standard ⅛-page measure) and a per-day eighths total. Useful for printing and handing to the 1st AD.</p>
          </section>

          <section class="help-section" id="stats">
            <h3>Script Statistics</h3>
            <p>Press <kbd>Cmd+Shift+I</kbd> or go to <strong>View → Statistics</strong> for a vertical-tab panel covering five views:</p>
            <ul>
              <li><strong>Overview</strong> — page count, word count, scene count, dialogue blocks, estimated screen time, INT/EXT and day/night breakdowns</li>
              <li><strong>Characters</strong> — per-character scenes, dialogue blocks, and percentage of total dialogue</li>
              <li><strong>Locations</strong> — every unique slug with scene counts and INT/EXT split</li>
              <li><strong>Schedule</strong> — scheduled scenes grouped by shoot date with per-day totals; collapses cleanly when nothing is scheduled</li>
              <li><strong>Episodes</strong> (series only) — per-episode totals, status field, and a status pill (Outline / Draft / Revision / Final)</li>
            </ul>
            <p>The Characters / Locations / Schedule tables have <strong>sortable columns</strong> — click a column header to sort, click again to reverse — and a <strong>CSV export</strong> button so you can hand the data to a spreadsheet for production planning.</p>
          </section>

          <section class="help-section" id="export">
            <h3>Exporting to PDF</h3>
            <p>Click <strong>Export</strong> in the title bar to open the export modal — a two-pane layout with iconified section tiles on the left and format / option cards on the right.</p>
            <p>Pick the sections to include:</p>
            <ul>
              <li><strong>Title Page</strong> — generated from your metadata. Tagline, registration number, and a centered footnote come along when filled in.</li>
              <li><strong>Synopsis</strong>, <strong>Treatment</strong>, and <strong>Narrative</strong> — each renders on its own section cover (small eyebrow with flanking rules above the dominant film title).</li>
              <li><strong>Screenplay</strong> — the full script in Hollywood single-column or Indian two-column layout.</li>
              <li><strong>Scene Cards</strong> — full-width production cards with eyebrow, slug, cast, description, shoot notes, plus the scheduled date and location group when set.</li>
              <li><strong>Daily Shoot List</strong> — shoot-day report grouped by date and location with per-day eighths totals (only available when at least one scene has a shoot date).</li>
            </ul>
            <p>Bulk select / clear lets you pick everything at once. If the writer and director are the same person, the title page automatically reads "Written and Directed by".</p>
            <p>Pick a format:</p>
            <ul>
              <li><strong>Hollywood</strong> — standard single-column layout</li>
              <li><strong>Indian</strong> — two-column layout (dialogue left, translation right)</li>
            </ul>
            <p>Layout options:</p>
            <ul>
              <li><strong>Page break per scene</strong> — every scene starts on a fresh page</li>
              <li><strong>Characters under heading</strong> — auto-list speakers below each scene heading</li>
              <li><strong>Page numbers</strong> — top-right of every body page; each section restarts at 1</li>
              <li><strong>Compact card view</strong> — strips description / notes / location group from each scene card so the breakdown reads as a slug-only at-a-glance overview</li>
              <li><strong>Scene range</strong> — export only a slice of the script (useful for sending an actor their scenes)</li>
            </ul>
            <p><strong>Series projects</strong> show a <strong>Scope</strong> option: export just the <em>Active episode</em> or the <em>Entire series</em>. A series export starts each episode on a new page with a centered <em>EPISODE N: Title</em> header and restarts scene numbering from 1 per episode.</p>
            <p>The selected Malayalam font (Manjari or Noto Sans Malayalam) is embedded in the PDF, and Courier Prime is always bundled alongside it for the editorial accent typography (slugs, hero numerals, credit names). Bold, italic, and underline formatting is preserved.</p>
            <p>You can also export as:</p>
            <ul>
              <li><strong>Fountain</strong> — plain-text screenwriting format, compatible with Highland, Fade In, and other tools</li>
              <li><strong>Plain Text</strong> — formatted .txt file with proper screenplay indentation</li>
            </ul>
          </section>

          <section class="help-section" id="fonts">
            <h3>Fonts</h3>
            <p>English text in the editor uses <strong>Courier Prime</strong> — the Hollywood-standard screenplay face. Malayalam text renders in the font you pick under Settings (<kbd>⌘K</kbd> → Settings):</p>
            <ul>
              <li><strong>Manjari</strong> — the default, contemporary Malayalam display face</li>
              <li><strong>Noto</strong> — Noto Sans Malayalam, a lighter cut for a quieter feel</li>
            </ul>
            <p>The Malayalam font you pick is embedded in exported PDFs. Courier Prime is also bundled into every PDF — the templates use it for the editorial accent typography (slugs, hero numerals, credit names, scene-card eyebrow), so the on-screen vocabulary carries onto paper.</p>
            <p>The editor's font size is adjustable from Settings — independent of the PDF body size, so you can read comfortably while writing without changing the printed output.</p>
          </section>

          <section class="help-section" id="saving">
            <h3>Saving & Autosave</h3>
            <p>An amber dot next to the title means you have unsaved changes.</p>
            <ul>
              <li><kbd>Cmd+S</kbd> — save (or Save As on first save)</li>
              <li><kbd>Cmd+Shift+S</kbd> — Save As (pick a new location)</li>
            </ul>
            <p>If you try to create a new document, open another file, or quit while there are unsaved changes, Scriptty will ask if you want to save first.</p>
            <p><strong>Autosave + crash recovery.</strong> Scriptty silently autosaves your in-progress work to a hidden recovery file every few seconds while you type. If the app or your computer crashes, the next launch detects the recovery file and offers to restore the unsaved work — so even an unsaved new document survives a power loss. Saving normally with <kbd>Cmd+S</kbd> clears the recovery file.</p>
          </section>

          <section class="help-section" id="shortcuts">
            <h3>Keyboard Shortcuts</h3>
            <p class="hint">On Windows/Linux, replace <kbd>Cmd</kbd> with <kbd>Ctrl</kbd>.</p>

            <div class="shortcut-grid">
              <div class="shortcut-group">
                <h4>File</h4>
                <dl>
                  <dt><kbd>Cmd+K</kbd></dt><dd>Command palette (run any action)</dd>
                  <dt><kbd>Cmd+N</kbd></dt><dd>New film</dd>
                  <dt><kbd>Cmd+Shift+N</kbd></dt><dd>New series</dd>
                  <dt><kbd>Cmd+O</kbd></dt><dd>Open document</dd>
                  <dt><kbd>Cmd+S</kbd></dt><dd>Save</dd>
                  <dt><kbd>Cmd+Shift+S</kbd></dt><dd>Save As</dd>
                </dl>
              </div>

              <div class="shortcut-group">
                <h4>Formatting</h4>
                <dl>
                  <dt><kbd>Cmd+B</kbd></dt><dd>Bold</dd>
                  <dt><kbd>Cmd+I</kbd></dt><dd>Italic</dd>
                  <dt><kbd>Cmd+U</kbd></dt><dd>Underline</dd>
                  <dt><kbd>Cmd+Z</kbd></dt><dd>Undo</dd>
                  <dt><kbd>Cmd+Shift+Z</kbd></dt><dd>Redo</dd>
                </dl>
              </div>

              <div class="shortcut-group">
                <h4>Navigation</h4>
                <dl>
                  <dt><kbd>Tab</kbd></dt><dd>Next element type</dd>
                  <dt><kbd>Shift+Tab</kbd></dt><dd>Previous element type</dd>
                  <dt><kbd>Cmd+1</kbd></dt><dd>Scene heading</dd>
                  <dt><kbd>Cmd+2</kbd></dt><dd>Action</dd>
                  <dt><kbd>Cmd+3</kbd></dt><dd>Character</dd>
                  <dt><kbd>Cmd+4</kbd></dt><dd>Dialogue</dd>
                  <dt><kbd>Cmd+Shift+T</kbd></dt><dd>Transition</dd>
                  <dt><kbd>Shift+Enter</kbd></dt><dd>New scene heading</dd>
                  <dt><kbd>Escape</kbd></dt><dd>Close any modal</dd>
                </dl>
              </div>

              <div class="shortcut-group">
                <h4>Views &amp; Panels</h4>
                <dl>
                  <dt><kbd>Cmd+\</kbd></dt><dd>Toggle left panel</dd>
                  <dt><kbd>Cmd+Shift+K</kbd></dt><dd>Scene Cards view</dd>
                  <dt><kbd>Cmd+Shift+L</kbd></dt><dd>Story view</dd>
                  <dt><kbd>Cmd+Shift+O</kbd></dt><dd>Toggle outline peek strip</dd>
                  <dt><kbd>Cmd+Shift+A</kbd></dt><dd>Toggle annotations</dd>
                  <dt><kbd>Cmd+Shift+D</kbd></dt><dd>Edit current annotation</dd>
                  <dt><kbd>Cmd+Shift+I</kbd></dt><dd>Script statistics</dd>
                </dl>
              </div>

              <div class="shortcut-group">
                <h4>Search</h4>
                <dl>
                  <dt><kbd>Cmd+F</kbd></dt><dd>Find</dd>
                  <dt><kbd>Cmd+Shift+H</kbd></dt><dd>Find and Replace</dd>
                </dl>
              </div>

              <div class="shortcut-group">
                <h4>Input</h4>
                <dl>
                  <dt><kbd>Ctrl+Space</kbd></dt><dd>Toggle English / Malayalam</dd>
                </dl>
              </div>
            </div>
          </section>

        </div>
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
    width: var(--modal-w-lg);
    max-width: 92vw;
    height: 82vh;
    max-height: 820px;
    box-shadow: var(--modal-shadow);
    animation: modal-in var(--modal-anim-duration) ease-out;
    font-family: system-ui, -apple-system, sans-serif;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  @keyframes modal-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 20px 24px 18px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .modal-header-text {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .modal-header :global(.mh-eyebrow) {
    flex-shrink: 0;
  }

  .help-title {
    margin: 0;
    font-size: 18px;
    line-height: 1;
  }

  .btn-close {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    font-size: 18px;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .btn-close:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  /* Body splits into sidebar + content. Body fills remaining vertical
     space; children manage their own scroll. */
  .modal-body {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  /* ─── Sidebar TOC ─── */
  .help-toc {
    flex: 0 0 220px;
    width: 220px;
    min-width: 220px;
    border-right: 1px solid var(--border-subtle);
    background: var(--surface-elevated);
    padding: 16px 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .toc-list {
    list-style: none;
    padding: 0 8px;
    margin: 0;
    flex: 1;
  }

  .toc-list li {
    margin: 1px 0;
  }

  .toc-link {
    display: block;
    width: 100%;
    text-align: left;
    padding: 6px 12px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12.5px;
    font-family: inherit;
    border-radius: 6px;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }

  .toc-link:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .toc-link.active {
    background: var(--accent-muted);
    color: var(--accent);
    font-weight: 600;
  }

  .toc-footer {
    margin-top: 16px;
    padding: 12px 20px 4px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11.5px;
  }

  .about-link {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    color: var(--accent);
    cursor: pointer;
    text-align: left;
    align-self: flex-start;
  }

  .about-link:hover {
    text-decoration: underline;
  }

  .toc-footer-version {
    color: var(--text-muted);
    font-size: 11px;
  }

  /* ─── Content pane ─── */
  .help-content {
    flex: 1;
    overflow-y: auto;
    padding: 8px 40px 32px;
    scroll-behavior: smooth;
    /* Make this the offsetParent for child sections so offsetTop is
       correct relative to the scroll container. */
    position: relative;
  }

  .help-section {
    padding: 24px 0;
    border-bottom: 1px solid var(--border-subtle);
    max-width: 680px;
  }

  .help-section:last-child {
    border-bottom: none;
  }

  .help-section h3 {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 12px;
    letter-spacing: -0.01em;
  }

  .help-section p {
    font-size: 13px;
    line-height: 1.65;
    color: var(--text-secondary);
    margin: 8px 0;
  }

  .help-section ol,
  .help-section ul {
    font-size: 13px;
    line-height: 1.65;
    color: var(--text-secondary);
    margin: 8px 0;
    padding-left: 20px;
  }

  .help-section li {
    margin: 4px 0;
  }

  code {
    font-size: 12px;
    background: var(--surface-hover);
    padding: 1px 5px;
    border-radius: 3px;
    color: var(--text-primary);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

  kbd {
    display: inline-block;
    font-size: 11px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    padding: 1px 6px;
    border: 1px solid var(--border-medium);
    border-radius: 4px;
    background: var(--surface-elevated);
    color: var(--text-primary);
    box-shadow: 0 1px 0 var(--border-subtle);
    line-height: 1.6;
  }

  .accent {
    color: var(--accent);
  }

  /* Tables embedded in content (e.g. Writing section's flow table). */
  .shortcut-table {
    width: 100%;
    border-collapse: collapse;
    margin: 12px 0;
    font-size: 13px;
  }

  .shortcut-table th {
    text-align: left;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .shortcut-table td {
    padding: 5px 8px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-subtle);
  }

  .shortcut-table tbody tr:last-child td {
    border-bottom: none;
  }

  .hint {
    font-size: 12px;
    font-style: italic;
    color: var(--text-muted);
  }

  /* ─── Keyboard Shortcuts grid ───
     Responsive multi-column layout of grouped dl/dt/dd pairs. Each group
     has a small header + rows of <kbd> ↔ action. Uses CSS grid with
     auto-fit so narrow windows collapse to a single column gracefully. */
  .shortcut-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 20px 32px;
    margin-top: 16px;
  }

  .shortcut-group h4 {
    font-size: 10.5px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin: 0 0 8px;
  }

  .shortcut-group dl {
    display: grid;
    grid-template-columns: auto 1fr;
    column-gap: 12px;
    row-gap: 6px;
    margin: 0;
    font-size: 12.5px;
  }

  .shortcut-group dt {
    justify-self: start;
  }

  .shortcut-group dd {
    margin: 0;
    color: var(--text-secondary);
    align-self: center;
  }
</style>
