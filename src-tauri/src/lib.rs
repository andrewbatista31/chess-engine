mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::legal_moves,
            commands::make_move,
            commands::validate_fen,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
