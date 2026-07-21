use std::fs;

use craftel_core::domain::{Stage, WorkflowAction};
use craftel_core::storage::{NewTask, SqliteRepository, UpdateTask};
use tempfile::tempdir;

#[test]
fn repositories_persist_projects_tasks_and_transitions() {
    let temp = tempdir().unwrap();
    let db = temp.path().join("craftel.sqlite3");
    let work = temp.path().join("work");
    fs::create_dir(&work).unwrap();
    let mut repo = SqliteRepository::open(&db).unwrap();
    let project = repo.register_project("Work", &work.join(".")).unwrap();
    assert!(project.work_dir.is_absolute());
    assert_eq!(project.work_dir, work.canonicalize().unwrap());
    assert!(repo.register_project("Again", &work).is_err());

    let task = repo
        .create_task(NewTask::new(
            &project.id,
            "One",
            "Body",
            "craftel/tasks/T0001-one",
        ))
        .unwrap();
    assert_eq!(task.id, "T0001");
    let second = repo
        .create_task(NewTask::new(
            &project.id,
            "Two",
            "Other",
            "craftel/tasks/T0002-two",
        ))
        .unwrap();
    assert_eq!(second.id, "T0002");
    repo.apply_transition(&project.id, &task.id, WorkflowAction::Next)
        .unwrap();
    repo.apply_transition(&project.id, &task.id, WorkflowAction::Next)
        .unwrap();
    repo.apply_transition(&project.id, &task.id, WorkflowAction::Next)
        .unwrap();
    repo.apply_transition(&project.id, &task.id, WorkflowAction::Pass)
        .unwrap();
    repo.update_task(UpdateTask::new(
        &project.id,
        &task.id,
        "Changed",
        "Changed body",
    ))
    .unwrap();
    drop(repo);

    let repo = SqliteRepository::open(&db).unwrap();
    let task = repo.get_task(&project.id, "T0001").unwrap();
    assert_eq!(
        (task.title.as_str(), task.content.as_str(), task.stage),
        ("Changed", "Changed body", Stage::Reviewing)
    );
    assert!(task.review_approved);
    assert_eq!(
        task.relative_dir,
        std::path::Path::new("craftel/tasks/T0001-one")
    );
    assert!(repo.foreign_keys_enabled().unwrap());
    fs::remove_dir_all(&work).unwrap();
    assert!(!repo.list_projects().unwrap()[0].available);
}

#[test]
fn registration_order_and_removal_are_safe() {
    let temp = tempdir().unwrap();
    let db = temp.path().join("db");
    let one = temp.path().join("one");
    let two = temp.path().join("two");
    fs::create_dir(&one).unwrap();
    fs::create_dir(&two).unwrap();
    let mut repo = SqliteRepository::open(&db).unwrap();
    let p1 = repo.register_project("One", &one).unwrap();
    let p2 = repo.register_project("Two", &two).unwrap();
    repo.touch_project(&p1.id).unwrap();
    assert_eq!(
        repo.list_projects()
            .unwrap()
            .iter()
            .map(|p| &p.id)
            .collect::<Vec<_>>(),
        [&p1.id, &p2.id]
    );
    repo.remove_project(&p1.id).unwrap();
    assert!(one.exists());
}

#[test]
fn independent_connections_allocate_unique_numbers() {
    let temp = tempdir().unwrap();
    let db = temp.path().join("db");
    let work = temp.path().join("work");
    fs::create_dir(&work).unwrap();
    let mut first = SqliteRepository::open(&db).unwrap();
    let project = first.register_project("P", &work).unwrap();
    let mut second = SqliteRepository::open(&db).unwrap();
    let a = first
        .create_task(NewTask::new(&project.id, "A", "A", "a"))
        .unwrap();
    let b = second
        .create_task(NewTask::new(&project.id, "B", "B", "b"))
        .unwrap();
    assert_eq!((a.id.as_str(), b.id.as_str()), ("T0001", "T0002"));
}

#[test]
fn app_path_override_is_shared() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("nested/db.sqlite3");
    unsafe { std::env::set_var("CRAFTEL_DB_PATH", &path) };
    let cli = craftel_core::app_paths::database_path().unwrap();
    let desktop = craftel_core::app_paths::database_path().unwrap();
    unsafe { std::env::remove_var("CRAFTEL_DB_PATH") };
    assert_eq!(cli, desktop);
    assert_eq!(cli, path);
    assert!(path.parent().unwrap().exists());
}
