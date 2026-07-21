mod commands;
mod state;

use commands::*;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            use tauri::Manager;
            app.manage(AppState::open().map_err(|error| error.message)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            register_project,
            list_projects,
            open_project,
            remove_project,
            create_task,
            list_tasks,
            update_task,
            move_task,
            next_task,
            pass_task,
            fail_task,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CRAFTEL");
}
