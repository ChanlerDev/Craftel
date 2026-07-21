use super::{
    DocumentCause, DocumentChange, DocumentChanged, DocumentError, DocumentRepository,
    path::{eligible, task_id},
};
use crate::domain::Project;
use crate::storage::SqliteRepository;
use chrono::Utc;
use sha2::Digest;
use std::{collections::HashSet, fs, path::Path, thread, time::Duration};
use walkdir::WalkDir;
pub fn reconcile_project(db: &Path, p: &Project) -> Result<Vec<DocumentChanged>, DocumentError> {
    reconcile_project_with_cause(db, p, DocumentCause::Scan)
}
pub(crate) fn reconcile_project_with_cause(
    db: &Path,
    p: &Project,
    cause: DocumentCause,
) -> Result<Vec<DocumentChanged>, DocumentError> {
    let _lease = DocumentRepository::acquire_mutation(db, &p.id, "$project")?;
    let root = p.work_dir.join("craftel");
    let mut repo = DocumentRepository::open(db)?;
    let tasks: HashSet<_> = SqliteRepository::open(db)
        .map_err(|e| DocumentError::Io(std::io::Error::other(e)))?
        .list_tasks(&p.id)
        .map_err(|e| DocumentError::Io(std::io::Error::other(e)))?
        .into_iter()
        .map(|t| t.relative_dir)
        .collect();
    let mut seen = HashSet::new();
    let mut changes = Vec::new();
    let root_present = match fs::metadata(&root) {
        Ok(metadata) if metadata.is_dir() => true,
        Ok(_) => {
            return Err(DocumentError::Io(std::io::Error::other(
                "craftel root is not a directory",
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    if root_present {
        let canonical_root = root.canonicalize()?;
        for entry in WalkDir::new(&root).follow_links(false) {
            let e = entry.map_err(|error| DocumentError::Io(std::io::Error::other(error)))?;
            if e.file_type().is_symlink() || !e.file_type().is_file() {
                continue;
            }
            let rel = e
                .path()
                .strip_prefix(&p.work_dir)
                .map_err(|_| DocumentError::InvalidPath)?;
            let in_registered_task = rel == Path::new("craftel/INDEX.md")
                || tasks.iter().any(|task| {
                    rel.strip_prefix(task).is_ok_and(|tail| {
                        eligible(
                            &Path::new("craftel")
                                .join("tasks")
                                .join("T-placeholder")
                                .join(tail),
                        )
                    })
                });
            if !eligible(rel)
                || !in_registered_task
                || !e.path().canonicalize()?.starts_with(&canonical_root)
            {
                continue;
            }
            let (bytes, m) = stable_read(e.path())?;
            let path = rel.to_string_lossy().replace('\\', "/");
            let previous = repo.read(&p.id, &path).ok();
            repo.ingest(
                &p.id,
                &path,
                task_id(rel).as_deref(),
                &bytes,
                cause,
                Utc::now(),
                modified(&m),
                m.len() as i64,
                false,
            )?;
            if previous.as_ref().is_none_or(|document| {
                !document.present
                    || document.content_hash != format!("{:x}", sha2::Sha256::digest(&bytes))
            }) {
                changes.push(DocumentChanged {
                    project_id: p.id.clone(),
                    path: path.clone(),
                    change: if previous.as_ref().is_some_and(|document| document.present) {
                        DocumentChange::Edit
                    } else {
                        DocumentChange::Create
                    },
                });
            }
            seen.insert(path);
        }
    }
    // Only a successful traversal or a confirmed NotFound may reconcile absence.
    // Permission and other metadata/walk failures return above and preserve the index.
    for d in repo.list(&p.id, true)? {
        if d.present && !seen.contains(&d.relative_path) {
            repo.mark_deleted(&p.id, &d.relative_path, Utc::now())?;
            changes.push(DocumentChanged {
                project_id: p.id.clone(),
                path: d.relative_path,
                change: DocumentChange::Delete,
            });
        }
    }
    repo.prune(Utc::now())?;
    Ok(changes)
}
pub(crate) fn process_path(
    db: &Path,
    p: &Project,
    absolute: &Path,
    remove: bool,
) -> Result<Vec<DocumentChanged>, DocumentError> {
    let relative = match absolute.strip_prefix(&p.work_dir) {
        Ok(_) if absolute.is_dir() && !remove => {
            let mut changes = Vec::new();
            for entry in WalkDir::new(absolute).follow_links(false) {
                let entry =
                    entry.map_err(|error| DocumentError::Io(std::io::Error::other(error)))?;
                if entry.file_type().is_file() {
                    changes.extend(process_path(db, p, entry.path(), false)?);
                }
            }
            return Ok(changes);
        }
        Ok(value) if eligible(value) => value,
        _ => return Ok(Vec::new()),
    };
    let path = relative.to_string_lossy().replace('\\', "/");
    let _lease = DocumentRepository::acquire_mutation(db, &p.id, "$project")?;
    let mut repo = DocumentRepository::open(db)?;
    let previous = repo.read(&p.id, &path).ok();
    if remove || !absolute.is_file() {
        if previous.as_ref().is_some_and(|document| document.present) {
            repo.mark_deleted(&p.id, &path, Utc::now())?;
            return Ok(vec![DocumentChanged {
                project_id: p.id.clone(),
                path,
                change: DocumentChange::Delete,
            }]);
        }
        return Ok(Vec::new());
    }
    let tasks: HashSet<_> = SqliteRepository::open(db)
        .map_err(|e| DocumentError::Io(std::io::Error::other(e)))?
        .list_tasks(&p.id)
        .map_err(|e| DocumentError::Io(std::io::Error::other(e)))?
        .into_iter()
        .map(|task| task.relative_dir)
        .collect();
    if relative != Path::new("craftel/INDEX.md")
        && !tasks.iter().any(|task| relative.strip_prefix(task).is_ok())
    {
        return Ok(Vec::new());
    }
    let (bytes, metadata) = stable_read(absolute)?;
    let hash = format!("{:x}", sha2::Sha256::digest(&bytes));
    repo.ingest(
        &p.id,
        &path,
        task_id(relative).as_deref(),
        &bytes,
        DocumentCause::Watch,
        Utc::now(),
        modified(&metadata),
        metadata.len() as i64,
        false,
    )?;
    if previous
        .as_ref()
        .is_some_and(|document| document.present && document.content_hash == hash)
    {
        Ok(Vec::new())
    } else {
        Ok(vec![DocumentChanged {
            project_id: p.id.clone(),
            path,
            change: if previous.as_ref().is_some_and(|d| d.present) {
                DocumentChange::Edit
            } else {
                DocumentChange::Create
            },
        }])
    }
}
fn stable_read(path: &Path) -> Result<(Vec<u8>, fs::Metadata), DocumentError> {
    for _ in 0..5 {
        match fs::metadata(path) {
            Ok(a) => {
                let bytes = match fs::read(path) {
                    Ok(bytes) => bytes,
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
                        ) =>
                    {
                        thread::sleep(Duration::from_millis(20));
                        continue;
                    }
                    Err(error) => return Err(error.into()),
                };
                let b = match fs::metadata(path) {
                    Ok(metadata) => metadata,
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
                        ) =>
                    {
                        thread::sleep(Duration::from_millis(20));
                        continue;
                    }
                    Err(error) => return Err(error.into()),
                };
                if (a.len(), modified(&a)) == (b.len(), modified(&b)) {
                    return Ok((bytes, b));
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
                ) =>
            {
                thread::sleep(Duration::from_millis(20))
            }
            Err(e) => return Err(e.into()),
        }
    }
    Err(DocumentError::Io(std::io::Error::other(
        "file remained unavailable or unstable",
    )))
}
fn modified(m: &fs::Metadata) -> i64 {
    m.modified()
        .ok()
        .and_then(|v| v.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|v| v.as_nanos() as i64)
        .unwrap_or(0)
}
