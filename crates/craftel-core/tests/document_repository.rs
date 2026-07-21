use chrono::{Duration, Utc};
use craftel_core::{
    documents::{DocumentCause, DocumentRepository},
    storage::SqliteRepository,
};

#[test]
fn migration_reopen_search_dedup_delete_restore_retention_and_cascade() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("db.sqlite");
    let project_dir = temp.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();
    let mut base = SqliteRepository::open(&db).unwrap();
    let project = base.register_project("one", &project_dir).unwrap();
    drop(base);
    let mut docs = DocumentRepository::open(&db).unwrap();
    let now = Utc::now();
    docs.ingest(
        &project.id,
        "craftel/INDEX.md",
        None,
        b"# Alpha\nneedle",
        DocumentCause::Scan,
        now,
        1,
        10,
        false,
    )
    .unwrap();
    docs.ingest(
        &project.id,
        "craftel/INDEX.md",
        None,
        b"# Alpha\nneedle",
        DocumentCause::Watch,
        now,
        1,
        10,
        false,
    )
    .unwrap();
    assert_eq!(
        docs.revisions(&project.id, "craftel/INDEX.md")
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        docs.search(&project.id, "needle").unwrap()[0].relative_path,
        "craftel/INDEX.md"
    );
    docs.mark_deleted(&project.id, "craftel/INDEX.md", now)
        .unwrap();
    assert!(!docs.read(&project.id, "craftel/INDEX.md").unwrap().present);
    let snapshot = docs.revisions(&project.id, "craftel/INDEX.md").unwrap()[0].clone();
    docs.ingest(
        &project.id,
        "craftel/INDEX.md",
        None,
        &snapshot.content,
        DocumentCause::Restore,
        now,
        1,
        10,
        true,
    )
    .unwrap();
    assert_eq!(
        docs.revisions(&project.id, "craftel/INDEX.md")
            .unwrap()
            .len(),
        2
    );
    docs.prune(now + Duration::days(31)).unwrap();
    assert_eq!(
        docs.revisions(&project.id, "craftel/INDEX.md")
            .unwrap()
            .len(),
        1
    );
    drop(docs);
    let mut base = SqliteRepository::open(&db).unwrap();
    base.remove_project(&project.id).unwrap();
    drop(base);
    let docs = DocumentRepository::open(&db).unwrap();
    assert!(docs.list(&project.id, true).unwrap().is_empty());
}

#[test]
fn paths_and_search_scope_are_deterministic() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("db");
    let mut base = SqliteRepository::open(&db).unwrap();
    let mut ids = Vec::new();
    for name in ["a", "b"] {
        let d = temp.path().join(name);
        std::fs::create_dir(&d).unwrap();
        ids.push(base.register_project(name, &d).unwrap().id);
    }
    drop(base);
    let mut docs = DocumentRepository::open(&db).unwrap();
    let now = Utc::now();
    assert!(
        docs.ingest(
            &ids[0],
            "../SPEC.md",
            None,
            b"x",
            DocumentCause::Scan,
            now,
            0,
            1,
            false
        )
        .is_err()
    );
    for (id, path) in [
        (&ids[0], "craftel/tasks/T0001-x/SPEC.md"),
        (&ids[0], "craftel/INDEX.md"),
        (&ids[1], "craftel/INDEX.md"),
    ] {
        docs.ingest(
            id,
            path,
            None,
            b"# Same\nterm",
            DocumentCause::Scan,
            now,
            0,
            10,
            false,
        )
        .unwrap();
    }
    let found = docs.search(&ids[0], "term").unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].relative_path, "craftel/INDEX.md");
}
