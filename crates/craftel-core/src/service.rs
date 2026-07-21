use crate::{
    documents::{
        Document, DocumentCause, DocumentChange, DocumentChanged, DocumentError,
        DocumentProjectStatus, DocumentRepository, DocumentSnapshot, ExpectedDocumentState,
        ProjectWatcher, atomic_write_task, eligible, initialize_index, initialize_spec,
        reconcile_project, slugify,
    },
    domain::{Project, Stage, Task, WorkflowAction},
    storage::{NewTask, SqliteRepository, StorageError, UpdateTask},
};
use sha2::Digest;
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("{0} must not be blank")]
    Validation(&'static str),
    #[error("task document projection failed: {0}")]
    Projection(#[source] std::io::Error),
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error("conflict")]
    Conflict,
    #[error("unavailable")]
    Unavailable,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub struct CraftelService {
    repository: SqliteRepository,
    database_path: PathBuf,
    watchers: HashMap<String, ProjectWatcher>,
    notifications: mpsc::SyncSender<DocumentChanged>,
    subscription: Option<mpsc::Receiver<DocumentChanged>>,
}

impl CraftelService {
    pub fn open(path: &Path) -> Result<Self, ServiceError> {
        let (notifications, subscription) = mpsc::sync_channel(256);
        let mut service = Self {
            repository: SqliteRepository::open(path)?,
            database_path: path.to_path_buf(),
            watchers: HashMap::new(),
            notifications,
            subscription: Some(subscription),
        };
        service.repair_available_projects()?;
        DocumentRepository::open(path)?.prune(chrono::Utc::now())?;
        service.start_available_document_watchers();
        Ok(service)
    }
    pub fn subscribe_document_changes(&mut self) -> Option<mpsc::Receiver<DocumentChanged>> {
        self.subscription.take()
    }
    pub fn register_project(&mut self, name: &str, path: &Path) -> Result<Project, ServiceError> {
        let project = self.repository.register_project(name, path)?;
        self.activate_project(&project);
        Ok(project)
    }
    pub fn list_projects(&self) -> Result<Vec<Project>, ServiceError> {
        Ok(self.repository.list_projects()?)
    }
    pub fn open_project(&mut self, id: &str) -> Result<Project, ServiceError> {
        let project = self.repository.touch_project(id)?;
        self.activate_project(&project);
        Ok(project)
    }
    pub fn remove_project(&mut self, id: &str) -> Result<(), ServiceError> {
        self.watchers.remove(id);
        Ok(self.repository.remove_project(id)?)
    }

    pub fn create_task(
        &mut self,
        project_id: &str,
        title: &str,
        content: &str,
    ) -> Result<Task, ServiceError> {
        validate(title, content)?;
        let project = self.repository.get_project(project_id)?;
        initialize_index(&project.work_dir.join("craftel/INDEX.md"))
            .map_err(ServiceError::Projection)?;
        // The final relative path depends on the allocated ID, so reserve using a placeholder then keep the immutable slug path.
        let task = self.repository.create_task(NewTask::new(
            project_id,
            title.trim(),
            content.trim(),
            "pending",
        ))?;
        let relative = format!("craftel/tasks/{}-{}", task.id, slugify(title));
        // Update relative_dir without changing public metadata via the narrowly scoped repository operation.
        self.repository
            .set_new_task_relative_dir(project_id, &task.id, Path::new(&relative))?;
        let task = self.repository.get_task(project_id, &task.id)?;
        let final_dir = project.work_dir.join(&relative);
        let temp_dir = project.work_dir.join("craftel/tasks").join(format!(
            ".{}.{}.tmp",
            task.id,
            Uuid::new_v4()
        ));
        let mut temp_created = false;
        let mut final_created = false;
        let result = (|| -> std::io::Result<()> {
            fs::create_dir_all(temp_dir.parent().unwrap())?;
            fs::create_dir(&temp_dir)?;
            temp_created = true;
            atomic_write_task(&temp_dir.join("TASK.md"), &task)?;
            initialize_spec(&temp_dir.join("SPEC.md"))?;
            fs::rename(&temp_dir, &final_dir)?;
            temp_created = false;
            final_created = true;
            Ok(())
        })();
        if let Err(error) = result {
            if temp_created {
                let _ = fs::remove_dir_all(&temp_dir);
            }
            if final_created {
                let _ = fs::remove_dir_all(&final_dir);
            }
            let _ = self
                .repository
                .delete_new_task_after_projection_failure(project_id, &task.id);
            return Err(ServiceError::Projection(error));
        }
        self.repository
            .mark_projection_clean(project_id, &task.id)?;
        // Task creation is durable at this point. Watch activation is best-effort and
        // must never turn a committed task into an API failure.
        self.activate_project(&project);
        Ok(task)
    }
    pub fn list_tasks(&self, project_id: &str) -> Result<Vec<Task>, ServiceError> {
        Ok(self.repository.list_tasks(project_id)?)
    }
    pub fn get_task(&mut self, project_id: &str, task_id: &str) -> Result<Task, ServiceError> {
        self.repair_task(project_id, task_id)?;
        Ok(self.repository.get_task(project_id, task_id)?)
    }
    pub fn update_task(
        &mut self,
        project_id: &str,
        task_id: &str,
        title: &str,
        content: &str,
    ) -> Result<Task, ServiceError> {
        validate(title, content)?;
        let task = self.repository.update_task(UpdateTask::new(
            project_id,
            task_id,
            title.trim(),
            content.trim(),
        ))?;
        self.project_task(&task)?;
        Ok(task)
    }
    pub fn apply(
        &mut self,
        project_id: &str,
        task_id: &str,
        action: WorkflowAction,
    ) -> Result<Task, ServiceError> {
        let task = self
            .repository
            .apply_transition(project_id, task_id, action)?;
        self.project_task(&task)?;
        Ok(task)
    }
    pub fn move_task(&mut self, p: &str, t: &str, stage: Stage) -> Result<Task, ServiceError> {
        self.apply(p, t, WorkflowAction::Move(stage))
    }
    pub fn next_task(&mut self, p: &str, t: &str) -> Result<Task, ServiceError> {
        self.apply(p, t, WorkflowAction::Next)
    }
    pub fn pass_task(&mut self, p: &str, t: &str) -> Result<Task, ServiceError> {
        self.apply(p, t, WorkflowAction::Pass)
    }
    pub fn fail_task(&mut self, p: &str, t: &str) -> Result<Task, ServiceError> {
        self.apply(p, t, WorkflowAction::Fail)
    }
    pub fn reconcile_documents(&mut self, project_id: &str) -> Result<(), ServiceError> {
        let p = self.available_project(project_id)?;
        match reconcile_project(&self.database_path, &p) {
            Ok(changes) => {
                DocumentRepository::record_status(project_id, &self.database_path, None);
                self.dispatch(changes);
                Ok(())
            }
            Err(error) => {
                DocumentRepository::record_status(
                    project_id,
                    &self.database_path,
                    Some(&error.to_string()),
                );
                Err(error.into())
            }
        }
    }
    pub fn list_documents(&self, p: &str, deleted: bool) -> Result<Vec<Document>, ServiceError> {
        self.available_project(p)?;
        Ok(DocumentRepository::open(&self.database_path)?.list(p, deleted)?)
    }
    pub fn document_status(&self, p: &str) -> Result<DocumentProjectStatus, ServiceError> {
        self.available_project(p)?;
        Ok(DocumentRepository::open(&self.database_path)?.status(p)?)
    }
    pub fn read_document(&self, p: &str, path: &str) -> Result<Document, ServiceError> {
        self.available_project(p)?;
        Ok(DocumentRepository::open(&self.database_path)?.read(p, path)?)
    }
    pub fn search_documents(&self, p: &str, q: &str) -> Result<Vec<Document>, ServiceError> {
        self.available_project(p)?;
        Ok(DocumentRepository::open(&self.database_path)?.search(p, q)?)
    }
    pub fn list_document_revisions(
        &self,
        p: &str,
        path: &str,
    ) -> Result<Vec<DocumentSnapshot>, ServiceError> {
        self.available_project(p)?;
        Ok(DocumentRepository::open(&self.database_path)?.revisions(p, path)?)
    }
    pub fn write_document(
        &mut self,
        p: &str,
        path: &str,
        content: &str,
        expected: ExpectedDocumentState,
    ) -> Result<Document, ServiceError> {
        let project = self.available_project(p)?;
        let target = self.resolve_document_path(&project, path, false)?;
        let _lease = DocumentRepository::acquire_mutation(&self.database_path, p, "$project")?;
        let mut repo = DocumentRepository::open(&self.database_path)?;
        let current = repo.read(p, path)?;
        if !matches_disk_state(&target, &expected)? {
            return Err(ServiceError::Conflict);
        }
        if !current.present {
            return Err(ServiceError::Conflict);
        }
        atomic_write(&target, content.as_bytes())?;
        let m = fs::metadata(&target)?;
        let document = repo.ingest(
            p,
            path,
            current.task_id.as_deref(),
            content.as_bytes(),
            DocumentCause::Edit,
            chrono::Utc::now(),
            0,
            m.len() as i64,
            false,
        )?;
        self.notify(DocumentChanged {
            project_id: p.to_string(),
            path: path.to_string(),
            change: DocumentChange::Edit,
        });
        Ok(document)
    }
    pub fn restore_document_revision(
        &mut self,
        p: &str,
        path: &str,
        id: &str,
        expected: ExpectedDocumentState,
    ) -> Result<Document, ServiceError> {
        let project = self.available_project(p)?;
        let target = self.resolve_document_path(&project, path, true)?;
        let _lease = DocumentRepository::acquire_mutation(&self.database_path, p, "$project")?;
        let mut repo = DocumentRepository::open(&self.database_path)?;
        let current = repo.read(p, path)?;
        let snapshot = repo.snapshot(id)?;
        if snapshot.project_id != p || snapshot.relative_path != path {
            return Err(ServiceError::Conflict);
        }
        if !matches_disk_state(&target, &expected)? {
            return Err(ServiceError::Conflict);
        }
        if path != "craftel/INDEX.md" {
            fs::create_dir_all(target.parent().ok_or(DocumentError::InvalidPath)?)?;
        }
        atomic_write(&target, &snapshot.content)?;
        let m = fs::metadata(&target)?;
        let document = repo.ingest(
            p,
            path,
            current.task_id.as_deref(),
            &snapshot.content,
            DocumentCause::Restore,
            chrono::Utc::now(),
            0,
            m.len() as i64,
            true,
        )?;
        self.notify(DocumentChanged {
            project_id: p.to_string(),
            path: path.to_string(),
            change: DocumentChange::Restore,
        });
        Ok(document)
    }
    fn available_project(&self, id: &str) -> Result<Project, ServiceError> {
        let p = self.repository.get_project(id)?;
        if !p.available {
            return Err(ServiceError::Unavailable);
        }
        Ok(p)
    }
    fn resolve_document_path(
        &self,
        project: &Project,
        relative: &str,
        allow_missing: bool,
    ) -> Result<PathBuf, ServiceError> {
        let rel = Path::new(relative);
        if !eligible(rel) {
            return Err(DocumentError::InvalidPath.into());
        }
        let owned = relative == "craftel/INDEX.md"
            || self
                .repository
                .list_tasks(&project.id)?
                .iter()
                .any(|task| rel.strip_prefix(&task.relative_dir).is_ok());
        if !owned {
            return Err(DocumentError::InvalidPath.into());
        }
        let target = project.work_dir.join(rel);
        let mut cursor = target.as_path();
        loop {
            match fs::symlink_metadata(cursor) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(DocumentError::InvalidPath.into());
                }
                Ok(_) => {}
                Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            if cursor == project.work_dir {
                break;
            }
            cursor = cursor.parent().ok_or(DocumentError::InvalidPath)?;
        }
        let canonical_work = project.work_dir.canonicalize()?;
        let existing = target
            .ancestors()
            .find(|p| p.exists())
            .ok_or(DocumentError::InvalidPath)?;
        if !existing.canonicalize()?.starts_with(canonical_work) {
            return Err(DocumentError::InvalidPath.into());
        }
        Ok(target)
    }
    fn project_task(&mut self, task: &Task) -> Result<(), ServiceError> {
        let project = self.repository.get_project(&task.project_id)?;
        atomic_write_task(
            &project.work_dir.join(&task.relative_dir).join("TASK.md"),
            task,
        )
        .map_err(ServiceError::Projection)?;
        self.repository
            .mark_projection_clean(&task.project_id, &task.id)?;
        Ok(())
    }
    fn repair_task(&mut self, p: &str, t: &str) -> Result<(), ServiceError> {
        let task = self.repository.get_task(p, t)?;
        let project = self.repository.get_project(p)?;
        let task_md = project.work_dir.join(&task.relative_dir).join("TASK.md");
        if project.available && (self.repository.projection_dirty(p, t)? || !task_md.is_file()) {
            self.project_task(&task)?;
        }
        Ok(())
    }
    fn repair_available_projects(&mut self) -> Result<(), ServiceError> {
        for project in self.repository.list_projects()? {
            if project.available {
                for task in self.repository.list_tasks(&project.id)? {
                    let path = project.work_dir.join(&task.relative_dir).join("TASK.md");
                    if self.repository.projection_dirty(&project.id, &task.id)? || !path.is_file() {
                        self.project_task(&task)?;
                    }
                }
            }
        }
        Ok(())
    }
    fn start_available_document_watchers(&mut self) {
        let Ok(projects) = self.repository.list_projects() else {
            return;
        };
        for project in projects {
            if project.available {
                self.activate_project(&project);
            }
        }
    }
    fn activate_project(&mut self, project: &Project) {
        if !project.available {
            return;
        }
        if initialize_index(&project.work_dir.join("craftel/INDEX.md")).is_err() {
            return;
        }
        // Watch first closes the scan/start race. Both paths call the same idempotent ingest.
        let _ = self.start_document_watcher(project);
        match reconcile_project(&self.database_path, project) {
            Ok(changes) => {
                DocumentRepository::record_status(&project.id, &self.database_path, None);
                self.dispatch(changes);
            }
            Err(error) => DocumentRepository::record_status(
                &project.id,
                &self.database_path,
                Some(&error.to_string()),
            ),
        }
    }
    fn start_document_watcher(&mut self, project: &Project) -> Result<(), ServiceError> {
        if !self.watchers.contains_key(&project.id) {
            let watcher = ProjectWatcher::start_with_notifications(
                self.database_path.clone(),
                project.clone(),
                self.notifications.clone(),
            )?;
            self.watchers.insert(project.id.clone(), watcher);
        }
        Ok(())
    }
    fn notify(&self, change: DocumentChanged) {
        let _ = self.notifications.try_send(change);
    }
    fn dispatch(&self, changes: Vec<DocumentChanged>) {
        for change in changes {
            self.notify(change);
        }
    }
}
fn matches_disk_state(path: &Path, expected: &ExpectedDocumentState) -> Result<bool, ServiceError> {
    match (fs::read(path), expected) {
        (Ok(bytes), ExpectedDocumentState::Present(hash)) => {
            Ok(format!("{:x}", sha2::Sha256::digest(bytes)) == *hash)
        }
        (Err(error), ExpectedDocumentState::Missing)
            if error.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(true)
        }
        (Ok(_), ExpectedDocumentState::Missing) | (Err(_), ExpectedDocumentState::Present(_)) => {
            Ok(false)
        }
        (Err(error), _) => Err(error.into()),
    }
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ServiceError> {
    let tmp = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        Uuid::new_v4()
    ));
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map_err(ServiceError::Io)
}
fn validate(title: &str, content: &str) -> Result<(), ServiceError> {
    if title.trim().is_empty() {
        Err(ServiceError::Validation("title"))
    } else if content.trim().is_empty() {
        Err(ServiceError::Validation("content"))
    } else {
        Ok(())
    }
}
