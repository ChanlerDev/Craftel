use craftel_core::{
    CraftelService,
    documents::{
        DocumentChange, DocumentRepository, ExpectedDocumentState, ProjectWatcher,
        reconcile_project,
    },
};
use std::{fs, sync::mpsc, thread, time::Duration};

fn setup() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    craftel_core::domain::Project,
) {
    let t = tempfile::tempdir().unwrap();
    let db = t.path().join("db");
    let root = t.path().join("p");
    fs::create_dir(&root).unwrap();
    let mut service = CraftelService::open(&db).unwrap();
    let p = service.register_project("p", &root).unwrap();
    service.create_task(&p.id, "x", "y").unwrap();
    fs::create_dir_all(root.join("craftel/tasks/T0001-x/notes")).unwrap();
    (t, db, p)
}
#[test]
fn reconcile_ingests_only_owned_markdown_and_marks_deletions() {
    let (_t, db, p) = setup();
    let root = &p.work_dir;
    fs::write(root.join("craftel/INDEX.md"), "# Index").unwrap();
    fs::write(root.join("craftel/tasks/T0001-x/SPEC.md"), "# Spec").unwrap();
    fs::write(root.join("craftel/tasks/T0001-x/TASK.md"), "# managed").unwrap();
    fs::write(root.join("craftel/tasks/T0001-x/notes/a.md"), "# Note").unwrap();
    fs::write(root.join("craftel/tasks/T0001-x/notes/a.md.tmp"), "no").unwrap();
    reconcile_project(&db, &p).unwrap();
    let docs = DocumentRepository::open(&db).unwrap();
    assert_eq!(docs.list(&p.id, false).unwrap().len(), 3);
    drop(docs);
    fs::remove_file(root.join("craftel/tasks/T0001-x/SPEC.md")).unwrap();
    reconcile_project(&db, &p).unwrap();
    assert!(
        !DocumentRepository::open(&db)
            .unwrap()
            .read(&p.id, "craftel/tasks/T0001-x/SPEC.md")
            .unwrap()
            .present
    );
}
#[test]
fn linux_watcher_coalesces_changes_and_handles_rename_delete() {
    let (_t, db, p) = setup();
    let spec = p.work_dir.join("craftel/tasks/T0001-x/SPEC.md");
    fs::write(&spec, "# one").unwrap();
    reconcile_project(&db, &p).unwrap();
    let baseline = DocumentRepository::open(&db)
        .unwrap()
        .revisions(&p.id, "craftel/tasks/T0001-x/SPEC.md")
        .unwrap()
        .len();
    let watcher = ProjectWatcher::start(db.clone(), p.clone()).unwrap();
    for n in 0..5 {
        fs::write(&spec, format!("# value {n}")).unwrap();
    }
    thread::sleep(Duration::from_millis(700));
    let renamed = spec.parent().unwrap().join("notes/other.md");
    fs::rename(&spec, &renamed).unwrap();
    for _ in 0..40 {
        if DocumentRepository::open(&db)
            .unwrap()
            .read(&p.id, "craftel/tasks/T0001-x/notes/other.md")
            .is_ok_and(|document| document.present)
        {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        DocumentRepository::open(&db)
            .unwrap()
            .read(&p.id, "craftel/tasks/T0001-x/notes/other.md")
            .is_ok_and(|document| document.present),
        "watcher did not index the renamed document"
    );
    fs::remove_file(&renamed).unwrap();
    for _ in 0..40 {
        if DocumentRepository::open(&db)
            .unwrap()
            .read(&p.id, "craftel/tasks/T0001-x/notes/other.md")
            .is_ok_and(|document| !document.present)
        {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    watcher.shutdown().unwrap();
    let docs = DocumentRepository::open(&db).unwrap();
    assert!(
        !docs
            .read(&p.id, "craftel/tasks/T0001-x/notes/other.md")
            .unwrap()
            .present
    );
    assert!(
        docs.revisions(&p.id, "craftel/tasks/T0001-x/SPEC.md")
            .unwrap()
            .len()
            <= baseline + 1
    );
}

fn next_change(
    receiver: &mpsc::Receiver<craftel_core::documents::DocumentChanged>,
    expected: &str,
) -> craftel_core::documents::DocumentChanged {
    receiver
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_else(|error| panic!("timed out waiting for {expected}: {error}"))
}

fn drain(receiver: &mpsc::Receiver<craftel_core::documents::DocumentChanged>) {
    while receiver.try_recv().is_ok() {}
}

#[test]
fn service_subscription_reports_watcher_create_edit_delete_and_not_unchanged_scans() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("p");
    fs::create_dir(&root).unwrap();
    let mut service = CraftelService::open(&t.path().join("db")).unwrap();
    let receiver = service.subscribe_document_changes().unwrap();
    let project = service.register_project("p", &root).unwrap();
    service.create_task(&project.id, "x", "y").unwrap();
    thread::sleep(Duration::from_millis(600));
    drain(&receiver);

    service.reconcile_documents(&project.id).unwrap();
    assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());

    let path = root.join("craftel/tasks/T0001-x/notes/new.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "# created").unwrap();
    let created = next_change(&receiver, "create");
    assert_eq!(
        (created.path.as_str(), created.change),
        ("craftel/tasks/T0001-x/notes/new.md", DocumentChange::Create)
    );

    fs::write(&path, "# edited").unwrap();
    assert_eq!(next_change(&receiver, "edit").change, DocumentChange::Edit);
    fs::remove_file(&path).unwrap();
    assert_eq!(
        next_change(&receiver, "delete").change,
        DocumentChange::Delete
    );
}

#[test]
fn explicit_write_emits_once_despite_filesystem_echo() {
    let (_t, db, project) = setup();
    let mut service = CraftelService::open(&db).unwrap();
    let receiver = service.subscribe_document_changes().unwrap();
    thread::sleep(Duration::from_millis(600));
    drain(&receiver);
    let path = "craftel/tasks/T0001-x/SPEC.md";
    let current = service.read_document(&project.id, path).unwrap();
    service
        .write_document(
            &project.id,
            path,
            "# explicit",
            ExpectedDocumentState::Present(current.content_hash),
        )
        .unwrap();
    assert_eq!(
        next_change(&receiver, "explicit edit").change,
        DocumentChange::Edit
    );
    assert!(receiver.recv_timeout(Duration::from_millis(700)).is_err());
}

#[test]
fn removal_and_service_drop_stop_notification_producers() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("p");
    fs::create_dir(&root).unwrap();
    let mut service = CraftelService::open(&t.path().join("db")).unwrap();
    let receiver = service.subscribe_document_changes().unwrap();
    let project = service.register_project("p", &root).unwrap();
    service.create_task(&project.id, "x", "y").unwrap();
    thread::sleep(Duration::from_millis(600));
    drain(&receiver);
    service.remove_project(&project.id).unwrap();
    fs::write(root.join("craftel/INDEX.md"), "# after removal").unwrap();
    assert!(receiver.recv_timeout(Duration::from_millis(500)).is_err());
    drop(service);
    assert!(matches!(
        receiver.recv_timeout(Duration::from_secs(1)),
        Err(mpsc::RecvTimeoutError::Disconnected)
    ));
}
