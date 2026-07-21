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
            let mut state = AppState::open().map_err(|error| error.message)?;
            state.start_document_dispatcher(app.handle().clone());
            app.manage(state);
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
            list_documents,
            document_status,
            read_document,
            search_documents,
            write_document,
            list_document_revisions,
            restore_document_revision,
            start_current_phase,
            stop_run,
            follow_up,
            get_session,
            list_sessions,
            list_runs,
            list_active_runs,
            get_run,
            list_run_events,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CRAFTEL");
}
