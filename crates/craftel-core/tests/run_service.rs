use craftel_core::{
    domain::{Stage, WorkflowAction},
    run_service::{
        ProcessFactory, RunService, SystemControllerClock, SystemProcessInspector,
        SystemProcessSignals,
    },
    runs::RunState,
    storage::{NewTask, SqliteRepository},
};
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};

struct VersionBarrierFactory {
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
    spawns: Arc<AtomicUsize>,
}
impl ProcessFactory for VersionBarrierFactory {
    fn version(&self) -> std::io::Result<String> {
        self.entered.wait();
        self.release.wait();
        Ok("acceptance".into())
    }
    fn spawn(
        &self,
        _: &str,
        _: Option<&str>,
        cwd: &std::path::Path,
        _: &str,
    ) -> std::io::Result<std::process::Child> {
        self.spawns.fetch_add(1, Ordering::SeqCst);
        std::process::Command::new("sh")
            .args(["-c", "exit 0"])
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
    }
}

#[test]
fn queued_run_atomically_pins_the_task_stage_before_spawn() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("db");
    let work = t.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let mut repo = SqliteRepository::open(&db).unwrap();
    let p = repo.register_project("p", &work).unwrap();
    let task = repo
        .create_task(NewTask::new(&p.id, "t", "c", "task"))
        .unwrap();
    repo.apply_transition(&p.id, &task.id, WorkflowAction::Next)
        .unwrap();
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let spawns = Arc::new(AtomicUsize::new(0));
    let factory = Arc::new(VersionBarrierFactory {
        entered: entered.clone(),
        release: release.clone(),
        spawns: spawns.clone(),
    });
    let db2 = db.clone();
    let project = p.id.clone();
    let task_id = task.id.clone();
    let worker = std::thread::spawn(move || {
        let mut service = RunService::open_with_seams(
            &db2,
            factory,
            Arc::new(SystemProcessInspector),
            Arc::new(SystemControllerClock),
            Arc::new(SystemProcessSignals),
        )
        .unwrap();
        service.start_current_phase(&project, &task_id)
    });
    entered.wait();
    assert!(
        repo.apply_transition(&p.id, &task.id, WorkflowAction::Move(Stage::Implementation))
            .is_err()
    );
    release.wait();
    assert!(worker.join().unwrap().is_ok());
    assert_eq!(spawns.load(Ordering::SeqCst), 1);
}
#[test]
fn fake_end_to_end_persists_and_resumes() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("db");
    let work = t.path().join("work");
    let record = t.path().join("record");
    std::fs::create_dir(&work).unwrap();
    std::fs::create_dir(&record).unwrap();
    let mut base = SqliteRepository::open(&db).unwrap();
    let p = base.register_project("p", &work).unwrap();
    let task = base
        .create_task(NewTask::new(&p.id, "t", "c", "task"))
        .unwrap();
    base.apply_transition(&p.id, &task.id, WorkflowAction::Next)
        .unwrap();
    drop(base);
    unsafe { std::env::set_var("CRAFTEL_RECORD", &record) };
    let fake =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_cursor.sh");
    let mut service = RunService::open(&db, fake).unwrap();
    let run = service.start_current_phase(&p.id, &task.id).unwrap();
    for _ in 0..100 {
        if !matches!(
            service.get_run(&run.id).unwrap().state,
            RunState::Running | RunState::Queued
        ) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let done = service.get_run(&run.id).unwrap();
    assert_eq!(done.state, RunState::Succeeded);
    assert_eq!(service.list_run_events(&run.id, 0, 10).unwrap().len(), 4);
    let follow = service.follow_up(&run.session_id, "again").unwrap();
    for _ in 0..100 {
        if service.get_run(&follow.id).unwrap().state == RunState::Succeeded {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        service.get_run(&follow.id).unwrap().state,
        RunState::Succeeded
    );
    assert!(
        std::fs::read_to_string(record.join("argv"))
            .unwrap()
            .contains("--resume=fake-session")
    );
    SqliteRepository::open(&db)
        .unwrap()
        .apply_transition(&p.id, &task.id, WorkflowAction::Move(Stage::Implementation))
        .unwrap();
    assert!(service.follow_up(&run.session_id, "stale phase").is_err());
}
