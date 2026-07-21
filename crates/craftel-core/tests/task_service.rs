use craftel_core::{
    CraftelService,
    automation::build_prompt,
    domain::Stage,
    runs::{Phase, RunRepository},
    storage::SqliteRepository,
};
use std::fs;
use tempfile::TempDir;

fn setup() -> (TempDir, std::path::PathBuf, String) {
    let root = tempfile::tempdir().unwrap();
    let work = root.path().join("project");
    fs::create_dir(&work).unwrap();
    let db = root.path().join("craftel.db");
    let mut service = CraftelService::open(&db).unwrap();
    let id = service.register_project("Project", &work).unwrap().id;
    (root, db, id)
}

#[test]
fn creates_documents_and_preserves_spec_during_updates_and_repair() {
    let (root, db, project) = setup();
    let work = root.path().join("project");
    let mut service = CraftelService::open(&db).unwrap();
    assert!(service.create_task(&project, " ", "content").is_err());
    let task = service
        .create_task(&project, "Café Feature", "full\ncontent")
        .unwrap();
    assert_eq!(
        task.relative_dir,
        std::path::PathBuf::from("craftel/tasks/T0001-cafe-feature")
    );
    let dir = work.join(&task.relative_dir);
    let task_md = fs::read_to_string(dir.join("TASK.md")).unwrap();
    let yaml = task_md.split("---").nth(1).unwrap();
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(value["id"], "T0001");
    assert_eq!(value["status"], "inbox");
    assert!(task_md.contains("full\ncontent") && task_md.contains("managed by CRAFTEL"));
    fs::write(dir.join("SPEC.md"), "agent authored").unwrap();
    service
        .update_task(&project, "T0001", "Renamed", "new content")
        .unwrap();
    assert_eq!(
        fs::read_to_string(dir.join("SPEC.md")).unwrap(),
        "agent authored"
    );
    fs::remove_file(dir.join("TASK.md")).unwrap();
    service.get_task(&project, "T0001").unwrap();
    assert!(dir.join("TASK.md").is_file());
    assert_eq!(
        fs::read_to_string(dir.join("SPEC.md")).unwrap(),
        "agent authored"
    );
    assert!(work.join("craftel/INDEX.md").is_file());
}

#[test]
fn deterministic_create_failure_compensates_only_its_row_and_directory() {
    let (root, db, project) = setup();
    let work = root.path().join("project");
    let existing = work.join("craftel/tasks/T0001-conflict");
    fs::create_dir_all(&existing).unwrap();
    fs::write(existing.join("keep"), "safe").unwrap();
    let mut service = CraftelService::open(&db).unwrap();
    assert!(
        service
            .create_task(&project, "Conflict", "content")
            .is_err()
    );
    assert_eq!(fs::read_to_string(existing.join("keep")).unwrap(), "safe");
    assert!(service.list_tasks(&project).unwrap().is_empty());
}

#[test]
fn deterministic_update_and_workflow_failures_remain_authoritative_and_repair() {
    let (root, db, project) = setup();
    let mut service = CraftelService::open(&db).unwrap();
    let task = service.create_task(&project, "Title", "content").unwrap();
    let task_md = root
        .path()
        .join("project")
        .join(&task.relative_dir)
        .join("TASK.md");
    fs::remove_file(&task_md).unwrap();
    fs::create_dir(&task_md).unwrap();
    assert!(
        service
            .update_task(&project, "T0001", "Updated", "durable")
            .is_err()
    );
    assert_eq!(
        SqliteRepository::open(&db)
            .unwrap()
            .get_task(&project, "T0001")
            .unwrap()
            .title,
        "Updated"
    );
    assert!(
        service
            .move_task(&project, "T0001", Stage::Defining)
            .is_err()
    );
    let repo = SqliteRepository::open(&db).unwrap();
    assert_eq!(
        repo.get_task(&project, "T0001").unwrap().stage,
        Stage::Defining
    );
    assert!(repo.projection_dirty(&project, "T0001").unwrap());
    fs::remove_dir(&task_md).unwrap();
    drop(service);
    let mut reopened = CraftelService::open(&db).unwrap();
    assert_eq!(
        reopened.get_task(&project, "T0001").unwrap().title,
        "Updated"
    );
    assert!(task_md.is_file());
    assert!(
        !SqliteRepository::open(&db)
            .unwrap()
            .projection_dirty(&project, "T0001")
            .unwrap()
    );
}

#[test]
fn opening_skips_unavailable_projects() {
    let (root, db, project) = setup();
    let work = root.path().join("project");
    let mut service = CraftelService::open(&db).unwrap();
    service.create_task(&project, "Title", "content").unwrap();
    drop(service);
    fs::remove_dir_all(work).unwrap();
    let service = CraftelService::open(&db).unwrap();
    assert!(!service.list_projects().unwrap()[0].available);
}

#[test]
fn active_runs_block_manual_stage_changes() {
    let (root, db, project) = setup();
    let mut service = CraftelService::open(&db).unwrap();
    let task = service.create_task(&project, "Title", "content").unwrap();
    service
        .move_task(&project, &task.id, Stage::Defining)
        .unwrap();
    let prompt = build_prompt(
        Phase::Defining,
        &task.id,
        &project,
        &root.path().join("project"),
    );
    RunRepository::open(&db)
        .unwrap()
        .reserve_phase_run(&project, &task.id, Phase::Defining, &prompt)
        .unwrap();

    assert!(
        service
            .apply(
                &project,
                &task.id,
                craftel_core::domain::WorkflowAction::Move(Stage::Implementation),
            )
            .is_err()
    );
    assert!(
        service
            .apply(
                &project,
                &task.id,
                craftel_core::domain::WorkflowAction::Next,
            )
            .is_err()
    );
    assert_eq!(
        service.get_task(&project, &task.id).unwrap().stage,
        Stage::Defining
    );
}
