use craftel_core::{
    run_service::RunService,
    runs::{Phase, RunState},
    storage::{NewTask, SqliteRepository},
};
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
    drop(base);
    unsafe { std::env::set_var("CRAFTEL_RECORD", &record) };
    let fake =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_cursor.sh");
    let mut service = RunService::open(&db, fake).unwrap();
    let run = service
        .start_phase_run(&p.id, &task.id, Phase::Defining, "hello")
        .unwrap();
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
    assert!(
        std::fs::read_to_string(record.join("argv"))
            .unwrap()
            .contains("--resume=fake-session")
    );
}
