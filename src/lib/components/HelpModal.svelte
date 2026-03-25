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
            <li>Click <strong>Meta</strong> to set your title, writer, director, and draft info.</li>
            <li>Start writing — the editor begins in <em>Action</em> mode. Press <kbd>Tab</kbd> to switch to a Character name, then <kbd>Enter</kbd> to write Dialogue.</li>
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
          <p>Scene headings and character names are automatically uppercased as you type.</p>
          <p>Parentheticals are automatically wrapped in parentheses — just type the direction (e.g., "whispering") and the app adds <strong>(</strong> and <strong>)</strong> for you.</p>
        </section>

        <!-- Malayalam Input -->
        <section class="help-section">
          <h3>Malayalam Input</h3>
          <p>Scriptty has built-in Malayalam input — no OS keyboard setup required.</p>
          <ol>
            <li>Press <kbd>Ctrl+Space</kbd> to toggle between English and Malayalam.</li>
            <li>The status bar at the bottom shows your current mode (<strong>ENGLISH</strong> or <strong class="accent">MALAYALAM</strong>).</li>
            <li>When in Malayalam mode, choose your input scheme from the status bar:
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
          <p>Switch to the <strong>Story</strong> tab in the left panel to work on your story's structure:</p>
          <ul>
            <li><strong>Idea</strong> — your logline or core premise (1–3 lines)</li>
            <li><strong>Synopsis</strong> — the full story arc in prose (300–800 words)</li>
            <li><strong>Treatment</strong> — detailed scene-by-scene narrative</li>
            <li><strong>Narrative</strong> — full-length prose version of your story</li>
          </ul>
          <p>The panel widens automatically when the Story tab is active. Everything you write here is saved in the <code>.screenplay</code> file and can be included in PDF exports.</p>
        </section>

        <!-- Story Mode -->
        <section class="help-section">
          <h3>Story Mode</h3>
          <p>Press <kbd>Cmd+Shift+L</kbd> to enter Story Mode — a full-screen, distraction-free writing view for your narrative.</p>
          <p>Story Mode has its own status bar with Malayalam input support (<kbd>Ctrl+Space</kbd> to toggle) and a live word count. Press <kbd>Escape</kbd> or click <strong>Back to Script</strong> to return to the screenplay editor.</p>
        </section>

        <!-- Scene Cards -->
        <section class="help-section">
          <h3>Scene Cards</h3>
          <p>Press <kbd>Cmd+Shift+K</kbd> to open the Scene Cards view — a grid of cards for planning and production notes.</p>
          <p>Each card automatically shows:</p>
          <ul>
            <li>Scene number and heading</li>
            <li>Location and time of day (parsed from the heading)</li>
            <li>Characters who appear in the scene</li>
            <li>Estimated page count</li>
          </ul>
          <p>You can add a <strong>Description</strong> (what happens) and <strong>Shoot Notes</strong> (equipment, VFX, location details) to each card. Drag the scene number badge to reorder cards — this reorders the scenes in your screenplay. Click <strong>Back to Script</strong> to return to the editor.</p>
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
          <p>The selected font (Noto Sans Malayalam or Manjari) is embedded in the PDF.</p>
          <p>You can also export as:</p>
          <ul>
            <li><strong>Fountain</strong> — plain-text screenwriting format, compatible with Highland, Fade In, and other tools</li>
            <li><strong>Plain Text</strong> — formatted .txt file with proper screenplay indentation</li>
          </ul>
        </section>

        <!-- Fonts -->
        <section class="help-section">
          <h3>Fonts</h3>
          <p>Use the segmented control in the title bar to switch between:</p>
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
              <tr><td><kbd>Cmd+Shift+K</kbd></td><td>Toggle scene cards</td></tr>
              <tr><td><kbd>Cmd+Shift+I</kbd></td><td>Script statistics</td></tr>
              <tr><td><kbd>Cmd+Shift+L</kbd></td><td>Story Mode</td></tr>
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
    width: 480px;
    max-width: 90vw;
    max-height: 85vh;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
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
</style>
