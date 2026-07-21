use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("storage operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid stored value: {0}")]
    InvalidData(String),
    #[error("project or task was not found")]
    NotFound,
    #[error("task has an active run")]
    ActiveRun,
    #[error("application data directory is unavailable")]
    ApplicationDataDirectoryUnavailable,
    #[error(transparent)]
    Workflow(#[from] crate::domain::WorkflowError),
}
