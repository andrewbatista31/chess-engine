mod commands;

use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let path = app
                .path()
                .resolve(
                    "binaries/stockfish-x86_64-pc-windows-msvc.exe",
                    tauri::path::BaseDirectory::Resource,
                )
                .unwrap_or_else(|_| {
                    // Dev fallback: relative to CARGO_MANIFEST_DIR (src-tauri/).
                    std::path::PathBuf::from("binaries/stockfish-x86_64-pc-windows-msvc.exe")
                });
            app.manage(commands::EngineManager::new(path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::legal_moves,
            commands::make_move,
            commands::validate_fen,
            commands::parse_pgn,
            commands::serialize_pgn,
            commands::save_pgn_file,
            commands::load_pgn_file,
            commands::start_analysis,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::Exit = event {
                // Drop the engine so its kill_on_drop Child terminates Stockfish.
                if let Some(mgr) = app_handle.try_state::<commands::EngineManager>() {
                    let _ = mgr.engine.lock().map(|mut g| *g = None);
                }
            }
        });
}
