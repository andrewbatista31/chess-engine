mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::legal_moves,
            commands::make_move,
            commands::validate_fen,
            commands::parse_pgn,
            commands::serialize_pgn,
            commands::save_pgn_file,
            commands::load_pgn_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
