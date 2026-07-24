use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use craftel_core::{
    documents::{Document, DocumentProjectStatus, DocumentSnapshot, ExpectedDocumentState},
    domain::{Project, Stage, Task},
    git_summary::GitWorkingCopySummary,
    runs::{PhaseSession, Run, RunEvent},
};
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcErrorCode {
    Conflict,
    Unavailable,
    NotFound,
    InvalidPath,
    InvalidUtf8,
    Validation,
    Io,
}

fn with_runs<T>(
    state: &AppState,
    operation: impl FnOnce(
        &mut craftel_core::run_service::RunService,
    ) -> Result<T, craftel_core::run_service::RunServiceError>,
) -> Result<T, IpcError> {
    let mut runs = state
        .runs
        .lock()
        .map_err(|_| IpcError::from_display("run supervisor unavailable"))?;
    operation(&mut runs).map_err(IpcError::from_display)
}

#[tauri::command]
pub fn start_current_phase(
    state: State<'_, AppState>,
    project_id: String,
    task_id: String,
) -> Result<Run, IpcError> {
    with_runs(&state, |s| s.start_current_phase(&project_id, &task_id))
}
#[tauri::command]
pub fn stop_run(state: State<'_, AppState>, run_id: String) -> Result<Run, IpcError> {
    with_runs(&state, |s| s.stop_run(&run_id))
}
#[tauri::command]
pub fn follow_up(
    state: State<'_, AppState>,
    session_id: String,
    prompt: String,
) -> Result<Run, IpcError> {
    with_runs(&state, |s| s.follow_up(&session_id, &prompt))
}
#[tauri::command]
pub fn get_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<PhaseSession, IpcError> {
    with_runs(&state, |s| s.get_session(&session_id))
}
#[tauri::command]
pub fn list_sessions(
    state: State<'_, AppState>,
    project_id: String,
    task_id: String,
) -> Result<Vec<PhaseSession>, IpcError> {
    with_runs(&state, |s| s.list_sessions(&project_id, &task_id))
}
#[tauri::command]
pub fn list_runs(state: State<'_, AppState>, session_id: String) -> Result<Vec<Run>, IpcError> {
    with_runs(&state, |s| s.list_runs(&session_id))
}
#[tauri::command]
pub fn list_active_runs(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<Run>, IpcError> {
    with_runs(&state, |s| s.list_active_runs(&project_id))
}
#[tauri::command]
pub fn get_run(state: State<'_, AppState>, run_id: String) -> Result<Run, IpcError> {
    with_runs(&state, |s| s.get_run(&run_id))
}
#[tauri::command]
pub fn list_run_events(
    state: State<'_, AppState>,
    run_id: String,
    after_sequence: i64,
    limit: usize,
) -> Result<Vec<RunEvent>, IpcError> {
    with_runs(&state, |s| {
        s.list_run_events(&run_id, after_sequence, limit)
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IpcError {
    pub message: String,
    pub code: IpcErrorCode,
}

impl IpcError {
    pub(crate) fn from_display(error: impl Display) -> Self {
        Self {
            message: error.to_string(),
            code: IpcErrorCode::Io,
        }
    }
    fn from_service(error: craftel_core::ServiceError) -> Self {
        use craftel_core::{ServiceError, documents::DocumentError};
        let code = match &error {
            ServiceError::Conflict | ServiceError::Document(DocumentError::Conflict) => {
                IpcErrorCode::Conflict
            }
            ServiceError::Unavailable => IpcErrorCode::Unavailable,
            ServiceError::Document(DocumentError::NotFound)
            | ServiceError::Storage(craftel_core::storage::StorageError::NotFound) => {
                IpcErrorCode::NotFound
            }
            ServiceError::Document(DocumentError::InvalidPath) => IpcErrorCode::InvalidPath,
            ServiceError::Document(DocumentError::InvalidUtf8) => IpcErrorCode::InvalidUtf8,
            ServiceError::Validation(_) => IpcErrorCode::Validation,
            _ => IpcErrorCode::Io,
        };
        Self {
            message: error.to_string(),
            code,
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
    operation(&mut service).map_err(IpcError::from_service)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: String,
    pub hidden: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DirectoryListing {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<DirectoryEntry>,
}

fn directory_listing(path: Option<&Path>) -> Result<DirectoryListing, IpcError> {
    let requested = match path {
        Some(path) => path.to_path_buf(),
        None => dirs::home_dir()
            .ok_or_else(|| IpcError::from_display("The home directory is unavailable"))?,
    };
    if !requested.is_absolute() {
        return Err(IpcError {
            message: "Enter an absolute folder path".into(),
            code: IpcErrorCode::InvalidPath,
        });
    }
    let metadata = std::fs::symlink_metadata(&requested).map_err(|error| IpcError {
        message: format!("The folder cannot be opened: {error}"),
        code: IpcErrorCode::InvalidPath,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(IpcError {
            message: "The path must identify a directory, not a file or symbolic link".into(),
            code: IpcErrorCode::InvalidPath,
        });
    }
    let canonical = dunce::canonicalize(&requested).map_err(|error| IpcError {
        message: format!("The folder cannot be resolved: {error}"),
        code: IpcErrorCode::InvalidPath,
    })?;
    let path = canonical.to_str().ok_or_else(|| IpcError {
        message: "The folder path is not valid UTF-8".into(),
        code: IpcErrorCode::InvalidUtf8,
    })?;
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&canonical).map_err(|error| IpcError {
        message: format!("The folder cannot be read: {error}"),
        code: IpcErrorCode::Io,
    })? {
        let Ok(entry) = entry else { continue };
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let entry_path = entry.path();
        let Some(entry_path) = entry_path.to_str().map(str::to_owned) else {
            continue;
        };
        entries.push(DirectoryEntry {
            hidden: name.starts_with('.'),
            name,
            path: entry_path,
        });
    }
    entries.sort_by_cached_key(|entry| entry.name.to_lowercase());
    let parent = canonical.parent().and_then(Path::to_str).map(str::to_owned);
    Ok(DirectoryListing {
        path: path.to_owned(),
        parent,
        entries,
    })
}

#[tauri::command]
pub fn list_directory(path: Option<PathBuf>) -> Result<DirectoryListing, IpcError> {
    directory_listing(path.as_deref())
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
pub fn git_working_copy_summary(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<GitWorkingCopySummary, IpcError> {
    let work_dir = with_service(&state, |service| service.project_work_dir(&project_id))?;
    craftel_core::git_summary::working_copy_summary(&work_dir).map_err(IpcError::from_display)
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

#[tauri::command]
pub fn list_documents(
    state: State<'_, AppState>,
    project_id: String,
    include_deleted: bool,
) -> Result<Vec<Document>, IpcError> {
    with_service(&state, |s| s.list_documents(&project_id, include_deleted))
}
#[tauri::command]
pub fn document_status(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<DocumentProjectStatus, IpcError> {
    with_service(&state, |s| s.document_status(&project_id))
}
#[tauri::command]
pub fn read_document(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<Document, IpcError> {
    with_service(&state, |s| s.read_document(&project_id, &path))
}
#[tauri::command]
pub fn search_documents(
    state: State<'_, AppState>,
    project_id: String,
    query: String,
) -> Result<Vec<Document>, IpcError> {
    with_service(&state, |s| s.search_documents(&project_id, &query))
}
#[tauri::command]
pub fn list_document_revisions(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<Vec<DocumentSnapshot>, IpcError> {
    with_service(&state, |s| s.list_document_revisions(&project_id, &path))
}
#[tauri::command]
pub fn write_document(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
    content: String,
    expected_state: ExpectedDocumentState,
) -> Result<Document, IpcError> {
    let d = with_service(&state, |s| {
        s.write_document(&project_id, &path, &content, expected_state)
    })?;
    Ok(d)
}
#[tauri::command]
pub fn restore_document_revision(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
    snapshot_id: String,
    expected_state: ExpectedDocumentState,
) -> Result<Document, IpcError> {
    let d = with_service(&state, |s| {
        s.restore_document_revision(&project_id, &path, &snapshot_id, expected_state)
    })?;
    Ok(d)
}

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
            serde_json::json!({"message": "safe message", "code": "io"})
        );
    }

    #[test]
    fn directory_listing_returns_only_real_directories_in_name_order() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("beta")).unwrap();
        std::fs::create_dir(temp.path().join("Alpha")).unwrap();
        std::fs::create_dir(temp.path().join(".hidden")).unwrap();
        std::fs::write(temp.path().join("file.txt"), "ignored").unwrap();

        let listing = directory_listing(Some(temp.path())).unwrap();

        assert_eq!(
            listing.path,
            temp.path().canonicalize().unwrap().to_str().unwrap()
        );
        assert_eq!(
            listing
                .entries
                .iter()
                .map(|entry| (entry.name.as_str(), entry.hidden))
                .collect::<Vec<_>>(),
            [(".hidden", true), ("Alpha", false), ("beta", false)]
        );
        assert_eq!(
            listing.parent.as_deref(),
            temp.path()
                .canonicalize()
                .unwrap()
                .parent()
                .unwrap()
                .to_str()
        );
    }

    #[test]
    fn directory_listing_rejects_relative_files_and_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("file.txt");
        std::fs::write(&file, "not a directory").unwrap();
        assert_eq!(
            directory_listing(Some(Path::new("relative")))
                .unwrap_err()
                .code,
            IpcErrorCode::InvalidPath
        );
        assert_eq!(
            directory_listing(Some(&file)).unwrap_err().code,
            IpcErrorCode::InvalidPath
        );

        #[cfg(unix)]
        {
            let link = temp.path().join("link");
            std::os::unix::fs::symlink(temp.path(), &link).unwrap();
            assert_eq!(
                directory_listing(Some(&link)).unwrap_err().code,
                IpcErrorCode::InvalidPath
            );
            assert!(
                directory_listing(Some(temp.path()))
                    .unwrap()
                    .entries
                    .iter()
                    .all(|entry| entry.name != "link")
            );
        }
    }

    #[test]
    fn git_summary_delegates_and_has_the_desktop_wire_shape() {
        let temp = tempfile::tempdir().unwrap();
        let state = AppState::open_at(&temp.path().join("craftel.sqlite3")).unwrap();
        let project_dir = temp.path().join("project");
        std::fs::create_dir(&project_dir).unwrap();
        let project = with_service(&state, |service| {
            service.register_project("Demo", &project_dir)
        })
        .unwrap();
        let summary = with_service(&state, |service| {
            service.git_working_copy_summary(&project.id)
        })
        .unwrap();
        assert_eq!(
            serde_json::to_value(summary).unwrap(),
            serde_json::json!({
                "is_repository": false, "branch": null, "latest_commit": null,
                "staged_diff": "", "unstaged_diff": "", "untracked_paths": [],
                "truncated": false
            })
        );
    }

    #[test]
    fn document_change_hint_has_the_typed_desktop_shape() {
        let hint = craftel_core::documents::DocumentChanged {
            project_id: "P1".into(),
            path: "craftel/INDEX.md".into(),
            change: craftel_core::documents::DocumentChange::Delete,
        };
        assert_eq!(
            serde_json::to_value(hint).unwrap(),
            serde_json::json!({
                "project_id": "P1", "path": "craftel/INDEX.md", "change": "delete"
            })
        );
    }

    #[test]
    fn expected_document_states_have_the_desktop_wire_shape() {
        assert_eq!(
            serde_json::to_value(ExpectedDocumentState::Missing).unwrap(),
            serde_json::json!({"state": "missing"})
        );
        assert_eq!(
            serde_json::to_value(ExpectedDocumentState::Present("abc".into())).unwrap(),
            serde_json::json!({"state": "present", "hash": "abc"})
        );
    }

    #[test]
    fn run_event_has_the_typed_stream_wire_shape() {
        let expected = serde_json::json!({
            "run_id":"R1", "sequence":7, "kind":"tool_complete",
            "event_at":"2026-07-21T12:00:00Z", "display_text":"done",
            "tool_name":"Shell", "tool_call_id":"call-1", "raw_json":"{}"
        });
        let event: RunEvent = serde_json::from_value(expected.clone()).unwrap();
        assert_eq!(serde_json::to_value(event).unwrap(), expected);
    }
}
