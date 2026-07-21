use craftel_core::{
    domain::{Stage, WorkflowAction},
    run_service::RunService,
    runs::{Phase, Run, RunState},
    storage::{NewTask, SqliteRepository},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

struct AutomationFixture {
    _root: tempfile::TempDir,
    db: PathBuf,
    project_id: String,
    task_id: String,
    cursor: PathBuf,
}

impl AutomationFixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let work = root.path().join("work");
        std::fs::create_dir(&work).unwrap();
        let db = root.path().join("craftel.sqlite");
        let mut repository = SqliteRepository::open(&db).unwrap();
        let project = repository.register_project("automation", &work).unwrap();
        let task = repository
            .create_task(NewTask::new(&project.id, "task", "body", "task"))
            .unwrap();
        repository
            .apply_transition(&project.id, &task.id, WorkflowAction::Next)
            .unwrap();
        let cursor =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/automation_cursor.sh");
        Self {
            _root: root,
            db,
            project_id: project.id,
            task_id: task.id,
            cursor,
        }
    }

    fn service(&self) -> RunService {
        RunService::open(&self.db, &self.cursor).unwrap()
    }
    fn stage(&self) -> (Stage, bool) {
        let repository = SqliteRepository::open(&self.db).unwrap();
        let task = repository
            .get_task(&self.project_id, &self.task_id)
            .unwrap();
        (task.stage, task.review_approved)
    }
    fn action(&self, action: WorkflowAction) {
        SqliteRepository::open(&self.db)
            .unwrap()
            .apply_transition(&self.project_id, &self.task_id, action)
            .unwrap();
    }
}

fn mode(fixture: &AutomationFixture, value: &str) {
    // This suite is one test so its inherited process environment cannot race
    // another automation fixture mode in this test binary.
    unsafe {
        std::env::set_var("CRAFTEL_AUTOMATION_MODE", value);
        std::env::set_var("CRAFTEL_TEST_BIN", env!("CARGO_BIN_EXE_craftel"));
        std::env::set_var("CRAFTEL_DB_PATH", &fixture.db);
    }
}

fn terminal(service: &RunService, run: &Run) -> Run {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let current = service.get_run(&run.id).unwrap();
        if !matches!(current.state, RunState::Queued | RunState::Running) {
            return current;
        }
        assert!(
            Instant::now() < deadline,
            "run {} did not become terminal",
            run.id
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn start(service: &mut RunService, fixture: &AutomationFixture, selected_mode: &str) -> Run {
    mode(fixture, selected_mode);
    let run = service
        .start_current_phase(&fixture.project_id, &fixture.task_id)
        .unwrap();
    terminal(service, &run)
}

fn assert_transition(run: &Run, state: RunState, observed: bool, missing: bool) {
    assert_eq!(
        run.state, state,
        "error={:?}, stderr={}",
        run.error, run.stderr
    );
    assert_eq!(run.observed_transition_event_id.is_some(), observed);
    assert_eq!(run.missing_transition, missing);
}

#[test]
fn real_cli_transition_matrix_and_complete_session_cycles() {
    // Defining pass/fail and a successful transition followed by non-zero exit.
    for (selected_mode, expected_stage, expected_state) in [
        ("fail", Stage::Defining, RunState::Succeeded),
        ("pass", Stage::Implementation, RunState::Succeeded),
    ] {
        let fixture = AutomationFixture::new();
        let mut service = fixture.service();
        let run = start(&mut service, &fixture, selected_mode);
        assert_transition(&run, expected_state, true, false);
        assert_eq!(fixture.stage().0, expected_stage);
    }
    let fixture = AutomationFixture::new();
    let mut service = fixture.service();
    let run = start(&mut service, &fixture, "pass_nonzero");
    assert_transition(&run, RunState::Failed, true, false);
    assert_eq!(fixture.stage().0, Stage::Implementation);

    // Implementation pass and fail both execute the actual CLI.
    for (selected_mode, expected_stage) in
        [("fail", Stage::Implementation), ("pass", Stage::Reviewing)]
    {
        let fixture = AutomationFixture::new();
        fixture.action(WorkflowAction::Move(Stage::Implementation));
        let mut service = fixture.service();
        let run = start(&mut service, &fixture, selected_mode);
        assert_transition(&run, RunState::Succeeded, true, false);
        assert_eq!(fixture.stage().0, expected_stage);
    }

    // Review approval remains Reviewing until a human advances delivery.
    let fixture = AutomationFixture::new();
    fixture.action(WorkflowAction::Move(Stage::Reviewing));
    let mut service = fixture.service();
    let approved = start(&mut service, &fixture, "pass");
    assert_transition(&approved, RunState::Succeeded, true, false);
    assert_eq!(fixture.stage(), (Stage::Reviewing, true));
    fixture.action(WorkflowAction::Next);
    assert_eq!(fixture.stage().0, Stage::Done);

    // R1 fail returns to the reusable implementation session; R2 is fresh.
    let fixture = AutomationFixture::new();
    fixture.action(WorkflowAction::Move(Stage::Implementation));
    let mut service = fixture.service();
    let implementation_one = start(&mut service, &fixture, "pass");
    let review_one = start(&mut service, &fixture, "fail");
    assert_transition(&review_one, RunState::Succeeded, true, false);
    assert_eq!(fixture.stage().0, Stage::Implementation);
    let implementation_two = start(&mut service, &fixture, "pass");
    assert_eq!(implementation_one.session_id, implementation_two.session_id);
    let review_two = start(&mut service, &fixture, "pass");
    assert_ne!(review_one.session_id, review_two.session_id);
    let sessions = service
        .list_sessions(&fixture.project_id, &fixture.task_id)
        .unwrap();
    assert_eq!(
        sessions
            .iter()
            .filter(|s| s.phase == Phase::Reviewing)
            .count(),
        2
    );

    // No command is durable across a RunService reopen.
    let fixture = AutomationFixture::new();
    let mut service = fixture.service();
    let no_command = start(&mut service, &fixture, "none");
    assert_transition(&no_command, RunState::Succeeded, false, true);
    drop(service);
    let reopened = fixture.service();
    assert_transition(
        &reopened.get_run(&no_command.id).unwrap(),
        RunState::Succeeded,
        false,
        true,
    );
    drop(reopened);

    // Manual Move/Next after the run baseline are not automation attribution.
    for action in [
        WorkflowAction::Move(Stage::Implementation),
        WorkflowAction::Next,
    ] {
        let fixture = AutomationFixture::new();
        let mut service = fixture.service();
        mode(&fixture, "delayed_none");
        let running = service
            .start_current_phase(&fixture.project_id, &fixture.task_id)
            .unwrap();
        fixture.action(action);
        let finished = terminal(&service, &running);
        assert_transition(&finished, RunState::Succeeded, false, true);
    }

    // Stop never manufactures a fail transition.
    let fixture = AutomationFixture::new();
    let mut service = fixture.service();
    mode(&fixture, "hold");
    let running = service
        .start_current_phase(&fixture.project_id, &fixture.task_id)
        .unwrap();
    service.stop_run(&running.id).unwrap();
    let stopped = terminal(&service, &running);
    assert_eq!(stopped.state, RunState::Stopped);
    assert!(stopped.observed_transition_event_id.is_none());
    assert_eq!(fixture.stage().0, Stage::Defining);
}
