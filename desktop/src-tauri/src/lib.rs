mod commands;
mod state;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::load_image,
            commands::run_matrix,
            commands::cancel_run,
            commands::get_candidate_svg,
            commands::render_full,
            commands::export_result,
            commands::save_recipe,
            commands::load_recipe,
            commands::save_session,
            commands::load_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running VTracer Studio");
}
