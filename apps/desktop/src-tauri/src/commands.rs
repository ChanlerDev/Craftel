use std::{fmt::Display, path::PathBuf};

use craftel_core::domain::{Project, Stage, Task};
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IpcError {
    pub message: String,
}

impl IpcError {
    pub(crate) fn from_display(error: impl Display) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

fn with_service<T>(
    state: &AppState,
    operation: impl FnOnce(&mut craftel_core::CraftelService) -> Result<T, craftel_core::ServiceError>,
) -> Result<T, IpcError> {
    let mut service = state
        .service
        .lock()
        .map_err(|_| IpcError::from_display("CRAFTEL service state is unavailable"))?;
    operation(&mut service).map_err(IpcError::from_display)
}

#[tauri::command]
pub fn register_project(
    state: State<'_, AppState>,
    name: String,
    path: PathBuf,
) -> Result<Project, IpcError> {
    with_service(&state, |service| service.register_project(&name, &path))
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>, IpcError> {
    with_service(&state, |service| service.list_projects())
}

#[tauri::command]
pub fn open_project(state: State<'_, AppState>, id: String) -> Result<Project, IpcError> {
    with_service(&state, |service| service.open_project(&id))
}

#[tauri::command]
pub fn remove_project(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    with_service(&state, |service| service.remove_project(&id))
}

#[tauri::command]
pub fn create_task(
    state: State<'_, AppState>,
    project_id: String,
    title: String,
    content: String,
) -> Result<Task, IpcError> {
    with_service(&state, |service| {
        service.create_task(&project_id, &title, &content)
    })
}

#[tauri::command]
pub fn list_tasks(state: State<'_, AppState>, project_id: String) -> Result<Vec<Task>, IpcError> {
    with_service(&state, |service| service.list_tasks(&project_id))
}

#[tauri::command]
pub fn update_task(
    state: State<'_, AppState>,
    project_id: String,
    task_id: String,
    title: String,
    content: String,
) -> Result<Task, IpcError> {
    with_service(&state, |service| {
        service.update_task(&project_id, &task_id, &title, &content)
    })
}

#[tauri::command]
pub fn move_task(
    state: State<'_, AppState>,
    project_id: String,
    task_id: String,
    stage: Stage,
) -> Result<Task, IpcError> {
    with_service(&state, |service| {
        service.move_task(&project_id, &task_id, stage)
    })
}

macro_rules! transition_command {
    ($name:ident) => {
        #[tauri::command]
        pub fn $name(
            state: State<'_, AppState>,
            project_id: String,
            task_id: String,
        ) -> Result<Task, IpcError> {
            with_service(&state, |service| service.$name(&project_id, &task_id))
        }
    };
}

transition_command!(next_task);
transition_command!(pass_task);
transition_command!(fail_task);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_helpers_delegate_to_the_core_service() {
        let temp = tempfile::tempdir().unwrap();
        let state = AppState::open_at(&temp.path().join("craftel.sqlite3")).unwrap();
        let project_dir = temp.path().join("project");
        std::fs::create_dir(&project_dir).unwrap();

        let project = with_service(&state, |service| {
            service.register_project("Demo", &project_dir)
        })
        .unwrap();
        let task = with_service(&state, |service| {
            service.create_task(&project.id, "Typed IPC", "Exercise commands")
        })
        .unwrap();
        let moved =
            with_service(&state, |service| service.next_task(&project.id, &task.id)).unwrap();

        assert_eq!(moved.stage, Stage::Defining);
    }

    #[test]
    fn ipc_errors_are_serializable_messages() {
        let error = IpcError::from_display("safe message");
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            serde_json::json!({"message": "safe message"})
        );
    }
}
