use std::sync::Mutex;

use craftel_core::{CraftelService, app_paths};

use crate::commands::IpcError;

pub struct AppState {
    pub(crate) service: Mutex<CraftelService>,
}

impl AppState {
    pub fn open() -> Result<Self, IpcError> {
        let path = app_paths::database_path().map_err(IpcError::from_display)?;
        Self::open_at(&path)
    }

    pub(crate) fn open_at(path: &std::path::Path) -> Result<Self, IpcError> {
        Ok(Self {
            service: Mutex::new(CraftelService::open(path).map_err(IpcError::from_display)?),
        })
    }
}
