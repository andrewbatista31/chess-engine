mod commands;

use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Resolve the bundled Stockfish path. In release builds Tauri places the
            // externalBin under BaseDirectory::Resource. In dev (`tauri dev` / `cargo run`)
            // it lives at <src-tauri>/binaries/...exe, which CARGO_MANIFEST_DIR points to.
            // `resolve()` succeeds in dev too but returns a path inside target/ that
            // doesn't exist — so we check for existence and fall back to the source tree.
            let bin = "stockfish-x86_64-pc-windows-msvc.exe";
            let resource_attempt = app
                .path()
                .resolve(format!("binaries/{bin}"), tauri::path::BaseDirectory::Resource)
                .ok()
                .filter(|p| p.exists());
            let path = resource_attempt.unwrap_or_else(|| {
                let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                p.push("binaries");
                p.push(bin);
                p
            });
            eprintln!("[chess-engine-app] Stockfish path: {}", path.display());
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
            commands::reset_engine,
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
