use super::{
    DocumentCause, DocumentChanged, DocumentError, DocumentRepository,
    indexer::{process_path, reconcile_project_with_cause},
};
use crate::domain::Project;
use notify::{
    EventKind, RecursiveMode, Watcher,
    event::{ModifyKind, RenameMode},
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
#[derive(Clone, Copy)]
enum PendingAction {
    Upsert,
    Remove,
}
pub struct ProjectWatcher {
    stop: mpsc::Sender<()>,
    join: Option<JoinHandle<()>>,
}
impl ProjectWatcher {
    pub fn start(db: PathBuf, p: Project) -> Result<Self, DocumentError> {
        let (notifications, _subscription) = mpsc::sync_channel(1);
        Self::start_with_notifications(db, p, notifications)
    }
    pub(crate) fn start_with_notifications(
        db: PathBuf,
        p: Project,
        notifications: mpsc::SyncSender<DocumentChanged>,
    ) -> Result<Self, DocumentError> {
        let (stx, srx) = mpsc::channel();
        let (rtx, rrx) = mpsc::channel();
        let join = thread::spawn(move || {
            let (tx, rx) = mpsc::channel();
            let mut w = match notify::recommended_watcher(tx) {
                Ok(v) => v,
                Err(e) => {
                    let _ = rtx.send(Err(e.to_string()));
                    return;
                }
            };
            if let Err(e) = w.watch(&p.work_dir.join("craftel"), RecursiveMode::Recursive) {
                let _ = rtx.send(Err(e.to_string()));
                return;
            }
            let _ = rtx.send(Ok(()));
            let mut due: HashMap<PathBuf, (Instant, PendingAction)> = HashMap::new();
            loop {
                if srx.try_recv().is_ok() {
                    break;
                }
                match rx.recv_timeout(Duration::from_millis(25)) {
                    Ok(Ok(event)) => {
                        let deadline = Instant::now() + Duration::from_millis(250);
                        match event.kind {
                            EventKind::Create(_)
                            | EventKind::Modify(ModifyKind::Data(_))
                            | EventKind::Modify(ModifyKind::Metadata(_))
                            | EventKind::Modify(ModifyKind::Any) => {
                                for path in event.paths {
                                    due.insert(path, (deadline, PendingAction::Upsert));
                                }
                            }
                            EventKind::Remove(_) => {
                                for path in event.paths {
                                    due.insert(path, (deadline, PendingAction::Remove));
                                }
                            }
                            EventKind::Modify(ModifyKind::Name(RenameMode::Both))
                                if event.paths.len() >= 2 =>
                            {
                                due.insert(
                                    event.paths[0].clone(),
                                    (deadline, PendingAction::Remove),
                                );
                                due.insert(
                                    event.paths[1].clone(),
                                    (deadline, PendingAction::Upsert),
                                );
                            }
                            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                                for path in event.paths {
                                    due.insert(path, (deadline, PendingAction::Remove));
                                }
                            }
                            EventKind::Modify(ModifyKind::Name(_)) => {
                                for path in event.paths {
                                    due.insert(path, (deadline, PendingAction::Upsert));
                                }
                            }
                            EventKind::Modify(_) => {
                                for path in event.paths {
                                    due.insert(path, (deadline, PendingAction::Upsert));
                                }
                            }
                            EventKind::Other | EventKind::Any => dispatch(
                                &notifications,
                                &db,
                                &p.id,
                                reconcile_project_with_cause(&db, &p, DocumentCause::Watch),
                            ),
                            _ => {}
                        }
                    }
                    Ok(Err(_)) => dispatch(
                        &notifications,
                        &db,
                        &p.id,
                        reconcile_project_with_cause(&db, &p, DocumentCause::Watch),
                    ),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let now = Instant::now();
                let ready: Vec<_> = due
                    .iter()
                    .filter(|(_, (deadline, _))| now >= *deadline)
                    .map(|(path, (_, action))| (path.clone(), *action))
                    .collect();
                for (path, action) in ready {
                    due.remove(&path);
                    dispatch(
                        &notifications,
                        &db,
                        &p.id,
                        process_path(&db, &p, &path, matches!(action, PendingAction::Remove)),
                    );
                }
            }
        });
        match rrx.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(())) => Ok(Self {
                stop: stx,
                join: Some(join),
            }),
            result => {
                // A failed or timed-out start still owns a thread. Signal and join it
                // before returning so service startup can never leak a watcher.
                let _ = stx.send(());
                let _ = join.join();
                match result {
                    Ok(Err(e)) => Err(DocumentError::Io(std::io::Error::other(e))),
                    Err(_) => Err(DocumentError::Io(std::io::Error::other(
                        "watcher startup timeout",
                    ))),
                    Ok(Ok(())) => unreachable!(),
                }
            }
        }
    }
    pub fn shutdown(mut self) -> Result<(), DocumentError> {
        let _ = self.stop.send(());
        if self.join.take().unwrap().join().is_err() {
            return Err(DocumentError::Io(std::io::Error::other("watcher panicked")));
        }
        Ok(())
    }
}
fn dispatch(
    sender: &mpsc::SyncSender<DocumentChanged>,
    database: &std::path::Path,
    project: &str,
    result: Result<Vec<DocumentChanged>, DocumentError>,
) {
    match result {
        Ok(changes) => {
            DocumentRepository::record_status(project, database, None);
            for change in changes {
                // Notifications are lossy hints. Durable ingest must never wait for a client.
                let _ = sender.try_send(change);
            }
        }
        Err(error) => {
            DocumentRepository::record_status(project, database, Some(&error.to_string()))
        }
    }
}
impl Drop for ProjectWatcher {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}
