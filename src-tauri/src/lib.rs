// Declare the commands module for Tauri command handlers (file I/O, export).
mod commands;
// Declare the fonts module for bundled font loading.
mod fonts;
// Declare the screenplay module so Rust knows about our document types.
mod screenplay;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    // Register the dialog plugin so the frontend can open native file dialogs.
    .plugin(tauri_plugin_dialog::init())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::file::new_screenplay,
      commands::file::save_screenplay,
      commands::file::open_screenplay,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
