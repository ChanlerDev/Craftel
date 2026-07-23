use craftel_core::CraftelService;
use std::{fs, path::Path, process::Command};

fn git(dir: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn reports_staged_unstaged_and_untracked_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-b", "evidence"]);
    git(&repo, &["config", "user.name", "Craftel Test"]);
    git(&repo, &["config", "user.email", "craftel@example.invalid"]);
    fs::write(repo.join("staged.txt"), "before\n").unwrap();
    fs::write(repo.join("unstaged.txt"), "before\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "initial evidence"]);
    fs::write(repo.join("staged.txt"), "after staged\n").unwrap();
    git(&repo, &["add", "staged.txt"]);
    fs::write(repo.join("unstaged.txt"), "after unstaged\n").unwrap();
    fs::write(repo.join("untracked note.txt"), "new\n").unwrap();

    let mut service = CraftelService::open(&temp.path().join("db.sqlite")).unwrap();
    let project = service.register_project("repo", &repo).unwrap();
    let summary = service.git_working_copy_summary(&project.id).unwrap();
    assert!(summary.is_repository);
    assert_eq!(summary.branch.as_deref(), Some("evidence"));
    assert_eq!(summary.latest_commit.unwrap().subject, "initial evidence");
    assert!(summary.staged_diff.contains("+after staged"));
    assert!(summary.unstaged_diff.contains("+after unstaged"));
    assert!(
        summary
            .untracked_paths
            .contains(&"untracked note.txt".to_owned())
    );
    assert!(!summary.truncated);
    assert!(!summary.staged_diff.contains(repo.to_str().unwrap()));
}

#[test]
fn non_repository_is_a_typed_empty_summary() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("plain");
    fs::create_dir(&repo).unwrap();
    let mut service = CraftelService::open(&temp.path().join("db.sqlite")).unwrap();
    let project = service.register_project("plain", &repo).unwrap();
    let summary = service.git_working_copy_summary(&project.id).unwrap();
    assert!(!summary.is_repository);
    assert_eq!(summary.branch, None);
    assert!(summary.staged_diff.is_empty());
}

#[test]
fn repository_without_a_commit_still_returns_working_copy_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("initial");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-b", "initial"]);
    fs::write(repo.join("untracked.txt"), "draft\n").unwrap();
    let mut service = CraftelService::open(&temp.path().join("db.sqlite")).unwrap();
    let project = service.register_project("initial", &repo).unwrap();

    let summary = service.git_working_copy_summary(&project.id).unwrap();

    assert!(summary.is_repository);
    assert_eq!(summary.branch.as_deref(), Some("initial"));
    assert_eq!(summary.latest_commit, None);
    assert!(
        summary
            .untracked_paths
            .contains(&"untracked.txt".to_owned())
    );
    assert!(
        summary
            .untracked_paths
            .contains(&"craftel/INDEX.md".to_owned())
    );
}
