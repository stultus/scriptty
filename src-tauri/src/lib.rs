// Declare the commands module for Tauri command handlers (file I/O, export).
mod commands;
// Declare the fonts module for bundled font loading.
mod fonts;
// Declare the screenplay module so Rust knows about our document types.
mod screenplay;

// Menu API types for building the native macOS/Windows/Linux app menu bar.
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
// Emitter trait lets us send events from Rust to the frontend webview.
use tauri::Emitter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    // Register the dialog plugin so the frontend can open native file dialogs.
    .plugin(tauri_plugin_dialog::init())
    // Register the FS plugin so the frontend can write files (e.g. PDF export).
    .plugin(tauri_plugin_fs::init())
    // Register the opener plugin so we can open URLs in the default browser.
    .plugin(tauri_plugin_opener::init())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // --- Native App Menu ---
      // Build a standard File + Edit + Help menu bar. Custom items (New, Open, Save,
      // Save As, About, etc.) emit events to the frontend. Predefined items (Undo, Redo,
      // Cut, Copy, Paste, Select All, Quit) are handled automatically by the OS.

      // File menu with custom items for document operations
      let file_menu = Submenu::with_items(
        app,
        "File",
        true, // enabled
        &[
          // MenuItem::with_id creates a menu item with a custom ID we can match on later.
          // Args: app handle, id, display text, enabled, optional keyboard accelerator.
          &MenuItem::with_id(app, "new", "New", true, Some("CmdOrCtrl+N"))?,
          &MenuItem::with_id(app, "open", "Open...", true, Some("CmdOrCtrl+O"))?,
          &PredefinedMenuItem::separator(app)?,
          &MenuItem::with_id(app, "save", "Save", true, Some("CmdOrCtrl+S"))?,
          &MenuItem::with_id(app, "save-as", "Save As...", true, Some("CmdOrCtrl+Shift+S"))?,
          &PredefinedMenuItem::separator(app)?,
          // Custom quit item instead of PredefinedMenuItem::quit so the frontend
          // can intercept it and prompt for unsaved changes before quitting.
          &MenuItem::with_id(app, "quit", "Quit Scriptty", true, Some("CmdOrCtrl+Q"))?,
        ],
      )?;

      // Edit menu with standard OS-handled items (no custom event handling needed)
      let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
          &PredefinedMenuItem::undo(app, None)?,
          &PredefinedMenuItem::redo(app, None)?,
          &PredefinedMenuItem::separator(app)?,
          &PredefinedMenuItem::cut(app, None)?,
          &PredefinedMenuItem::copy(app, None)?,
          &PredefinedMenuItem::paste(app, None)?,
          &PredefinedMenuItem::select_all(app, None)?,
          &PredefinedMenuItem::separator(app)?,
          &MenuItem::with_id(app, "find", "Find", true, Some("CmdOrCtrl+F"))?,
          &MenuItem::with_id(app, "find-replace", "Find and Replace", true, Some("CmdOrCtrl+Shift+H"))?,
        ],
      )?;

      // View menu for statistics and scene cards
      let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[
          &MenuItem::with_id(app, "statistics", "Statistics", true, Some("CmdOrCtrl+Shift+I"))?,
          &MenuItem::with_id(app, "scene-cards", "Scene Cards", true, Some("CmdOrCtrl+Shift+K"))?,
        ],
      )?;

      // Help menu with About dialog and external links
      let help_menu = Submenu::with_items(
        app,
        "Help",
        true,
        &[
          &MenuItem::with_id(app, "about", "About Scriptty", true, None::<&str>)?,
          &MenuItem::with_id(app, "help-guide", "How to Use Scriptty", true, None::<&str>)?,
          &PredefinedMenuItem::separator(app)?,
          &MenuItem::with_id(app, "report-issue", "Report an Issue", true, None::<&str>)?,
          &MenuItem::with_id(app, "view-github", "View on GitHub", true, None::<&str>)?,
        ],
      )?;

      // Assemble the menu bar from the submenus and apply it to the app
      let menu = Menu::with_items(app, &[&file_menu, &edit_menu, &view_menu, &help_menu])?;
      app.set_menu(menu)?;

      // Handle clicks on our custom menu items by emitting events to the frontend.
      // The `move` keyword transfers ownership of captured variables into the closure —
      // needed because this closure outlives the setup function.
      // `event.id().as_ref()` gives us the string ID we set in MenuItem::with_id above.
      app.on_menu_event(move |app, event| {
        // Match the menu item's ID string and emit the corresponding event.
        // `let _ = ...` discards the Result — if emit fails, we silently ignore it
        // (there's no meaningful recovery for a failed emit).
        match event.id().as_ref() {
          "new" => { let _ = app.emit("menu-new", ()); }
          "open" => { let _ = app.emit("menu-open", ()); }
          "save" => { let _ = app.emit("menu-save", ()); }
          "save-as" => { let _ = app.emit("menu-save-as", ()); }
          "about" => { let _ = app.emit("menu-about", ()); }
          "help-guide" => { let _ = app.emit("menu-help-guide", ()); }
          "statistics" => { let _ = app.emit("menu-statistics", ()); }
          "scene-cards" => { let _ = app.emit("menu-scene-cards", ()); }
          "find" => { let _ = app.emit("menu-find", ()); }
          "find-replace" => { let _ = app.emit("menu-find-replace", ()); }
          "quit" => { let _ = app.emit("menu-quit", ()); }
          // External links — open in the default browser using the opener plugin.
          // `tauri_plugin_opener::OpenerExt` provides the `.opener()` method on AppHandle.
          "report-issue" => {
            use tauri_plugin_opener::OpenerExt;
            let _ = app.opener().open_url("https://github.com/stultus/scriptty/issues", None::<&str>);
          }
          "view-github" => {
            use tauri_plugin_opener::OpenerExt;
            let _ = app.opener().open_url("https://github.com/stultus/scriptty", None::<&str>);
          }
          _ => {} // Ignore predefined items — the OS handles those
        }
      });

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::file::new_screenplay,
      commands::file::save_screenplay,
      commands::file::open_screenplay,
      commands::export::export_typst_markup,
      commands::export::export_pdf,
      commands::export::export_pdf_indian,
      commands::export::export_combined_pdf,
      commands::export::export_plaintext,
      commands::export::export_fountain,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
