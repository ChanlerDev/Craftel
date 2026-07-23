use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

struct Fixture {
    root: TempDir,
    db: std::path::PathBuf,
    project: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir(&project).unwrap();
        Self {
            db: root.path().join("db.sqlite"),
            root,
            project,
        }
    }
    fn cmd(&self) -> Command {
        let mut c = Command::cargo_bin("craftel").unwrap();
        c.env("CRAFTEL_DB_PATH", &self.db);
        c
    }
    fn add(&self) -> String {
        let out = self
            .cmd()
            .args([
                "project",
                "add",
                self.project.to_str().unwrap(),
                "--name",
                "Test",
            ])
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout)
            .unwrap()
            .split('\t')
            .next()
            .unwrap()
            .into()
    }
}

#[test]
fn project_task_and_transition_workflows() {
    let f = Fixture::new();
    let id = f.add();
    let projects = f
        .cmd()
        .args(["project", "list", "--json"])
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&projects.stdout).unwrap();
    assert_eq!(value[0]["available"], true);
    f.cmd()
        .args([
            "create",
            "--project",
            &id,
            "--title",
            "Task",
            "--content",
            "Content",
        ])
        .assert()
        .success();
    f.cmd()
        .args([
            "task",
            "update",
            "T0001",
            "--project",
            &id,
            "--title",
            "New",
            "--content",
            "Body",
        ])
        .assert()
        .success();
    f.cmd()
        .args(["move", "T0001", "defining", "--project", &id])
        .assert()
        .success();
    f.cmd()
        .args(["pass", "T0001", "--project", &id])
        .assert()
        .success();
    f.cmd()
        .args(["fail", "T0001", "--project", &id])
        .assert()
        .success();
    f.cmd()
        .args(["next", "T0001", "--project", &id])
        .assert()
        .success();
    f.cmd()
        .args(["pass", "T0001", "--project", &id])
        .assert()
        .success();
    let tasks = f
        .cmd()
        .args(["task", "list", "--project", &id, "--json"])
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&tasks.stdout).unwrap();
    assert_eq!(value[0]["stage"], "reviewing");
    assert!(f.project.join("craftel/tasks/T0001-task/TASK.md").is_file());
    assert!(f.project.join("craftel/tasks/T0001-task/SPEC.md").is_file());
}

#[test]
fn infers_longest_ancestor_and_explicit_project_wins() {
    let f = Fixture::new();
    let outer = f.add();
    let nested = f.project.join("nested");
    fs::create_dir(&nested).unwrap();
    let output = f
        .cmd()
        .args([
            "project",
            "add",
            nested.to_str().unwrap(),
            "--name",
            "Nested",
        ])
        .output()
        .unwrap();
    let inner = String::from_utf8(output.stdout)
        .unwrap()
        .split('\t')
        .next()
        .unwrap()
        .to_string();
    fs::create_dir(nested.join("deep")).unwrap();
    f.cmd()
        .current_dir(nested.join("deep"))
        .args(["create", "--title", "Inner", "--content", "C"])
        .assert()
        .success();
    f.cmd()
        .current_dir(&nested)
        .args(["task", "list", "--project", &outer, "--json"])
        .assert()
        .stdout("[]\n");
    f.cmd()
        .args(["task", "list", "--project", &inner, "--json"])
        .assert()
        .success();
}

#[test]
fn missing_project_is_reported_removal_preserves_files_and_invalid_transition_fails() {
    let f = Fixture::new();
    let id = f.add();
    f.cmd()
        .args([
            "create",
            "--project",
            &id,
            "--title",
            "Task",
            "--content",
            "C",
        ])
        .assert()
        .success();
    f.cmd()
        .args(["next", "T0001", "--project", &id])
        .assert()
        .success();
    f.cmd()
        .args(["next", "T0001", "--project", &id])
        .assert()
        .success();
    f.cmd()
        .args(["next", "T0001", "--project", &id])
        .assert()
        .success();
    f.cmd()
        .args(["next", "T0001", "--project", &id])
        .assert()
        .success();
    f.cmd()
        .args(["next", "T0001", "--project", &id])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot next"));
    let marker = f.project.join("marker");
    fs::write(&marker, "keep").unwrap();
    let moved = f.root.path().join("moved");
    fs::rename(&f.project, &moved).unwrap();
    let listed = f
        .cmd()
        .args(["project", "list", "--json"])
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(value[0]["available"], false);
    f.cmd().args(["project", "remove", &id]).assert().success();
    assert_eq!(fs::read_to_string(moved.join("marker")).unwrap(), "keep");
}
