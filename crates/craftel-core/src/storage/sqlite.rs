use std::path::{Path, PathBuf};

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use uuid::Uuid;

use crate::domain::{Project, Stage, Task, WorkflowAction};

use super::StorageError;

const MIGRATION: &str = include_str!("../../migrations/001_foundation.sql");
const DOCUMENT_MIGRATION: &str = include_str!("../../migrations/002_documents.sql");
const DOCUMENT_HARDENING_MIGRATION: &str =
    include_str!("../../migrations/003_document_hardening.sql");
const DOCUMENT_STATUS_MIGRATION: &str = include_str!("../../migrations/004_document_status.sql");
const HARNESS_MIGRATION: &str = include_str!("../../migrations/005_harness.sql");
const SUPERVISOR_HARDENING_MIGRATION: &str =
    include_str!("../../migrations/006_supervisor_hardening.sql");
const AUTOMATION_MIGRATION: &str = include_str!("../../migrations/007_automation.sql");

pub struct NewTask {
    pub project_id: String,
    pub title: String,
    pub content: String,
    pub relative_dir: PathBuf,
}

impl NewTask {
    pub fn new(
        project_id: &str,
        title: &str,
        content: &str,
        relative_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            title: title.into(),
            content: content.into(),
            relative_dir: relative_dir.into(),
        }
    }
}

pub struct UpdateTask {
    pub project_id: String,
    pub task_id: String,
    pub title: String,
    pub content: String,
}

impl UpdateTask {
    pub fn new(project_id: &str, task_id: &str, title: &str, content: &str) -> Self {
        Self {
            project_id: project_id.into(),
            task_id: task_id.into(),
            title: title.into(),
            content: content.into(),
        }
    }
}

pub struct SqliteRepository {
    connection: Connection,
}

impl SqliteRepository {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        if !connection.query_row("PRAGMA foreign_keys", [], |row| row.get::<_, bool>(0))? {
            return Err(StorageError::InvalidData(
                "SQLite foreign keys could not be enabled".into(),
            ));
        }
        connection.execute_batch(MIGRATION)?;
        connection.execute_batch(DOCUMENT_MIGRATION)?;
        if !column_exists(&connection, "document_snapshots", "sequence")? {
            connection.execute_batch(DOCUMENT_HARDENING_MIGRATION)?;
        }
        connection.execute_batch(DOCUMENT_STATUS_MIGRATION)?;
        connection.execute_batch(HARNESS_MIGRATION)?;
        connection.execute_batch(SUPERVISOR_HARDENING_MIGRATION)?;
        if !column_exists(&connection, "runs", "stage_at_start")? {
            connection.execute_batch(AUTOMATION_MIGRATION)?;
        }
        Ok(Self { connection })
    }

    pub fn foreign_keys_enabled(&self) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))?)
    }

    pub fn register_project(
        &mut self,
        name: &str,
        work_dir: &Path,
    ) -> Result<Project, StorageError> {
        let canonical = work_dir.canonicalize()?;
        if !canonical.is_dir() {
            return Err(StorageError::InvalidData(
                "project path is not a directory".into(),
            ));
        }
        let id = Uuid::new_v4().to_string();
        let now = timestamp(Utc::now());
        self.connection.execute("INSERT INTO projects(id,name,work_dir,created_at,last_opened_at) VALUES(?1,?2,?3,?4,?4)", params![id, name, canonical.to_string_lossy(), now])?;
        self.get_project(&id)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, StorageError> {
        let mut statement = self.connection.prepare("SELECT id,name,work_dir,created_at,last_opened_at FROM projects ORDER BY last_opened_at DESC, id ASC")?;
        let values = statement
            .query_map([], project_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(values)
    }

    pub fn touch_project(&mut self, id: &str) -> Result<Project, StorageError> {
        let changed = self.connection.execute(
            "UPDATE projects SET last_opened_at=?1 WHERE id=?2",
            params![timestamp(Utc::now()), id],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound);
        }
        self.get_project(id)
    }

    pub fn remove_project(&mut self, id: &str) -> Result<(), StorageError> {
        if self
            .connection
            .execute("DELETE FROM projects WHERE id=?1", [id])?
            == 0
        {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    pub fn create_task(&mut self, input: NewTask) -> Result<Task, StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO task_counters(project_id,next_number) VALUES(?1,2) ON CONFLICT(project_id) DO UPDATE SET next_number=next_number+1", [&input.project_id])?;
        let number: i64 = tx.query_row(
            "SELECT next_number-1 FROM task_counters WHERE project_id=?1",
            [&input.project_id],
            |r| r.get(0),
        )?;
        let id = format!("T{number:04}");
        let now = timestamp(Utc::now());
        tx.execute("INSERT INTO tasks(project_id,task_number,id,title,content,stage,relative_dir,review_approved,projection_dirty,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,'inbox',?6,0,1,?7,?7)", params![input.project_id, number, id, input.title, input.content, input.relative_dir.to_string_lossy(), now])?;
        tx.commit()?;
        self.get_task(&input.project_id, &id)
    }

    pub fn get_task(&self, project_id: &str, task_id: &str) -> Result<Task, StorageError> {
        self.connection.query_row("SELECT id,project_id,title,content,stage,relative_dir,review_approved,created_at,updated_at FROM tasks WHERE project_id=?1 AND id=?2", params![project_id,task_id], task_from_row).optional()?.ok_or(StorageError::NotFound)
    }

    pub fn list_tasks(&self, project_id: &str) -> Result<Vec<Task>, StorageError> {
        let mut statement = self.connection.prepare("SELECT id,project_id,title,content,stage,relative_dir,review_approved,created_at,updated_at FROM tasks WHERE project_id=?1 ORDER BY task_number ASC")?;
        Ok(statement
            .query_map([project_id], task_from_row)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn update_task(&mut self, input: UpdateTask) -> Result<Task, StorageError> {
        let tx = self.connection.transaction()?;
        let changed = tx.execute("UPDATE tasks SET title=?1,content=?2,updated_at=?3,projection_dirty=1 WHERE project_id=?4 AND id=?5", params![input.title,input.content,timestamp(Utc::now()),input.project_id,input.task_id])?;
        if changed == 0 {
            return Err(StorageError::NotFound);
        }
        tx.commit()?;
        self.get_task(&input.project_id, &input.task_id)
    }

    pub fn apply_transition(
        &mut self,
        project_id: &str,
        task_id: &str,
        action: WorkflowAction,
    ) -> Result<Task, StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if matches!(action, WorkflowAction::Move(_) | WorkflowAction::Next) {
            let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM runs WHERE project_id=?1 AND task_id=?2 AND state IN ('queued','running'))", params![project_id,task_id], |row| row.get(0))?;
            if active {
                return Err(StorageError::ActiveRun);
            }
        }
        let mut task = tx.query_row("SELECT id,project_id,title,content,stage,relative_dir,review_approved,created_at,updated_at FROM tasks WHERE project_id=?1 AND id=?2", params![project_id,task_id], task_from_row).optional()?.ok_or(StorageError::NotFound)?;
        let event = task.apply_action(action, Utc::now())?;
        tx.execute("UPDATE tasks SET stage=?1,review_approved=?2,updated_at=?3,projection_dirty=1 WHERE project_id=?4 AND id=?5", params![task.stage.to_string(),task.review_approved,timestamp(task.updated_at),project_id,task_id])?;
        tx.execute("INSERT INTO workflow_events(project_id,task_id,action,from_stage,to_stage,outcome,timestamp) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![project_id,task_id,serde_json::to_string(&event.action).map_err(|e| StorageError::InvalidData(e.to_string()))?,event.from_stage.to_string(),event.to_stage.to_string(),serde_json::to_string(&event.outcome).map_err(|e| StorageError::InvalidData(e.to_string()))?,timestamp(event.timestamp)])?;
        tx.commit()?;
        Ok(task)
    }

    pub fn mark_projection_clean(
        &mut self,
        project_id: &str,
        task_id: &str,
    ) -> Result<(), StorageError> {
        if self.connection.execute(
            "UPDATE tasks SET projection_dirty=0 WHERE project_id=?1 AND id=?2",
            params![project_id, task_id],
        )? == 0
        {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    pub fn delete_new_task_after_projection_failure(
        &mut self,
        project_id: &str,
        task_id: &str,
    ) -> Result<(), StorageError> {
        if self.connection.execute(
            "DELETE FROM tasks WHERE project_id=?1 AND id=?2",
            params![project_id, task_id],
        )? == 0
        {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    pub fn set_new_task_relative_dir(
        &mut self,
        project_id: &str,
        task_id: &str,
        relative_dir: &Path,
    ) -> Result<(), StorageError> {
        if self.connection.execute(
            "UPDATE tasks SET relative_dir=?1 WHERE project_id=?2 AND id=?3 AND projection_dirty=1",
            params![relative_dir.to_string_lossy(), project_id, task_id],
        )? == 0
        {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    pub fn get_project(&self, id: &str) -> Result<Project, StorageError> {
        self.connection
            .query_row(
                "SELECT id,name,work_dir,created_at,last_opened_at FROM projects WHERE id=?1",
                [id],
                project_from_row,
            )
            .optional()?
            .ok_or(StorageError::NotFound)
    }

    pub fn projection_dirty(&self, project_id: &str, task_id: &str) -> Result<bool, StorageError> {
        self.connection
            .query_row(
                "SELECT projection_dirty FROM tasks WHERE project_id=?1 AND id=?2",
                params![project_id, task_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StorageError::NotFound)
    }
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool, StorageError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    Ok(rows.filter_map(Result::ok).any(|name| name == column))
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Nanos, true)
}
fn parse_time(value: String) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|v| v.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                value.len(),
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })
}
fn project_from_row(row: &Row<'_>) -> rusqlite::Result<Project> {
    let work_dir = PathBuf::from(row.get::<_, String>(2)?);
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        available: work_dir.is_dir(),
        work_dir,
        created_at: parse_time(row.get(3)?)?,
        last_opened_at: parse_time(row.get(4)?)?,
    })
}
fn task_from_row(row: &Row<'_>) -> rusqlite::Result<Task> {
    let stage_text: String = row.get(4)?;
    let stage = match stage_text.as_str() {
        "inbox" => Stage::Inbox,
        "defining" => Stage::Defining,
        "implementation" => Stage::Implementation,
        "reviewing" => Stage::Reviewing,
        "done" => Stage::Done,
        _ => {
            return Err(rusqlite::Error::InvalidColumnType(
                4,
                "stage".into(),
                rusqlite::types::Type::Text,
            ));
        }
    };
    Ok(Task {
        id: row.get(0)?,
        project_id: row.get(1)?,
        title: row.get(2)?,
        content: row.get(3)?,
        stage,
        relative_dir: PathBuf::from(row.get::<_, String>(5)?),
        review_approved: row.get(6)?,
        created_at: parse_time(row.get(7)?)?,
        updated_at: parse_time(row.get(8)?)?,
    })
}
