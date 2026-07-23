use craftel_core::{
    domain::{Stage, WorkflowAction},
    storage::{NewTask, SqliteRepository},
};

#[test]
fn formal_review_is_optional_and_fail_returns_to_implementation() {
    let temp = tempfile::tempdir().unwrap();
    let work = temp.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let mut repo = SqliteRepository::open(&temp.path().join("db")).unwrap();
    let project = repo.register_project("project", &work).unwrap();
    let task = repo
        .create_task(NewTask::new(&project.id, "task", "content", "task"))
        .unwrap();
    repo.apply_transition(
        &project.id,
        &task.id,
        WorkflowAction::Move(Stage::Reviewing),
    )
    .unwrap();
    let done_without_formal_review = repo
        .apply_transition(&project.id, &task.id, WorkflowAction::Next)
        .unwrap();
    assert_eq!(done_without_formal_review.stage.to_string(), "done");

    repo.apply_transition(&project.id, &task.id, WorkflowAction::Move(Stage::Inbox))
        .unwrap();
    for action in [
        WorkflowAction::Next,
        WorkflowAction::Pass,
        WorkflowAction::Next,
        WorkflowAction::Pass,
    ] {
        repo.apply_transition(&project.id, &task.id, action)
            .unwrap();
    }
    let approved = repo
        .apply_transition(&project.id, &task.id, WorkflowAction::Pass)
        .unwrap();
    assert_eq!(approved.stage.to_string(), "reviewing");
    assert!(approved.review_approved);
    let done = repo
        .apply_transition(&project.id, &task.id, WorkflowAction::Next)
        .unwrap();
    assert_eq!(done.stage.to_string(), "done");

    repo.apply_transition(
        &project.id,
        &task.id,
        WorkflowAction::Move(Stage::Reviewing),
    )
    .unwrap();
    let changes = repo
        .apply_transition(&project.id, &task.id, WorkflowAction::Fail)
        .unwrap();
    assert_eq!(changes.stage.to_string(), "implementation");
    assert!(!changes.review_approved);
}
