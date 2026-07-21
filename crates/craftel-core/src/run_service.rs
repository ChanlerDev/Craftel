use crate::{
    automation::build_prompt,
    harness::{CursorHarness, NdjsonParser, ParsedEvent, append_bounded},
    runs::{Phase, PhaseSession, Run, RunError, RunEvent, RunRepository, RunState},
};
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
    process::{Child, ExitStatus},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use thiserror::Error;

const STOP_GRACE: Duration = Duration::from_secs(5);
const LEASE_TTL: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunNotice {
    Event {
        run_id: String,
        last_persisted_sequence: i64,
    },
    Changed {
        run_id: String,
    },
}

/// Recovery seam. Ownership must be established before any signal is sent.
pub trait ProcessInspector: Send + Sync {
    fn ownership(&self, pid: u32, token: &str) -> ProcessOwnership;
    fn terminate(&self, pid: u32);
    fn kill(&self, pid: u32) {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
    fn alive(&self, pid: u32) -> bool {
        #[cfg(unix)]
        {
            unsafe { libc::kill(pid as i32, 0) == 0 }
        }
        #[cfg(not(unix))]
        {
            let _ = pid;
            false
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessOwnership {
    Owned,
    NotOwned,
    Unknown,
}
pub struct SystemProcessInspector;
impl ProcessInspector for SystemProcessInspector {
    fn ownership(&self, pid: u32, token: &str) -> ProcessOwnership {
        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
            true,
            sysinfo::ProcessRefreshKind::nothing().with_environ(sysinfo::UpdateKind::Always),
        );
        match system.process(sysinfo::Pid::from_u32(pid)) {
            Some(process) => {
                let marker = std::ffi::OsString::from(format!("CRAFTEL_OWNERSHIP_TOKEN={token}"));
                if process.environ().iter().any(|value| value == &marker) {
                    ProcessOwnership::Owned
                } else {
                    ProcessOwnership::NotOwned
                }
            }
            None => ProcessOwnership::Unknown,
        }
    }
    fn terminate(&self, pid: u32) {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(pid as i32), libc::SIGTERM);
        }
    }
}

pub trait ControllerClock: Send + Sync {
    fn wait(&self, duration: Duration);
}
pub struct SystemControllerClock;
impl ControllerClock for SystemControllerClock {
    fn wait(&self, duration: Duration) {
        thread::sleep(duration)
    }
}
pub trait ProcessSignals: Send + Sync {
    fn term_group(&self, leader: u32);
    fn kill_group(&self, leader: u32);
}
pub struct SystemProcessSignals;
impl ProcessSignals for SystemProcessSignals {
    fn term_group(&self, leader: u32) {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(leader as i32), libc::SIGTERM);
        }
    }
    fn kill_group(&self, leader: u32) {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(leader as i32), libc::SIGKILL);
        }
    }
}
pub trait ProcessFactory: Send + Sync {
    fn version(&self) -> std::io::Result<String>;
    fn spawn(
        &self,
        prompt: &str,
        resume: Option<&str>,
        cwd: &Path,
        token: &str,
    ) -> std::io::Result<Child>;
}
impl ProcessFactory for CursorHarness {
    fn version(&self) -> std::io::Result<String> {
        CursorHarness::version(self)
    }
    fn spawn(&self, p: &str, r: Option<&str>, cwd: &Path, token: &str) -> std::io::Result<Child> {
        CursorHarness::spawn(self, p, r, cwd, token)
    }
}

#[derive(Default)]
struct NoticeBroker {
    subscribers: Vec<(mpsc::SyncSender<()>, Arc<Mutex<NoticeSnapshot>>)>,
    high_water: HashMap<String, i64>,
    changed: HashSet<String>,
}
#[derive(Default)]
struct NoticeSnapshot {
    high_water: HashMap<String, i64>,
    changed: HashSet<String>,
}
pub struct NoticeSubscription {
    wake: mpsc::Receiver<()>,
    pending: Arc<Mutex<NoticeSnapshot>>,
}
impl NoticeSubscription {
    pub fn try_recv(&self) -> Result<RunNotice, mpsc::TryRecvError> {
        let wake = self.wake.try_recv();
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| mpsc::TryRecvError::Disconnected)?;
        let notice = if let Some(id) = pending.changed.iter().next().cloned() {
            pending.changed.remove(&id);
            Some(RunNotice::Changed { run_id: id })
        } else if let Some((id, sequence)) = pending
            .high_water
            .iter()
            .next()
            .map(|(id, sequence)| (id.clone(), *sequence))
        {
            pending.high_water.remove(&id);
            Some(RunNotice::Event {
                run_id: id,
                last_persisted_sequence: sequence,
            })
        } else {
            None
        };
        drop(pending);
        if let Some(notice) = notice {
            return Ok(notice);
        }
        match wake {
            Err(mpsc::TryRecvError::Disconnected) => Err(mpsc::TryRecvError::Disconnected),
            _ => Err(mpsc::TryRecvError::Empty),
        }
    }
}
impl NoticeBroker {
    fn publish(&mut self, notice: RunNotice) {
        if let RunNotice::Event {
            run_id,
            last_persisted_sequence,
        } = &notice
        {
            self.high_water
                .entry(run_id.clone())
                .and_modify(|v| *v = (*v).max(*last_persisted_sequence))
                .or_insert(*last_persisted_sequence);
        }
        if let RunNotice::Changed { run_id } = &notice {
            self.changed.insert(run_id.clone());
        }
        self.subscribers.retain(|(wake, pending)| {
            let Ok(mut pending) = pending.lock() else {
                return false;
            };
            match &notice {
                RunNotice::Event {
                    run_id,
                    last_persisted_sequence,
                } => {
                    pending
                        .high_water
                        .entry(run_id.clone())
                        .and_modify(|v| *v = (*v).max(*last_persisted_sequence))
                        .or_insert(*last_persisted_sequence);
                }
                RunNotice::Changed { run_id } => {
                    pending.changed.insert(run_id.clone());
                }
            }
            drop(pending);
            !matches!(wake.try_send(()), Err(mpsc::TrySendError::Disconnected(_)))
        });
    }
    fn subscribe(&mut self) -> NoticeSubscription {
        let (tx, rx) = mpsc::sync_channel(1);
        let pending = Arc::new(Mutex::new(NoticeSnapshot {
            high_water: self.high_water.clone(),
            changed: self.changed.clone(),
        }));
        if !self.high_water.is_empty() || !self.changed.is_empty() {
            let _ = tx.try_send(());
        }
        self.subscribers.push((tx, pending.clone()));
        NoticeSubscription { wake: rx, pending }
    }
}
enum ControllerCommand {
    Stop(mpsc::SyncSender<Result<Run, RunError>>),
    Shutdown,
}
struct Controller {
    command: mpsc::Sender<ControllerCommand>,
    join: Option<JoinHandle<()>>,
}

#[derive(Debug, Error)]
pub enum RunServiceError {
    #[error(transparent)]
    Run(#[from] RunError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Policy(String),
    #[error("run supervisor unavailable")]
    Poisoned,
    #[error("another run supervisor owns this database")]
    LeaseHeld,
    #[error("run supervisor is non-operational: {0}")]
    NonOperational(String),
}

pub struct RunService {
    database: PathBuf,
    factory: Arc<dyn ProcessFactory>,
    clock: Arc<dyn ControllerClock>,
    signals: Arc<dyn ProcessSignals>,
    controllers: Arc<Mutex<HashMap<String, Controller>>>,
    broker: Arc<Mutex<NoticeBroker>>,
    inspector: Arc<dyn ProcessInspector>,
    lease_owner: String,
    heartbeat_stop: Option<mpsc::Sender<()>>,
    heartbeat: Option<JoinHandle<()>>,
    operational: Arc<AtomicBool>,
    terminal_error: Arc<Mutex<Option<String>>>,
}
impl RunService {
    pub fn open(database: &Path, executable: impl Into<PathBuf>) -> Result<Self, RunServiceError> {
        Self::open_with_inspector(database, executable, Arc::new(SystemProcessInspector))
    }
    pub fn open_with_inspector(
        database: &Path,
        executable: impl Into<PathBuf>,
        inspector: Arc<dyn ProcessInspector>,
    ) -> Result<Self, RunServiceError> {
        Self::open_with_seams(
            database,
            Arc::new(CursorHarness::new(executable)),
            inspector,
            Arc::new(SystemControllerClock),
            Arc::new(SystemProcessSignals),
        )
    }
    pub fn open_with_seams(
        database: &Path,
        factory: Arc<dyn ProcessFactory>,
        inspector: Arc<dyn ProcessInspector>,
        clock: Arc<dyn ControllerClock>,
        signals: Arc<dyn ProcessSignals>,
    ) -> Result<Self, RunServiceError> {
        let owner = uuid::Uuid::new_v4().to_string();
        let mut repo = RunRepository::open(database)?;
        if !repo.acquire_supervisor_lease(&owner, LEASE_TTL)? {
            return Err(RunServiceError::LeaseHeld);
        }
        let (stop_tx, stop_rx) = mpsc::channel();
        let db = database.to_path_buf();
        let heartbeat_owner = owner.clone();
        let controllers = Arc::new(Mutex::new(HashMap::<String, Controller>::new()));
        let operational = Arc::new(AtomicBool::new(true));
        let terminal_error = Arc::new(Mutex::new(None));
        let heartbeat_operational = operational.clone();
        let heartbeat_controllers = controllers.clone();
        let heartbeat = thread::spawn(move || {
            let mut last_confirmed = std::time::Instant::now();
            loop {
                if stop_rx.recv_timeout(Duration::from_secs(1)).is_ok() {
                    break;
                }
                match RunRepository::open(&db)
                    .and_then(|mut r| r.heartbeat_supervisor_lease(&heartbeat_owner, LEASE_TTL))
                {
                    Ok(true) => last_confirmed = std::time::Instant::now(),
                    Ok(false) => last_confirmed = std::time::Instant::now() - LEASE_TTL,
                    Err(_) => {} // transient: retry while the last confirmed lease remains valid
                }
                if last_confirmed.elapsed() >= LEASE_TTL {
                    heartbeat_operational.store(false, Ordering::Release);
                    if let Ok(all) = heartbeat_controllers.lock() {
                        for controller in all.values() {
                            let _ = controller.command.send(ControllerCommand::Shutdown);
                        }
                    }
                    break;
                }
            }
        });
        let mut service = Self {
            database: database.into(),
            factory,
            clock,
            signals,
            controllers,
            broker: Arc::new(Mutex::new(NoticeBroker::default())),
            inspector,
            lease_owner: owner,
            heartbeat_stop: Some(stop_tx),
            heartbeat: Some(heartbeat),
            operational,
            terminal_error,
        };
        if let Err(error) = service.recover_stale() {
            service.shutdown();
            return Err(error);
        }
        Ok(service)
    }
    pub fn subscribe(&self) -> Option<NoticeSubscription> {
        self.broker.lock().ok().map(|mut b| b.subscribe())
    }
    fn notice(&self, n: RunNotice) {
        if let Ok(mut b) = self.broker.lock() {
            b.publish(n)
        }
    }
    fn repo(&self) -> Result<RunRepository, RunServiceError> {
        Ok(RunRepository::open(&self.database)?)
    }
    fn ensure_operational(&self) -> Result<(), RunServiceError> {
        if !self.operational.load(Ordering::Acquire) {
            let detail = self
                .terminal_error
                .lock()
                .ok()
                .and_then(|value| value.clone());
            return Err(detail.map_or(RunServiceError::LeaseHeld, RunServiceError::NonOperational));
        }
        Ok(())
    }
    pub fn start_current_phase(&mut self, p: &str, t: &str) -> Result<Run, RunServiceError> {
        let connection = rusqlite::Connection::open(&self.database).map_err(RunError::Sql)?;
        let (stage, work): (String, String) = connection.query_row(
            "SELECT t.stage,p.work_dir FROM tasks t JOIN projects p ON p.id=t.project_id WHERE t.project_id=?1 AND t.id=?2",
            rusqlite::params![p,t], |r| Ok((r.get(0)?,r.get(1)?))).map_err(RunError::Sql)?;
        let phase = match stage.as_str() {
            "defining" => Phase::Defining,
            "implementation" => Phase::Implementation,
            "reviewing" => Phase::Reviewing,
            _ => {
                return Err(RunServiceError::Policy(format!(
                    "cannot start a run in {stage}"
                )));
            }
        };
        let prompt = build_prompt(phase, t, p, Path::new(&work));
        self.ensure_operational()?;
        let (session, run) = self.repo()?.reserve_phase_run(p, t, phase, &prompt)?;
        self.notice(RunNotice::Changed {
            run_id: run.id.clone(),
        });
        self.launch(run, session.external_session_id.as_deref())
    }
    pub fn follow_up(&mut self, sid: &str, prompt: &str) -> Result<Run, RunServiceError> {
        let mut repo = self.repo()?;
        let s = repo.get_session(sid)?;
        if s.phase == Phase::Reviewing {
            return Err(RunServiceError::Policy(
                "review sessions do not accept follow-up".into(),
            ));
        }
        let ext = s.external_session_id.clone().ok_or_else(|| {
            RunServiceError::Policy("external Cursor session ID is required".into())
        })?;
        let runs = repo.list_runs(sid)?;
        let last = runs
            .last()
            .ok_or_else(|| RunServiceError::Policy("follow-up requires a terminal run".into()))?;
        if matches!(last.state, RunState::Queued | RunState::Running) {
            return Err(RunServiceError::Policy(
                "follow-up requires a terminal run".into(),
            ));
        }
        self.ensure_operational()?;
        let run = repo.reserve_run(&s, prompt, &last.work_dir)?;
        self.launch(run, Some(&ext))
    }
    fn launch(&mut self, run: Run, resume: Option<&str>) -> Result<Run, RunServiceError> {
        let version = match self.factory.version() {
            Ok(v) => v,
            Err(e) => {
                self.repo()?.finish(
                    &run.id,
                    RunState::Failed,
                    None,
                    "",
                    None,
                    Some(&format!("Cursor version discovery failed: {e}")),
                )?;
                self.notice(RunNotice::Changed {
                    run_id: run.id.clone(),
                });
                return Err(e.into());
            }
        };
        let mut repo = self.repo()?;
        let launch = repo.preflight_and_spawn(&run.id, &version, || {
            self.factory
                .spawn(&run.prompt, resume, &run.work_dir, &run.ownership_token)
        });
        let (running, child) = match launch {
            Ok(value) => value,
            Err(RunError::Io(e)) => {
                self.repo()?.finish(
                    &run.id,
                    RunState::Failed,
                    None,
                    "",
                    None,
                    Some(&e.to_string()),
                )?;
                self.notice(RunNotice::Changed {
                    run_id: run.id.clone(),
                });
                return Err(e.into());
            }
            Err(e) => return Err(e.into()),
        };
        let (tx, rx) = mpsc::channel();
        let id = run.id.clone();
        let map = self.controllers.clone();
        let db = self.database.clone();
        let broker = self.broker.clone();
        let clock = self.clock.clone();
        let signals = self.signals.clone();
        let operational = self.operational.clone();
        let terminal_error = self.terminal_error.clone();
        let session = run.session_id.clone();
        let join = thread::spawn(move || {
            controller_loop(
                child,
                rx,
                &db,
                &id,
                &session,
                &broker,
                clock.as_ref(),
                signals.as_ref(),
                &operational,
                &terminal_error,
            )
        });
        self.controllers
            .lock()
            .map_err(|_| RunServiceError::Poisoned)?
            .insert(
                run.id.clone(),
                Controller {
                    command: tx,
                    join: Some(join),
                },
            );
        self.notice(RunNotice::Changed {
            run_id: run.id.clone(),
        });
        // Finished controllers are retained until stop/shutdown so their join is never detached.
        let _ = map;
        Ok(running)
    }
    pub fn stop_run(&mut self, id: &str) -> Result<Run, RunServiceError> {
        self.ensure_operational()?;
        let run = self.repo()?.get_run(id)?;
        if !matches!(run.state, RunState::Queued | RunState::Running) {
            return Ok(run);
        }
        if let Some(c) = self
            .controllers
            .lock()
            .map_err(|_| RunServiceError::Poisoned)?
            .get(id)
        {
            let (reply, result) = mpsc::sync_channel(1);
            c.command
                .send(ControllerCommand::Stop(reply))
                .map_err(|_| RunServiceError::Poisoned)?;
            return result
                .recv()
                .map_err(|_| RunServiceError::Poisoned)?
                .map_err(Into::into);
        }
        Ok(run)
    }
    pub fn shutdown(&mut self) {
        let mut all = match self.controllers.lock() {
            Ok(mut c) => std::mem::take(&mut *c),
            Err(_) => HashMap::new(),
        };
        for c in all.values() {
            let _ = c.command.send(ControllerCommand::Shutdown);
        }
        for c in all.values_mut() {
            if let Some(j) = c.join.take() {
                let _ = j.join();
            }
        }
        if let Some(tx) = self.heartbeat_stop.take() {
            let _ = tx.send(());
        }
        if let Some(j) = self.heartbeat.take() {
            let _ = j.join();
        }
        let _ = self.repo().and_then(|mut r| {
            r.release_supervisor_lease(&self.lease_owner)
                .map_err(Into::into)
        });
    }
    pub fn get_session(&self, id: &str) -> Result<PhaseSession, RunServiceError> {
        Ok(self.repo()?.get_session(id)?)
    }
    pub fn list_sessions(&self, p: &str, t: &str) -> Result<Vec<PhaseSession>, RunServiceError> {
        Ok(self.repo()?.list_sessions(p, t)?)
    }
    pub fn list_runs(&self, s: &str) -> Result<Vec<Run>, RunServiceError> {
        Ok(self.repo()?.list_runs(s)?)
    }
    pub fn list_active_runs(&self, project: &str) -> Result<Vec<Run>, RunServiceError> {
        Ok(self.repo()?.list_active_runs(project)?)
    }
    pub fn get_run(&self, id: &str) -> Result<Run, RunServiceError> {
        Ok(self.repo()?.get_run(id)?)
    }
    pub fn list_run_events(
        &self,
        id: &str,
        after: i64,
        limit: usize,
    ) -> Result<Vec<RunEvent>, RunServiceError> {
        Ok(self.repo()?.list_events(id, after, limit)?)
    }
    fn recover_stale(&mut self) -> Result<(), RunServiceError> {
        let mut repo = self.repo()?;
        for run in repo.stale_runs()? {
            if run.state == RunState::Running
                && let Some(pid) = run.pid
                && self.inspector.ownership(pid, &run.ownership_token) == ProcessOwnership::Owned
            {
                self.inspector.terminate(pid);
                self.clock.wait(STOP_GRACE);
                if self.inspector.alive(pid)
                    && self.inspector.ownership(pid, &run.ownership_token)
                        == ProcessOwnership::Owned
                {
                    self.inspector.kill(pid);
                    self.clock.wait(STOP_GRACE);
                }
                if self.inspector.alive(pid) {
                    return Err(RunServiceError::Policy(format!(
                        "could not confirm process {pid} exited during recovery"
                    )));
                }
            }
            repo.finish_with_transition(
                &run.id,
                RunState::Interrupted,
                None,
                &run.stderr,
                run.final_result.as_deref(),
                Some("interrupted during startup recovery"),
            )?;
            self.notice(RunNotice::Changed { run_id: run.id });
        }
        Ok(())
    }
}
impl Drop for RunService {
    fn drop(&mut self) {
        self.shutdown()
    }
}

#[allow(clippy::too_many_arguments)]
fn controller_loop(
    mut child: Child,
    commands: mpsc::Receiver<ControllerCommand>,
    db: &Path,
    id: &str,
    session: &str,
    broker: &Arc<Mutex<NoticeBroker>>,
    clock: &dyn ControllerClock,
    signals: &dyn ProcessSignals,
    operational: &Arc<AtomicBool>,
    terminal_error: &Arc<Mutex<Option<String>>>,
) {
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let (out_tx, out_rx) = mpsc::channel();
    let (err_tx, err_rx) = mpsc::channel();
    enum ReaderMessage<T> {
        Item(T),
        Eof,
        Error(String),
    }
    let stdout_reader = thread::spawn(move || {
        let mut p = NdjsonParser::default();
        let mut b = [0; 8192];
        loop {
            match stdout.read(&mut b) {
                Ok(0) => {
                    let _ = out_tx.send(ReaderMessage::Eof);
                    break;
                }
                Ok(n) => {
                    for e in p.push(&b[..n]) {
                        if out_tx.send(ReaderMessage::Item(e)).is_err() {
                            return;
                        }
                    }
                }
                Err(error) => {
                    let _ =
                        out_tx.send(ReaderMessage::Error(format!("stdout read failed: {error}")));
                    break;
                }
            }
        }
    });
    let stderr_reader = thread::spawn(move || {
        let mut all = Vec::new();
        let mut b = [0; 8192];
        loop {
            match stderr.read(&mut b) {
                Ok(0) => {
                    let _ = err_tx.send(ReaderMessage::Eof);
                    break;
                }
                Ok(n) => {
                    append_bounded(&mut all, &b[..n]);
                    let _ = err_tx.send(ReaderMessage::Item(std::mem::take(&mut all)));
                }
                Err(error) => {
                    let _ =
                        err_tx.send(ReaderMessage::Error(format!("stderr read failed: {error}")));
                    break;
                }
            }
        }
    });
    let mut user_stopped = false;
    let mut shutdown = false;
    let mut persistence_error = None;
    let mut final_result = None;
    let status: Result<ExitStatus, std::io::Error> = loop {
        loop {
            match out_rx.try_recv() {
                Ok(ReaderMessage::Item(event)) => {
                    match persist_event(db, id, session, &event, broker) {
                        Ok(()) => {
                            // Final output is only promoted after its event transaction commits.
                            if event.final_result.is_some() {
                                final_result = event.final_result.clone()
                            }
                        }
                        Err(error) => {
                            persistence_error = Some(format!("event persistence failed: {error}"));
                            signals.term_group(child.id());
                        }
                    }
                }
                Ok(ReaderMessage::Error(error)) => {
                    persistence_error.get_or_insert(error);
                    signals.term_group(child.id());
                }
                Ok(ReaderMessage::Eof) | Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    persistence_error
                        .get_or_insert_with(|| "stdout reader disconnected without EOF".into());
                    break;
                }
            }
        }
        match child.try_wait() {
            Ok(Some(s)) => break Ok(s),
            Err(e) => break Err(e),
            Ok(None) => {}
        }
        match commands.recv_timeout(Duration::from_millis(10)) {
            Ok(ControllerCommand::Stop(reply)) if !user_stopped && !shutdown => {
                match child.try_wait() {
                    Ok(Some(s)) => {
                        let _ = reply.send(RunRepository::open(db).and_then(|r| r.get_run(id)));
                        break Ok(s);
                    }
                    Err(error) => {
                        let _ = reply.send(Err(RunError::Io(std::io::Error::new(
                            error.kind(),
                            error.to_string(),
                        ))));
                        break Err(error);
                    }
                    Ok(None) => {}
                }
                let marked =
                    RunRepository::open(db).and_then(|mut repo| repo.mark_stop_requested(id));
                let _ = reply.send(marked);
                if let Ok(mut b) = broker.lock() {
                    b.publish(RunNotice::Changed { run_id: id.into() });
                }
                user_stopped = true;
                signals.term_group(child.id());
                clock.wait(STOP_GRACE);
                match child.try_wait() {
                    Ok(Some(s)) => break Ok(s),
                    Err(e) => break Err(e),
                    Ok(None) => signals.kill_group(child.id()),
                }
            }
            Ok(ControllerCommand::Shutdown) if !shutdown && !user_stopped => {
                shutdown = true;
                signals.term_group(child.id());
                clock.wait(STOP_GRACE);
                match child.try_wait() {
                    Ok(Some(s)) => break Ok(s),
                    Err(e) => break Err(e),
                    Ok(None) => signals.kill_group(child.id()),
                }
            }
            Ok(_) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    };
    if status.is_ok() { /* try_wait already reaped */
    } else {
        let _ = child.wait();
    }
    let mut stdout_done = false;
    while !stdout_done {
        match out_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(ReaderMessage::Item(event)) => {
                match persist_event(db, id, session, &event, broker) {
                    Ok(()) => {
                        if event.final_result.is_some() {
                            final_result = event.final_result.clone()
                        }
                    }
                    Err(error) => {
                        persistence_error
                            .get_or_insert_with(|| format!("event persistence failed: {error}"));
                    }
                }
            }
            Ok(ReaderMessage::Eof) | Err(_) => stdout_done = true,
            Ok(ReaderMessage::Error(error)) => {
                persistence_error.get_or_insert(error);
                stdout_done = true;
            }
        }
    }
    let _ = stdout_reader.join();
    let mut err = Vec::new();
    loop {
        match err_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(ReaderMessage::Item(bytes)) => append_bounded(&mut err, &bytes),
            Ok(ReaderMessage::Eof) | Err(_) => break,
            Ok(ReaderMessage::Error(error)) => {
                persistence_error.get_or_insert(error);
                break;
            }
        }
    }
    let _ = stderr_reader.join();
    let stderr = String::from_utf8_lossy(&err);
    if let Ok(mut repo) = RunRepository::open(db) {
        let (state, code, error) = match status {
            _ if persistence_error.is_some() => (RunState::Failed, None, persistence_error.clone()),
            Ok(s) if user_stopped => (RunState::Stopped, s.code(), None),
            Ok(s) if shutdown => (
                RunState::Interrupted,
                s.code(),
                Some("run supervisor shut down".into()),
            ),
            Ok(s) if s.success() => (RunState::Succeeded, s.code(), None),
            Ok(s) => (
                RunState::Failed,
                s.code(),
                Some(format!(
                    "Cursor exited with {}",
                    s.code().map_or_else(|| "signal".into(), |c| c.to_string())
                )),
            ),
            Err(e) => (RunState::Failed, None, Some(e.to_string())),
        };
        let mut finish = repo.finish_with_transition(
            id,
            state,
            code,
            &stderr,
            final_result.as_deref(),
            error.as_deref(),
        );
        for attempt in 0..5 {
            if finish.is_ok() {
                break;
            }
            if !matches!(&finish, Err(RunError::Sql(rusqlite::Error::SqliteFailure(e, _))) if e.code == rusqlite::ErrorCode::DatabaseBusy || e.code == rusqlite::ErrorCode::DatabaseLocked)
            {
                break;
            }
            clock.wait(Duration::from_millis(20 * (attempt + 1)));
            finish = repo.finish_with_transition(
                id,
                state,
                code,
                &stderr,
                final_result.as_deref(),
                error.as_deref(),
            );
        }
        match finish {
            Ok(_) => {
                if let Ok(mut b) = broker.lock() {
                    b.publish(RunNotice::Changed { run_id: id.into() });
                }
            }
            Err(failure) => {
                operational.store(false, Ordering::Release);
                if let Ok(mut slot) = terminal_error.lock() {
                    *slot = Some(format!(
                        "terminal persistence failed for run {id}: {failure}"
                    ));
                }
            }
        }
    }
}
fn persist_event(
    db: &Path,
    id: &str,
    session: &str,
    event: &ParsedEvent,
    broker: &Arc<Mutex<NoticeBroker>>,
) -> Result<(), RunError> {
    let mut repo = RunRepository::open(db)?;
    let e = repo.append_event_with_metadata(session, id, event)?;
    if let Ok(mut b) = broker.lock() {
        b.publish(RunNotice::Event {
            run_id: id.into(),
            last_persisted_sequence: e.sequence,
        });
    }
    Ok(())
}
