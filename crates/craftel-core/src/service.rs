use crate::{
    documents::{atomic_write_task, initialize_index, initialize_spec, slugify},
    domain::{Project, Stage, Task, WorkflowAction},
    storage::{NewTask, SqliteRepository, StorageError, UpdateTask},
};
use std::{fs, path::Path};
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
}

pub struct CraftelService {
    repository: SqliteRepository,
}

impl CraftelService {
    pub fn open(path: &Path) -> Result<Self, ServiceError> {
        let mut service = Self {
            repository: SqliteRepository::open(path)?,
        };
        service.repair_available_projects()?;
        Ok(service)
    }
    pub fn register_project(&mut self, name: &str, path: &Path) -> Result<Project, ServiceError> {
        Ok(self.repository.register_project(name, path)?)
    }
    pub fn list_projects(&self) -> Result<Vec<Project>, ServiceError> {
        Ok(self.repository.list_projects()?)
    }
    pub fn open_project(&mut self, id: &str) -> Result<Project, ServiceError> {
        Ok(self.repository.touch_project(id)?)
    }
    pub fn remove_project(&mut self, id: &str) -> Result<(), ServiceError> {
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
