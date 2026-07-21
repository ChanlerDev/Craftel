use std::{
    sync::{Mutex, mpsc},
    thread::JoinHandle,
};

use craftel_core::{CraftelService, app_paths};

use crate::commands::IpcError;

pub struct AppState {
    pub(crate) service: Mutex<CraftelService>,
    dispatcher: Option<(mpsc::Sender<()>, JoinHandle<()>)>,
}

impl AppState {
    pub fn open() -> Result<Self, IpcError> {
        let path = app_paths::database_path().map_err(IpcError::from_display)?;
        Self::open_at(&path)
    }

    pub(crate) fn open_at(path: &std::path::Path) -> Result<Self, IpcError> {
        Ok(Self {
            service: Mutex::new(CraftelService::open(path).map_err(IpcError::from_display)?),
            dispatcher: None,
        })
    }

    pub(crate) fn start_document_dispatcher(&mut self, app: tauri::AppHandle) {
        use tauri::Emitter;
        let receiver = self
            .service
            .get_mut()
            .ok()
            .and_then(CraftelService::subscribe_document_changes);
        let Some(receiver) = receiver else { return };
        let (stop, stopped) = mpsc::channel();
        let join = std::thread::spawn(move || {
            loop {
                if stopped.try_recv().is_ok() {
                    break;
                }
                match receiver.recv_timeout(std::time::Duration::from_millis(50)) {
                    Ok(hint) => {
                        let _ = app.emit("document_changed", hint);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });
        self.dispatcher = Some((stop, join));
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        if let Some((stop, join)) = self.dispatcher.take() {
            let _ = stop.send(());
            let _ = join.join();
        }
    }
}
