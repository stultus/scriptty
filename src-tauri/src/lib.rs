// Declare the commands module for Tauri command handlers (file I/O, export).
mod commands;
// Declare the fonts module for bundled font loading.
mod fonts;
// Declare the screenplay module so Rust knows about our document types.
mod screenplay;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
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
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
