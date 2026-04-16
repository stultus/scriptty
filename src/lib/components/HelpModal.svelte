<script lang="ts">
  import { focusTrap } from '$lib/actions/focusTrap';

  let { open = $bindable(false), onShowAbout }: {
    open: boolean;
    onShowAbout?: () => void;
  } = $props();

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
  <div class="modal-backdrop" onclick={handleBackdropClick} onkeydown={handleKeydown} role="dialog" aria-modal="true" tabindex="-1">
    <div class="modal-card" use:focusTrap>
      <div class="modal-header">
        <h2>How to Use Scriptty</h2>
        <button class="btn-close" onclick={() => { open = false; }}>&times;</button>
      </div>

      <div class="help-content">

        <!-- Getting Started -->
        <section class="help-section">
          <h3>Getting Started</h3>
          <p>Scriptty is an offline screenwriting app for Malayalam and English writers. Your work is saved locally as <code>.screenplay</code> files — no cloud, no account needed.</p>
          <ol>
            <li>Click <strong>New</strong> in the title bar to create a blank screenplay.</li>
            <li>Go to <strong>View → Edit Meta Data</strong> to set your title, writer, director, and draft info.</li>
            <li>Start writing — the editor begins in <em>Action</em> mode. Press <kbd>Tab</kbd> to switch to a Character name, then <kbd>Enter</kbd> to write Dialogue.</li>
            <li>Use <kbd>Cmd+B</kbd>, <kbd>Cmd+I</kbd>, <kbd>Cmd+U</kbd> or the <strong>B I U</strong> buttons in the toolbar for bold, italic, and underline formatting.</li>
            <li>Press <kbd>Cmd+S</kbd> to save your work.</li>
          </ol>
        </section>

        <!-- Writing in the Editor -->
        <section class="help-section">
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
              <tr><td>Transition</td><td><kbd>Enter</kbd></td><td>Scene Heading</td></tr>
            </tbody>
          </table>

          <p><kbd>Shift+Enter</kbd> creates a new Scene Heading from anywhere.<br />
          <kbd>Shift+Tab</kbd> converts the current element back to Action (or Scene Heading if already Action, Dialogue if in Parenthetical).</p>

          <p><strong>Dialogue loop:</strong> Character → Dialogue → Character flows continuously. Press <kbd>Enter</kbd> on an empty Character to break out to Action. Press <kbd>Enter</kbd> on an empty Dialogue to return to the Character above.</p>
          <p><strong>Paragraph splitting:</strong> Press <kbd>Enter</kbd> in the middle of an Action element to split it into two paragraphs.</p>
          <p>Scene headings and character names are automatically uppercased as you type.</p>
          <p>Parentheticals are automatically wrapped in parentheses — just type the direction (e.g., "whispering") and the app adds <strong>(</strong> and <strong>)</strong> for you.</p>
        </section>

        <!-- Malayalam Input -->
        <section class="help-section">
          <h3>Malayalam Input</h3>
          <p>Scriptty has built-in Malayalam input — no OS keyboard setup required.</p>
          <ol>
            <li>Press <kbd>Ctrl+Space</kbd> to toggle between English and Malayalam.</li>
            <li>The status bar shows <strong>ENG</strong> or <strong class="accent">MAL</strong> to indicate the current mode.</li>
            <li>When in Malayalam mode, choose your input scheme from Settings (gear icon):
              <ul>
                <li><strong>Mozhi</strong> — type phonetically in English (e.g., "namaskkaaram" → നമസ്ക്കാരം)</li>
                <li><strong>Inscript 2</strong> — standard Indian keyboard layout</li>
                <li><strong>Inscript 1</strong> — legacy Indian keyboard layout</li>
              </ul>
            </li>
          </ol>
          <p>You can mix Malayalam and English freely on the same line (e.g., "രമേഷ് Flat ലേക്ക് നടന്നു").</p>
        </section>

        <!-- Character Autocomplete -->
        <section class="help-section">
          <h3>Character Autocomplete</h3>
          <p>When typing a character name, Scriptty suggests names already used in your screenplay after 2 characters. Use <kbd>↑</kbd> / <kbd>↓</kbd> to navigate, <kbd>Enter</kbd> or <kbd>Tab</kbd> to accept, <kbd>Escape</kbd> to dismiss.</p>
        </section>

        <!-- Text Formatting -->
        <section class="help-section">
          <h3>Text Formatting</h3>
          <p>Use the <strong>B I U</strong> buttons in the toolbar or keyboard shortcuts to format text:</p>
          <ul>
            <li><kbd>Cmd+B</kbd> — <strong>Bold</strong></li>
            <li><kbd>Cmd+I</kbd> — <em>Italic</em></li>
            <li><kbd>Cmd+U</kbd> — <u>Underline</u></li>
          </ul>
          <p>Select text first to apply formatting, or toggle the format before typing. The buttons highlight when the format is active at the cursor. Formatting is preserved in PDF and Fountain exports.</p>
        </section>

        <!-- Find and Replace -->
        <section class="help-section">
          <h3>Find and Replace</h3>
          <p>Press <kbd>Cmd+F</kbd> to open the Find bar, or <kbd>Cmd+Shift+H</kbd> to open Find and Replace. Matches are highlighted in yellow, with the current match in orange.</p>
          <ul>
            <li><kbd>Enter</kbd> or the arrow buttons to move between matches</li>
            <li>Toggle case sensitivity with the <strong>Aa</strong> button</li>
            <li>Replace one match at a time, or use <strong>All</strong> to replace every match in one step (undoable with <kbd>Cmd+Z</kbd>)</li>
          </ul>
        </section>

        <!-- Scene Navigator -->
        <section class="help-section">
          <h3>Scene Navigator</h3>
          <p>Press <kbd>Cmd+\</kbd> to toggle the left panel. The <strong>Scenes</strong> tab shows all your scene headings with auto-numbered labels.</p>
          <p>Click any scene to jump directly to it in the editor. Drag the handle (<strong>⠿</strong>) that appears on hover to reorder scenes — the entire scene (heading + all content until the next scene) moves as a single block, undoable with <kbd>Cmd+Z</kbd>.</p>
        </section>

        <!-- Story Panel -->
        <section class="help-section">
          <h3>Story Panel</h3>
          <p>The <strong>Story</strong> tab in the left panel provides quick access to your story's structure (Idea, Synopsis, Treatment, Narrative). For a full-page writing experience, use the Story view instead.</p>
        </section>

        <!-- Story View -->
        <section class="help-section">
          <h3>Story View</h3>
          <p>Press <kbd>Cmd+Shift+L</kbd> or click the <strong>Story</strong> tab to enter Story view — all four story sections (Idea, Synopsis, Treatment, Narrative) in one continuous page.</p>
          <p>Malayalam input is supported (<kbd>Ctrl+Space</kbd> to toggle). A word count is shown in the status bar.</p>
        </section>

        <!-- Views -->
        <section class="help-section">
          <h3>Views</h3>
          <p>Scriptty has three views, switchable from the tab bar in the title bar:</p>
          <ul>
            <li><strong>Writing</strong> — the screenplay editor with scene annotations in the right margin</li>
            <li><strong>Cards</strong> — a grid of scene cards for planning and production breakdown</li>
            <li><strong>Story</strong> — a combined view for Idea, Synopsis, Treatment, and Narrative writing</li>
          </ul>
        </section>

        <!-- Scene Annotations -->
        <section class="help-section">
          <h3>Scene Annotations</h3>
          <p>In Writing view, scene descriptions and notes appear in the right margin alongside each scene heading. These annotations are editable and saved with your <code>.screenplay</code> file.</p>
          <ul>
            <li><kbd>Cmd+Shift+D</kbd> — add or edit the annotation for the current scene</li>
            <li><kbd>Cmd+Shift+A</kbd> — toggle annotation visibility</li>
          </ul>
          <p>When an annotation is taller than its scene, the editor automatically adds space to keep everything aligned. You can also toggle annotations on or off from Settings.</p>
        </section>

        <!-- Scene Cards -->
        <section class="help-section">
          <h3>Scene Cards</h3>
          <p>The Cards view shows a grid of cards for planning and production notes. Each card shows the scene heading, characters, description, notes, and a page estimate.</p>
          <p>Click <strong>Add Scene</strong> to create a new scene from the Cards view. Drag the scene number badge to reorder cards — this reorders the scenes in your screenplay. Malayalam input is supported in card textareas (<kbd>Ctrl+Space</kbd> to toggle).</p>
        </section>

        <!-- Statistics -->
        <section class="help-section">
          <h3>Script Statistics</h3>
          <p>Press <kbd>Cmd+Shift+I</kbd> or go to <strong>View → Statistics</strong> to see a breakdown of your screenplay:</p>
          <ul>
            <li>Page count, word count, scene count, dialogue blocks, estimated screen time</li>
            <li>Interior vs. exterior and day vs. night scene counts</li>
            <li>Per-character table showing scenes, dialogue blocks, and percentage of total dialogue</li>
          </ul>
        </section>

        <!-- Exporting -->
        <section class="help-section">
          <h3>Exporting to PDF</h3>
          <p>Click <strong>Export</strong> in the title bar to open the export dialog.</p>
          <p>Choose what to include in your PDF:</p>
          <ul>
            <li><strong>Title Page</strong> — generated from your metadata (title, writer, director credits)</li>
            <li><strong>Synopsis</strong>, <strong>Treatment</strong>, and <strong>Narrative</strong> — from the Story panel</li>
            <li><strong>Screenplay</strong> — the full script</li>
            <li><strong>Scene Cards</strong> — production breakdown table</li>
          </ul>
          <p>If the writer and director are the same person, the title page shows "Written and Directed by" automatically.</p>
          <p>Pick a format:</p>
          <ul>
            <li><strong>Hollywood</strong> — standard single-column layout</li>
            <li><strong>Indian</strong> — two-column layout (dialogue left, translation right)</li>
          </ul>
          <p>Under <strong>Layout</strong>, you can enable <strong>Page break after each scene</strong> to start every scene on a new page.</p>
          <p>The selected font (Noto Sans Malayalam or Manjari) is embedded in the PDF. Bold, italic, and underline formatting is preserved.</p>
          <p>You can also export as:</p>
          <ul>
            <li><strong>Fountain</strong> — plain-text screenwriting format, compatible with Highland, Fade In, and other tools</li>
            <li><strong>Plain Text</strong> — formatted .txt file with proper screenplay indentation</li>
          </ul>
        </section>

        <!-- Fonts -->
        <section class="help-section">
          <h3>Fonts</h3>
          <p>Open Settings (gear icon in the status bar) to switch between:</p>
          <ul>
            <li><strong>Noto</strong> — Noto Sans Malayalam (default, clean and modern)</li>
            <li><strong>Manjari</strong> — a lighter, more traditional feel</li>
          </ul>
          <p>The font applies to both the editor and exported PDFs. Both fonts support Malayalam and English.</p>
        </section>

        <!-- Saving -->
        <section class="help-section">
          <h3>Saving Your Work</h3>
          <p>An amber dot next to the title means you have unsaved changes.</p>
          <ul>
            <li><kbd>Cmd+S</kbd> — save (or Save As on first save)</li>
            <li><kbd>Cmd+Shift+S</kbd> — Save As (pick a new location)</li>
          </ul>
          <p>If you try to create a new document, open another file, or quit while there are unsaved changes, Scriptty will ask if you want to save first.</p>
        </section>

        <!-- Keyboard Shortcuts Reference -->
        <section class="help-section">
          <h3>Keyboard Shortcuts</h3>
          <table class="shortcut-table">
            <thead>
              <tr><th>Shortcut</th><th>Action</th></tr>
            </thead>
            <tbody>
              <tr><td><kbd>Cmd+N</kbd></td><td>New document</td></tr>
              <tr><td><kbd>Cmd+O</kbd></td><td>Open document</td></tr>
              <tr><td><kbd>Cmd+S</kbd></td><td>Save</td></tr>
              <tr><td><kbd>Cmd+Shift+S</kbd></td><td>Save As</td></tr>
              <tr><td><kbd>Cmd+B</kbd></td><td>Bold text</td></tr>
              <tr><td><kbd>Cmd+I</kbd></td><td>Italic text</td></tr>
              <tr><td><kbd>Cmd+U</kbd></td><td>Underline text</td></tr>
              <tr><td><kbd>Cmd+\</kbd></td><td>Toggle left panel</td></tr>
              <tr><td><kbd>Cmd+Shift+K</kbd></td><td>Toggle Cards view</td></tr>
              <tr><td><kbd>Cmd+Shift+L</kbd></td><td>Toggle Story view</td></tr>
              <tr><td><kbd>Cmd+Shift+A</kbd></td><td>Toggle annotations</td></tr>
              <tr><td><kbd>Cmd+Shift+D</kbd></td><td>Edit current scene annotation</td></tr>
              <tr><td><kbd>Cmd+Shift+I</kbd></td><td>Script statistics</td></tr>
              <tr><td><kbd>Cmd+F</kbd></td><td>Find</td></tr>
              <tr><td><kbd>Cmd+Shift+H</kbd></td><td>Find and Replace</td></tr>
              <tr><td><kbd>Ctrl+Space</kbd></td><td>Toggle English / Malayalam</td></tr>
              <tr><td><kbd>Cmd+Z</kbd></td><td>Undo</td></tr>
              <tr><td><kbd>Cmd+Shift+Z</kbd></td><td>Redo</td></tr>
              <tr><td><kbd>Tab</kbd></td><td>Next element type</td></tr>
              <tr><td><kbd>Shift+Tab</kbd></td><td>Previous element type</td></tr>
              <tr><td><kbd>Cmd+Shift+T</kbd></td><td>Convert to transition</td></tr>
              <tr><td><kbd>Shift+Enter</kbd></td><td>New scene heading</td></tr>
              <tr><td><kbd>Escape</kbd></td><td>Close any modal</td></tr>
            </tbody>
          </table>
          <p class="hint">On Windows/Linux, replace <kbd>Cmd</kbd> with <kbd>Ctrl</kbd>.</p>
        </section>

        <div class="help-footer">
          <button class="about-link" onclick={handleAboutClick}>About Scriptty</button>
          <span class="help-footer-dot">·</span>
          <span class="help-footer-version">v0.5.2</span>
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
    width: 480px;
    max-width: 90vw;
    max-height: 85vh;
    box-shadow: 0 8px 32px var(--shadow-heavy);
    animation: modal-in 150ms ease-out;
    font-family: system-ui, -apple-system, sans-serif;
    display: flex;
    flex-direction: column;
  }

  @keyframes modal-in {
    from { opacity: 0; transform: scale(0.97); }
    to { opacity: 1; transform: scale(1); }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 24px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 15px;
    color: var(--text-primary);
    font-weight: 600;
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

  .help-content {
    overflow-y: auto;
    padding: 24px;
  }

  .help-section {
    padding: 16px 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .help-section:last-child {
    border-bottom: none;
  }

  .help-section h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 8px;
  }

  .help-section p {
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-secondary);
    margin: 6px 0;
  }

  .help-section ol,
  .help-section ul {
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-secondary);
    margin: 6px 0;
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
  }

  kbd {
    display: inline-block;
    font-size: 11px;
    font-family: system-ui, -apple-system, sans-serif;
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

  .shortcut-table {
    width: 100%;
    border-collapse: collapse;
    margin: 8px 0;
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

  .help-footer {
    margin-top: 24px;
    padding-top: 16px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .about-link {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    color: var(--accent);
    cursor: pointer;
    text-decoration: none;
  }

  .about-link:hover {
    text-decoration: underline;
  }

  .help-footer-dot {
    color: var(--text-muted);
  }
</style>
