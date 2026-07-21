use craftel_core::{
    automation::{PROMPT_VERSION, build_prompt},
    domain::WorkflowAction,
    runs::{EventKind, Phase, RunRepository, RunState},
    storage::{NewTask, SqliteRepository},
};

#[test]
fn exact_phase_prompt_snapshots_do_not_include_environment_secrets() {
    unsafe { std::env::set_var("CRAFTEL_ACCEPTANCE_SECRET", "must-not-leak") };
    let common = |stage: &str, workflow: &str| {
        format!(
            "CRAFTEL autonomous phase prompt v1\n\nTask ID: T0007\nProject ID: project-id\nAbsolute work directory: /isolated/project\nCurrent stage: {stage}\n\n{workflow}\n\nDocument ownership: TASK.md is generated and owned by CRAFTEL; do not edit TASK.md. Put agent-authored work in SPEC.md or supporting decisions/, discussions/, notes/, subtasks/, plans/, and reviews/ directories. Inspect all required artifacts, code, and tests before deciding the outcome.\n\nOn success, run exactly:\ncraftel pass T0007 --project project-id\n\nOn failure, run exactly:\ncraftel fail T0007 --project project-id\n\nInvoke exactly one of those commands before exiting. Do not infer or merely describe the transition. Do not commit, push, open a pull request, release, deploy, or deliver anything."
        )
    };
    let cases = [
        (
            Phase::Defining,
            "defining",
            "Use the installed define-spec workflow. Override its review-turn pause: proceed autonomously from the approved design and context, write the specification, and verify it.",
        ),
        (
            Phase::Implementation,
            "implementation",
            "Use the installed implement-spec workflow and the current agent-owned documents. Inspect the specification, supporting artifacts, code, and tests; implement and verify the task autonomously.",
        ),
        (
            Phase::Reviewing,
            "reviewing",
            "Perform a fresh evidence-based formal review in this newly created review session. Inspect the specification, supporting artifacts, code, and tests, and write a review record. Do not reuse implementation context and do not implement fixes in this review session.",
        ),
    ];
    for (phase, stage, workflow) in cases {
        let prompt = build_prompt(
            phase,
            "T0007",
            "project-id",
            std::path::Path::new("/isolated/project"),
        );
        assert_eq!(prompt.kind, phase);
        assert_eq!(prompt.version, PROMPT_VERSION);
        assert_eq!(prompt.text, common(stage, workflow));
        assert!(!prompt.text.contains("must-not-leak"));
    }
    unsafe { std::env::remove_var("CRAFTEL_ACCEPTANCE_SECRET") };
}
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
    base.apply_transition(&p.id, &task.id, WorkflowAction::Next)
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
