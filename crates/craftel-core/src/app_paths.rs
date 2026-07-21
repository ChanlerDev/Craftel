use std::{env, fs, path::PathBuf};

use crate::storage::StorageError;

pub fn database_path() -> Result<PathBuf, StorageError> {
    let path = if let Some(override_path) = env::var_os("CRAFTEL_DB_PATH") {
        PathBuf::from(override_path)
    } else {
        dirs::data_dir()
            .ok_or(StorageError::ApplicationDataDirectoryUnavailable)?
            .join("CRAFTEL")
            .join("craftel.sqlite3")
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(path)
}
