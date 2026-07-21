use craftel_core::{CraftelService, ServiceError, documents::ExpectedDocumentState};
use std::fs;
#[test]
fn document_crud_conflict_restore_delete_and_unavailable() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("db");
    let root = t.path().join("p");
    fs::create_dir(&root).unwrap();
    let mut s = CraftelService::open(&db).unwrap();
    let p = s.register_project("p", &root).unwrap();
    let task = s.create_task(&p.id, "x", "y").unwrap();
    s.reconcile_documents(&p.id).unwrap();
    let path = format!("{}/SPEC.md", task.relative_dir.to_string_lossy());
    let doc = s.read_document(&p.id, &path).unwrap();
    assert!(
        !s.list_documents(&p.id, true)
            .unwrap()
            .iter()
            .any(|d| d.relative_path.ends_with("TASK.md"))
    );
    let _edited = s
        .write_document(
            &p.id,
            &path,
            "# Changed\nsearchable",
            ExpectedDocumentState::Present(doc.content_hash.clone()),
        )
        .unwrap();
    assert!(matches!(
        s.write_document(
            &p.id,
            &path,
            "bad",
            ExpectedDocumentState::Present(doc.content_hash.clone())
        ),
        Err(ServiceError::Conflict)
    ));
    assert_eq!(s.search_documents(&p.id, "searchable").unwrap().len(), 1);
    let old = s
        .list_document_revisions(&p.id, &path)
        .unwrap()
        .last()
        .unwrap()
        .id
        .clone();
    fs::remove_file(root.join(&path)).unwrap();
    s.reconcile_documents(&p.id).unwrap();
    assert!(!s.read_document(&p.id, &path).unwrap().present);
    let restored = s
        .restore_document_revision(&p.id, &path, &old, ExpectedDocumentState::Missing)
        .unwrap();
    assert!(restored.present);
    assert_eq!(
        s.list_document_revisions(&p.id, &path).unwrap()[0].cause,
        "restore"
    );
    fs::remove_dir_all(&root).unwrap();
    assert!(matches!(
        s.list_documents(&p.id, false),
        Err(ServiceError::Unavailable)
    ));
}
#[test]
fn cross_project_restore_is_rejected() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("db");
    let mut s = CraftelService::open(&db).unwrap();
    let mut ps = vec![];
    for n in ["a", "b"] {
        let d = t.path().join(n);
        fs::create_dir(&d).unwrap();
        let p = s.register_project(n, &d).unwrap();
        s.create_task(&p.id, "x", "y").unwrap();
        s.reconcile_documents(&p.id).unwrap();
        ps.push(p)
    }
    let path = "craftel/tasks/T0001-x/SPEC.md";
    let rev = s.list_document_revisions(&ps[0].id, path).unwrap()[0]
        .id
        .clone();
    assert!(
        s.restore_document_revision(
            &ps[1].id,
            path,
            &rev,
            ExpectedDocumentState::Present(s.read_document(&ps[1].id, path).unwrap().content_hash)
        )
        .is_err()
    );
}

#[test]
fn document_error_is_durable_and_clears_after_recovery() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("db");
    let root = t.path().join("p");
    fs::create_dir(&root).unwrap();
    let mut service = CraftelService::open(&db).unwrap();
    let project = service.register_project("p", &root).unwrap();
    service.create_task(&project.id, "x", "y").unwrap();
    let spec = root.join("craftel/tasks/T0001-x/SPEC.md");
    fs::write(&spec, [0xff]).unwrap();
    assert!(service.reconcile_documents(&project.id).is_err());
    let prior = service
        .read_document(&project.id, "craftel/tasks/T0001-x/SPEC.md")
        .unwrap();
    drop(service);

    let mut reopened = CraftelService::open(&db).unwrap();
    let status = reopened.document_status(&project.id).unwrap();
    assert_eq!(status.state, "error");
    assert!(status.error.is_some());
    assert_eq!(
        reopened
            .read_document(&project.id, "craftel/tasks/T0001-x/SPEC.md")
            .unwrap()
            .content_hash,
        prior.content_hash
    );
    fs::write(spec, "# recovered").unwrap();
    reopened.reconcile_documents(&project.id).unwrap();
    assert_eq!(
        reopened.document_status(&project.id).unwrap().state,
        "healthy"
    );
}

#[test]
fn two_services_reject_a_stale_disk_write() {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("db");
    let root = t.path().join("p");
    fs::create_dir(&root).unwrap();
    let mut first = CraftelService::open(&db).unwrap();
    let project = first.register_project("p", &root).unwrap();
    first.create_task(&project.id, "x", "y").unwrap();
    let mut second = CraftelService::open(&db).unwrap();
    let path = "craftel/tasks/T0001-x/SPEC.md";
    let stale = second
        .read_document(&project.id, path)
        .unwrap()
        .content_hash;
    first
        .write_document(
            &project.id,
            path,
            "# first",
            ExpectedDocumentState::Present(stale.clone()),
        )
        .unwrap();
    assert!(matches!(
        second.write_document(
            &project.id,
            path,
            "# second",
            ExpectedDocumentState::Present(stale)
        ),
        Err(ServiceError::Conflict)
    ));
}
