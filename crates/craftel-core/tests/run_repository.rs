use craftel_core::{
    runs::{EventKind, Phase, RunRepository, RunState},
    storage::{NewTask, SqliteRepository},
};
#[test]
fn durable_ordered_runs_and_terminal_immutability() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("db");
    let work = t.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let mut base = SqliteRepository::open(&db).unwrap();
    let p = base.register_project("p", &work).unwrap();
    let task = base
        .create_task(NewTask::new(&p.id, "t", "c", "task"))
        .unwrap();
    drop(base);
    let mut r = RunRepository::open(&db).unwrap();
    let s = r
        .create_session(&p.id, &task.id, Phase::Defining, "cursor")
        .unwrap();
    let run = r.reserve_run(&s, "prompt", &work).unwrap();
    assert!(r.reserve_run(&s, "other", &work).is_err());
    r.append_event(&run.id, EventKind::Unknown, None, None, None, "raw")
        .unwrap();
    r.finish(&run.id, RunState::Succeeded, Some(0), "", None, None)
        .unwrap();
    assert!(
        r.finish(&run.id, RunState::Failed, Some(1), "", None, None)
            .is_err()
    );
    drop(r);
    assert_eq!(
        RunRepository::open(&db)
            .unwrap()
            .list_events(&run.id, 0, 10)
            .unwrap()[0]
            .raw_json,
        "raw"
    );
}
