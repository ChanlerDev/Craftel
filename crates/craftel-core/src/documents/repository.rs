use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Component, Path};
use thiserror::Error;
use uuid::Uuid;
const MIGRATIONS: &str = concat!(
    include_str!("../../migrations/001_foundation.sql"),
    include_str!("../../migrations/002_documents.sql"),
    include_str!("../../migrations/004_document_status.sql")
);
#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("invalid_path")]
    InvalidPath,
    #[error("invalid_utf8")]
    InvalidUtf8,
    #[error("not_found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("storage: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DocumentCause {
    Scan,
    Watch,
    Edit,
    Restore,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentChanged {
    pub project_id: String,
    pub path: String,
    pub change: DocumentChange,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentProjectStatus {
    pub project_id: String,
    pub state: String,
    pub error: Option<String>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DocumentChange {
    Create,
    Edit,
    Delete,
    Restore,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", content = "hash", rename_all = "snake_case")]
pub enum ExpectedDocumentState {
    Present(String),
    Missing,
}
impl DocumentCause {
    fn as_str(self) -> &'static str {
        match self {
            Self::Scan => "scan",
            Self::Watch => "watch",
            Self::Edit => "edit",
            Self::Restore => "restore",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub project_id: String,
    pub relative_path: String,
    pub task_id: Option<String>,
    pub title: String,
    pub body: String,
    pub content_hash: String,
    pub present: bool,
    pub indexed_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    pub id: String,
    pub project_id: String,
    pub relative_path: String,
    pub content_hash: String,
    pub content: Vec<u8>,
    pub captured_at: DateTime<Utc>,
    pub cause: String,
    pub sequence: i64,
}
pub struct DocumentRepository {
    connection: Connection,
}
pub struct MutationLease {
    database: std::path::PathBuf,
    project: String,
    path: String,
    owner: String,
}
impl Drop for MutationLease {
    fn drop(&mut self) {
        if let Ok(connection) = Connection::open(&self.database) {
            let _ = connection.execute("DELETE FROM document_mutation_leases WHERE project_id=?1 AND relative_path=?2 AND owner=?3", params![self.project,self.path,self.owner]);
        }
    }
}
impl DocumentRepository {
    pub fn open(path: &Path) -> Result<Self, DocumentError> {
        let c = Connection::open(path)?;
        c.busy_timeout(std::time::Duration::from_secs(5))?;
        c.pragma_update(None, "foreign_keys", true)?;
        c.execute_batch(MIGRATIONS)?;
        c.execute_batch(include_str!("../../migrations/005_harness.sql"))?;
        if !column_exists(&c, "document_snapshots", "sequence")? {
            c.execute_batch(include_str!("../../migrations/003_document_hardening.sql"))?;
        }
        if !column_exists(&c, "document_mutation_leases", "expires_at")? {
            c.execute("ALTER TABLE document_mutation_leases ADD COLUMN expires_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z'", [])?;
        }
        c.execute(
            "DELETE FROM document_mutation_leases WHERE expires_at < ?1",
            [time(Utc::now())],
        )?;
        Ok(Self { connection: c })
    }
    pub fn acquire_mutation(
        path: &Path,
        project: &str,
        relative: &str,
    ) -> Result<MutationLease, DocumentError> {
        validate(relative)?;
        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        let owner = Uuid::new_v4().to_string();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let now = Utc::now();
            connection.execute("DELETE FROM document_mutation_leases WHERE project_id=?1 AND relative_path=?2 AND expires_at < ?3", params![project,relative,time(now)])?;
            match connection.execute("INSERT INTO document_mutation_leases(project_id,relative_path,owner,acquired_at,expires_at) VALUES(?1,?2,?3,?4,?5)", params![project,relative,owner,time(now),time(now+Duration::seconds(30))]) {
                Ok(_) => break,
                Err(rusqlite::Error::SqliteFailure(ref failure, _)) if failure.extended_code == 1555 && std::time::Instant::now() < deadline => std::thread::sleep(std::time::Duration::from_millis(10)),
                Err(rusqlite::Error::SqliteFailure(ref failure, _)) if failure.extended_code == 1555 => return Err(DocumentError::Conflict),
                Err(error) => return Err(error.into()),
            }
        }
        Ok(MutationLease {
            database: path.to_path_buf(),
            project: project.to_string(),
            path: relative.to_string(),
            owner,
        })
    }
    pub fn status(&self, project: &str) -> Result<DocumentProjectStatus, DocumentError> {
        self.connection.query_row(
            "SELECT project_id,state,error,updated_at FROM document_project_status WHERE project_id=?1",
            [project],
            |row| Ok(DocumentProjectStatus { project_id: row.get(0)?, state: row.get(1)?, error: row.get(2)?, updated_at: parse(row.get(3)?)? }),
        ).optional()?.map_or_else(|| Ok(DocumentProjectStatus { project_id: project.into(), state: "healthy".into(), error: None, updated_at: Utc::now() }), Ok)
    }
    pub(crate) fn record_status(project: &str, database: &Path, error: Option<&str>) {
        if let Ok(connection) = Connection::open(database) {
            let bounded = error.map(|value| value.chars().take(1024).collect::<String>());
            let _ = connection.execute(
                "INSERT INTO document_project_status(project_id,state,error,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(project_id) DO UPDATE SET state=excluded.state,error=excluded.error,updated_at=excluded.updated_at",
                params![project, if bounded.is_some() { "error" } else { "healthy" }, bounded, time(Utc::now())],
            );
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn ingest(
        &mut self,
        p: &str,
        path: &str,
        task: Option<&str>,
        bytes: &[u8],
        cause: DocumentCause,
        at: DateTime<Utc>,
        mtime: i64,
        size: i64,
        force: bool,
    ) -> Result<Document, DocumentError> {
        validate(path)?;
        let body = std::str::from_utf8(bytes).map_err(|_| DocumentError::InvalidUtf8)?;
        let hash = format!("{:x}", Sha256::digest(bytes));
        let title = body
            .lines()
            .find_map(|l| l.strip_prefix("# "))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| Path::new(path).file_name().unwrap().to_str().unwrap())
            .to_string();
        let ts = time(at);
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let latest:Option<String>=tx.query_row("SELECT content_hash FROM document_snapshots WHERE project_id=?1 AND relative_path=?2 ORDER BY sequence DESC LIMIT 1",params![p,path],|r|r.get(0)).optional()?;
        tx.execute("INSERT INTO document_index(project_id,relative_path,task_id,title,body,content_hash,file_mtime,file_size,present,indexed_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,1,?9) ON CONFLICT(project_id,relative_path) DO UPDATE SET task_id=excluded.task_id,title=excluded.title,body=excluded.body,content_hash=excluded.content_hash,file_mtime=excluded.file_mtime,file_size=excluded.file_size,present=1,indexed_at=excluded.indexed_at",params![p,path,task,title,body,hash,mtime.to_string(),size,ts])?;
        if force || latest.as_deref() != Some(&hash) {
            let sequence: i64 = tx.query_row("SELECT coalesce(max(sequence),0)+1 FROM document_snapshots WHERE project_id=?1 AND relative_path=?2", params![p,path], |r| r.get(0))?;
            tx.execute("INSERT INTO document_snapshots(id,project_id,relative_path,content_hash,content,captured_at,cause,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![Uuid::new_v4().to_string(),p,path,hash,bytes,ts,cause.as_str(),sequence])?;
        }
        prune_identity(&tx, p, path, at)?;
        tx.commit()?;
        self.read(p, path)
    }
    pub fn mark_deleted(
        &mut self,
        p: &str,
        path: &str,
        at: DateTime<Utc>,
    ) -> Result<(), DocumentError> {
        validate(path)?;
        if self.connection.execute("UPDATE document_index SET present=0,indexed_at=?1 WHERE project_id=?2 AND relative_path=?3",params![time(at),p,path])?==0{return Err(DocumentError::NotFound)}
        Ok(())
    }
    pub fn read(&self, p: &str, path: &str) -> Result<Document, DocumentError> {
        validate(path)?;
        self.connection.query_row("SELECT project_id,relative_path,task_id,title,body,content_hash,present,indexed_at FROM document_index WHERE project_id=?1 AND relative_path=?2",params![p,path],row_doc).optional()?.ok_or(DocumentError::NotFound)
    }
    pub fn list(&self, p: &str, deleted: bool) -> Result<Vec<Document>, DocumentError> {
        let mut s=self.connection.prepare("SELECT project_id,relative_path,task_id,title,body,content_hash,present,indexed_at FROM document_index WHERE project_id=?1 AND (present=1 OR ?2) ORDER BY relative_path")?;
        Ok(s.query_map(params![p, deleted], row_doc)?
            .collect::<Result<Vec<_>, _>>()?)
    }
    pub fn search(&self, p: &str, q: &str) -> Result<Vec<Document>, DocumentError> {
        let mut s=self.connection.prepare("SELECT i.project_id,i.relative_path,i.task_id,i.title,i.body,i.content_hash,i.present,i.indexed_at FROM document_search f JOIN document_index i ON i.rowid=f.rowid WHERE f.project_id=?1 AND document_search MATCH ?2 AND i.present=1 ORDER BY bm25(document_search),i.relative_path")?;
        Ok(s.query_map(params![p, q], row_doc)?
            .collect::<Result<Vec<_>, _>>()?)
    }
    pub fn revisions(&self, p: &str, path: &str) -> Result<Vec<DocumentSnapshot>, DocumentError> {
        validate(path)?;
        let mut s=self.connection.prepare("SELECT id,project_id,relative_path,content_hash,content,captured_at,cause,sequence FROM document_snapshots WHERE project_id=?1 AND relative_path=?2 ORDER BY sequence DESC")?;
        Ok(s.query_map(params![p, path], snapshot_row)?
            .collect::<Result<Vec<_>, _>>()?)
    }
    pub fn snapshot(&self, id: &str) -> Result<DocumentSnapshot, DocumentError> {
        self.connection.query_row("SELECT id,project_id,relative_path,content_hash,content,captured_at,cause,sequence FROM document_snapshots WHERE id=?1",[id],snapshot_row).optional()?.ok_or(DocumentError::NotFound)
    }
    pub fn prune(&mut self, now: DateTime<Utc>) -> Result<(), DocumentError> {
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM document_snapshots WHERE captured_at < ?1 AND sequence < (SELECT max(sequence) FROM document_snapshots s2 WHERE s2.project_id=document_snapshots.project_id AND s2.relative_path=document_snapshots.relative_path)",[time(now-Duration::days(30))])?;
        tx.execute("DELETE FROM document_snapshots WHERE id IN (SELECT id FROM (SELECT id,row_number() OVER(PARTITION BY project_id,relative_path ORDER BY sequence DESC) n FROM document_snapshots) WHERE n>100)",[])?;
        tx.commit()?;
        Ok(())
    }
}
fn prune_identity(
    tx: &rusqlite::Transaction<'_>,
    p: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<(), rusqlite::Error> {
    tx.execute("DELETE FROM document_snapshots WHERE project_id=?1 AND relative_path=?2 AND captured_at < ?3 AND sequence < (SELECT max(sequence) FROM document_snapshots WHERE project_id=?1 AND relative_path=?2)", params![p,path,time(now-Duration::days(30))])?;
    tx.execute("DELETE FROM document_snapshots WHERE id IN (SELECT id FROM document_snapshots WHERE project_id=?1 AND relative_path=?2 ORDER BY sequence DESC LIMIT -1 OFFSET 100)", params![p,path])?;
    Ok(())
}
fn column_exists(c: &Connection, table: &str, column: &str) -> Result<bool, rusqlite::Error> {
    let mut statement = c.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = statement.query_map([], |row| row.get::<_, String>(1))?;
    Ok(names.filter_map(Result::ok).any(|name| name == column))
}
fn validate(p: &str) -> Result<(), DocumentError> {
    let path = Path::new(p);
    if path.is_absolute()
        || p.contains('\\')
        || path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        Err(DocumentError::InvalidPath)
    } else {
        Ok(())
    }
}
fn time(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Nanos, true)
}
fn parse(s: String) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&s)
        .map(|x| x.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                s.len(),
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })
}
fn row_doc(r: &rusqlite::Row<'_>) -> rusqlite::Result<Document> {
    Ok(Document {
        project_id: r.get(0)?,
        relative_path: r.get(1)?,
        task_id: r.get(2)?,
        title: r.get(3)?,
        body: r.get(4)?,
        content_hash: r.get(5)?,
        present: r.get(6)?,
        indexed_at: parse(r.get(7)?)?,
    })
}
fn snapshot_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<DocumentSnapshot> {
    Ok(DocumentSnapshot {
        id: r.get(0)?,
        project_id: r.get(1)?,
        relative_path: r.get(2)?,
        content_hash: r.get(3)?,
        content: r.get(4)?,
        captured_at: parse(r.get(5)?)?,
        cause: r.get(6)?,
        sequence: r.get(7)?,
    })
}
