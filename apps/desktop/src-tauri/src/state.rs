use std::{
    sync::{Mutex, mpsc},
    thread::JoinHandle,
};

use craftel_core::{CraftelService, app_paths, run_service::RunService};

use crate::commands::IpcError;

pub struct AppState {
    pub(crate) service: Mutex<CraftelService>,
    pub(crate) runs: Mutex<RunService>,
    dispatcher: Option<(mpsc::Sender<()>, JoinHandle<()>)>,
}

impl AppState {
    pub fn open() -> Result<Self, IpcError> {
        let path = app_paths::database_path().map_err(IpcError::from_display)?;
        Self::open_at(&path)
    }

    pub(crate) fn open_at(path: &std::path::Path) -> Result<Self, IpcError> {
        let cursor =
            std::env::var_os("CRAFTEL_CURSOR_EXECUTABLE").unwrap_or_else(|| "agent".into());
        Ok(Self {
            service: Mutex::new(CraftelService::open(path).map_err(IpcError::from_display)?),
            runs: Mutex::new(RunService::open(path, cursor).map_err(IpcError::from_display)?),
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
        let run_receiver = self.runs.get_mut().ok().and_then(|runs| runs.subscribe());
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
                if let Some(run_receiver) = &run_receiver {
                    while let Ok(notice) = run_receiver.try_recv() {
                        match notice {
                            craftel_core::run_service::RunNotice::Event {
                                run_id,
                                last_persisted_sequence,
                            } => {
                                let _=app.emit("run_event",serde_json::json!({"run_id":run_id,"last_persisted_sequence":last_persisted_sequence}));
                            }
                            craftel_core::run_service::RunNotice::Changed { run_id } => {
                                let _ =
                                    app.emit("run_changed", serde_json::json!({"run_id":run_id}));
                            }
                        }
                    }
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
        if let Ok(runs) = self.runs.get_mut() {
            runs.shutdown();
        }
    }
}
