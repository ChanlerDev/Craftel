use crate::automation::AutomationPrompt;
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("run not found")]
    NotFound,
    #[error("an active run already exists for this task")]
    ActiveRun,
    #[error("external session ID conflicts with the session already recorded")]
    ExternalSessionConflict,
    #[error("invalid run operation: {0}")]
    Invalid(String),
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

macro_rules! string_enum { ($n:ident { $($v:ident => $s:literal),+ $(,)? }) => {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all="snake_case")]
    pub enum $n { $($v),+ }
    impl $n { pub fn as_str(self)->&'static str { match self { $(Self::$v=>$s),+ } } fn parse(s:&str)->Result<Self,RunError>{match s {$($s=>Ok(Self::$v),)+ _=>Err(RunError::Invalid(format!("unknown {} {s}",stringify!($n))))}} }
}; }
string_enum!(Phase { Defining=>"defining", Implementation=>"implementation", Reviewing=>"reviewing" });
string_enum!(RunState { Queued=>"queued", Running=>"running", Succeeded=>"succeeded", Failed=>"failed", Stopped=>"stopped", Interrupted=>"interrupted" });
string_enum!(EventKind { User=>"user", Assistant=>"assistant", ToolStart=>"tool_start", ToolComplete=>"tool_complete", Result=>"result", System=>"system", Unknown=>"unknown" });

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhaseSession {
    pub id: String,
    pub project_id: String,
    pub task_id: String,
    pub phase: Phase,
    pub harness: String,
    pub external_session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub session_id: String,
    pub project_id: String,
    pub task_id: String,
    pub sequence: i64,
    pub state: RunState,
    pub prompt: String,
    pub harness: String,
    pub harness_version: Option<String>,
    pub model: Option<String>,
    pub work_dir: PathBuf,
    pub request_id: Option<String>,
    #[serde(skip_serializing)]
    pub pid: Option<u32>,
    #[serde(skip_serializing)]
    pub ownership_token: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub stderr: String,
    pub final_result: Option<String>,
    pub stop_requested_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub stage_at_start: Option<Phase>,
    pub workflow_event_id_before: Option<i64>,
    pub prompt_kind: Option<Phase>,
    pub prompt_version: Option<i64>,
    pub observed_transition_event_id: Option<i64>,
    pub missing_transition: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunEvent {
    pub run_id: String,
    pub sequence: i64,
    pub kind: EventKind,
    pub event_at: DateTime<Utc>,
    pub display_text: Option<String>,
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
    pub raw_json: String,
}
pub struct RunRepository {
    connection: Connection,
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
fn dt(s: String) -> Result<DateTime<Utc>, rusqlite::Error> {
    DateTime::parse_from_rfc3339(&s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

impl RunRepository {
    pub fn open(path: &Path) -> Result<Self, RunError> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?
        }
        let c = Connection::open(path)?;
        c.busy_timeout(std::time::Duration::from_secs(5))?;
        c.pragma_update(None, "foreign_keys", true)?;
        c.execute_batch(include_str!("../migrations/005_harness.sql"))?;
        c.execute_batch(include_str!("../migrations/006_supervisor_hardening.sql"))?;
        if !columns_for(&c, "runs")?
            .iter()
            .any(|x| x == "stage_at_start")
        {
            c.execute_batch(include_str!("../migrations/007_automation.sql"))?;
        }
        let columns: Vec<String> = {
            let mut statement = c.prepare("PRAGMA table_info(run_supervisor_lease)")?;
            statement
                .query_map([], |row| row.get(1))?
                .collect::<Result<_, _>>()?
        };
        if !columns.iter().any(|column| column == "expires_at") {
            c.execute(
                "ALTER TABLE run_supervisor_lease ADD COLUMN expires_at TEXT",
                [],
            )?;
        }
        Ok(Self { connection: c })
    }

    pub fn acquire_supervisor_lease(
        &mut self,
        owner: &str,
        ttl: std::time::Duration,
    ) -> Result<bool, RunError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let n = Utc::now();
        let expires = (n + chrono::Duration::from_std(ttl)
            .unwrap_or(chrono::Duration::seconds(15)))
        .to_rfc3339_opts(SecondsFormat::Millis, true);
        let now_text = n.to_rfc3339_opts(SecondsFormat::Millis, true);
        tx.execute("DELETE FROM run_supervisor_lease WHERE singleton=1 AND (expires_at IS NULL OR expires_at<=?1)", [&now_text])?;
        let acquired = tx.execute("INSERT OR IGNORE INTO run_supervisor_lease(singleton,instance_id,acquired_at,expires_at) VALUES(1,?1,?2,?3)", params![owner,now_text,expires])? == 1;
        tx.commit()?;
        Ok(acquired)
    }
    pub fn heartbeat_supervisor_lease(
        &mut self,
        owner: &str,
        ttl: std::time::Duration,
    ) -> Result<bool, RunError> {
        let expires = (Utc::now()
            + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::seconds(15)))
        .to_rfc3339_opts(SecondsFormat::Millis, true);
        Ok(self.connection.execute(
            "UPDATE run_supervisor_lease SET expires_at=?1 WHERE singleton=1 AND instance_id=?2",
            params![expires, owner],
        )? == 1)
    }
    pub fn release_supervisor_lease(&mut self, owner: &str) -> Result<(), RunError> {
        self.connection.execute(
            "DELETE FROM run_supervisor_lease WHERE singleton=1 AND instance_id=?1",
            [owner],
        )?;
        Ok(())
    }
    pub fn create_session(
        &mut self,
        p: &str,
        t: &str,
        phase: Phase,
        harness: &str,
    ) -> Result<PhaseSession, RunError> {
        let id = Uuid::new_v4().to_string();
        let n = now();
        self.connection.execute(
            "INSERT INTO phase_sessions VALUES(?1,?2,?3,?4,?5,NULL,?6,?6)",
            params![id, p, t, phase.as_str(), harness, n],
        )?;
        self.get_session(&id)
    }
    pub fn latest_session(
        &self,
        p: &str,
        t: &str,
        phase: Phase,
    ) -> Result<Option<PhaseSession>, RunError> {
        self.connection.query_row("SELECT id,project_id,task_id,phase,harness,external_session_id,created_at,updated_at FROM phase_sessions WHERE project_id=?1 AND task_id=?2 AND phase=?3 ORDER BY created_at DESC,id DESC LIMIT 1",params![p,t,phase.as_str()],session_row).optional().map_err(Into::into)
    }
    pub fn get_session(&self, id: &str) -> Result<PhaseSession, RunError> {
        self.connection.query_row("SELECT id,project_id,task_id,phase,harness,external_session_id,created_at,updated_at FROM phase_sessions WHERE id=?1",[id],session_row).optional()?.ok_or(RunError::NotFound)
    }
    pub fn list_sessions(&self, p: &str, t: &str) -> Result<Vec<PhaseSession>, RunError> {
        let mut s=self.connection.prepare("SELECT id,project_id,task_id,phase,harness,external_session_id,created_at,updated_at FROM phase_sessions WHERE project_id=?1 AND task_id=?2 ORDER BY created_at,id")?;
        Ok(s.query_map(params![p, t], session_row)?
            .collect::<Result<_, _>>()?)
    }
    pub fn set_external_session(&mut self, id: &str, external: &str) -> Result<(), RunError> {
        let n = now();
        let changed = self.connection.execute("UPDATE phase_sessions SET external_session_id=COALESCE(external_session_id,?1),updated_at=?2 WHERE id=?3 AND (external_session_id IS NULL OR external_session_id=?1)",params![external,n,id])?;
        if changed != 1 {
            return Err(RunError::ExternalSessionConflict);
        }
        Ok(())
    }
    /// Select/create the phase session and queue its run under one write lock.
    /// This prevents empty review sessions and races between separate clients.
    pub fn reserve_phase_run(
        &mut self,
        p: &str,
        t: &str,
        phase: Phase,
        prompt: &AutomationPrompt,
    ) -> Result<(PhaseSession, Run), RunError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM runs WHERE project_id=?1 AND task_id=?2 AND state IN ('queued','running'))", params![p,t], |r| r.get(0))?;
        if active {
            return Err(RunError::ActiveRun);
        }
        let (work, stage): (String,String) = tx.query_row("SELECT p.work_dir,t.stage FROM projects p JOIN tasks t ON t.project_id=p.id WHERE p.id=?1 AND t.id=?2", params![p,t], |r| Ok((r.get(0)?,r.get(1)?))).optional()?.ok_or(RunError::NotFound)?;
        if stage != phase.as_str() {
            return Err(RunError::Invalid(format!(
                "task stage is {stage}, not {}",
                phase.as_str()
            )));
        }
        let baseline: i64 = tx.query_row(
            "SELECT COALESCE(MAX(event_id),0) FROM workflow_events",
            [],
            |r| r.get(0),
        )?;
        let existing: Option<String> = if phase == Phase::Reviewing {
            None
        } else {
            tx.query_row("SELECT id FROM phase_sessions WHERE project_id=?1 AND task_id=?2 AND phase=?3 ORDER BY created_at DESC,id DESC LIMIT 1", params![p,t,phase.as_str()], |r| r.get(0)).optional()?
        };
        let sid = existing.unwrap_or_else(|| Uuid::new_v4().to_string());
        let n = now();
        tx.execute("INSERT OR IGNORE INTO phase_sessions(id,project_id,task_id,phase,harness,external_session_id,created_at,updated_at) VALUES(?1,?2,?3,?4,'cursor',NULL,?5,?5)",params![sid,p,t,phase.as_str(),n])?;
        let seq: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sequence),0)+1 FROM runs WHERE session_id=?1",
            [&sid],
            |r| r.get(0),
        )?;
        let id = Uuid::new_v4().to_string();
        let token = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO runs(id,session_id,project_id,task_id,sequence,state,prompt,harness,model,work_dir,ownership_token,created_at,updated_at,stage_at_start,workflow_event_id_before,prompt_kind,prompt_version) VALUES(?1,?2,?3,?4,?5,'queued',?6,'cursor',NULL,?7,?8,?9,?9,?10,?11,?12,?13)",params![id,sid,p,t,seq,prompt.text,work,token,n,phase.as_str(),baseline,prompt.kind.as_str(),prompt.version]).map_err(|e| if matches!(&e,rusqlite::Error::SqliteFailure(x,_) if x.extended_code==2067) { RunError::ActiveRun } else { e.into() })?;
        tx.commit()?;
        Ok((self.get_session(&sid)?, self.get_run(&id)?))
    }

    pub fn append_event_with_metadata(
        &mut self,
        session: &str,
        id: &str,
        event: &crate::harness::ParsedEvent,
    ) -> Result<RunEvent, RunError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let n = now();
        if let Some(external) = event.external_session_id.as_deref()
            && tx.execute("UPDATE phase_sessions SET external_session_id=COALESCE(external_session_id,?1),updated_at=?2 WHERE id=?3 AND (external_session_id IS NULL OR external_session_id=?1)",params![external,n,session])? != 1
        {
            return Err(RunError::ExternalSessionConflict);
        }
        if let Some(request) = event.request_id.as_deref() {
            tx.execute("UPDATE runs SET request_id=?1,updated_at=?2 WHERE id=?3 AND state IN ('queued','running')",params![request,n,id])?;
        }
        if let Some(model) = event.model.as_deref() {
            tx.execute("UPDATE runs SET model=?1,updated_at=?2 WHERE id=?3 AND state IN ('queued','running')",params![model,n,id])?;
        }
        let seq: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sequence),0)+1 FROM run_events WHERE run_id=?1",
            [id],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO run_events VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                id,
                seq,
                event.kind.as_str(),
                n,
                event.display_text,
                event.tool_name,
                event.tool_call_id,
                event.raw_json
            ],
        )?;
        tx.commit()?;
        Ok(self.list_events(id, seq - 1, 1)?.remove(0))
    }
    pub fn reserve_run(
        &mut self,
        session: &PhaseSession,
        prompt: &str,
        work_dir: &Path,
    ) -> Result<Run, RunError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stage: String = tx
            .query_row(
                "SELECT stage FROM tasks WHERE project_id=?1 AND id=?2",
                params![session.project_id, session.task_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(RunError::NotFound)?;
        if stage != session.phase.as_str() {
            return Err(RunError::Invalid(format!(
                "task stage is {stage}, not {}",
                session.phase.as_str()
            )));
        }
        let seq: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sequence),0)+1 FROM runs WHERE session_id=?1",
            [&session.id],
            |r| r.get(0),
        )?;
        let id = Uuid::new_v4().to_string();
        let token = Uuid::new_v4().to_string();
        let n = now();
        let result=tx.execute("INSERT INTO runs(id,session_id,project_id,task_id,sequence,state,prompt,harness,model,work_dir,ownership_token,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,'queued',?6,?7,NULL,?8,?9,?10,?10)",params![id,session.id,session.project_id,session.task_id,seq,prompt,session.harness,work_dir.to_string_lossy(),token,n]);
        if let Err(e) = result {
            if matches!(&e,rusqlite::Error::SqliteFailure(x,_) if x.extended_code==2067) {
                return Err(RunError::ActiveRun);
            }
            return Err(e.into());
        }
        tx.commit()?;
        self.get_run(&id)
    }
    pub fn mark_running(
        &mut self,
        id: &str,
        pid: u32,
        version: Option<&str>,
    ) -> Result<Run, RunError> {
        let n = now();
        if self.connection.execute("UPDATE runs SET state='running',pid=?1,harness_version=?2,started_at=?3,updated_at=?3 WHERE id=?4 AND state='queued'",params![pid,version,n,id])?!=1{return Err(RunError::Invalid("run is not queued".into()))}
        self.get_run(id)
    }

    /// Revalidate an automation reservation and spawn while holding the short write reservation.
    /// The child blocks in its CLI transition until this transaction commits.
    pub fn preflight_and_spawn<F>(
        &mut self,
        id: &str,
        version: &str,
        spawn: F,
    ) -> Result<(Run, std::process::Child), RunError>
    where
        F: FnOnce() -> std::io::Result<std::process::Child>,
    {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (project, task, stage, baseline): (String, String, Option<String>, Option<i64>) = tx.query_row(
            "SELECT project_id,task_id,stage_at_start,workflow_event_id_before FROM runs WHERE id=?1 AND state='queued'",
            [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?
            .ok_or_else(|| RunError::Invalid("run is not queued".into()))?;
        let mismatch = if let (Some(stage), Some(baseline)) = (stage.as_deref(), baseline) {
            let current: String = tx.query_row(
                "SELECT stage FROM tasks WHERE project_id=?1 AND id=?2",
                params![project, task],
                |r| r.get(0),
            )?;
            let changed: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM workflow_events WHERE project_id=?1 AND task_id=?2 AND event_id>?3)", params![project,task,baseline], |r| r.get(0))?;
            current != stage || changed
        } else {
            false
        };
        if mismatch {
            let n = now();
            tx.execute("UPDATE runs SET state='failed',error='task stage changed before launch',finished_at=?1,updated_at=?1,missing_transition=0 WHERE id=?2 AND state='queued'", params![n,id])?;
            tx.commit()?;
            return Err(RunError::Invalid("task stage changed before launch".into()));
        }
        let child = spawn()?;
        let n = now();
        tx.execute("UPDATE runs SET state='running',pid=?1,harness_version=?2,started_at=?3,updated_at=?3 WHERE id=?4 AND state='queued'", params![child.id(),version,n,id])?;
        tx.commit()?;
        Ok((self.get_run(id)?, child))
    }
    pub fn mark_stop_requested(&mut self, id: &str) -> Result<Run, RunError> {
        let n = now();
        self.connection.execute("UPDATE runs SET stop_requested_at=COALESCE(stop_requested_at,?1),updated_at=?1 WHERE id=?2 AND state IN ('queued','running')",params![n,id])?;
        self.get_run(id)
    }
    pub fn finish(
        &mut self,
        id: &str,
        state: RunState,
        code: Option<i32>,
        stderr: &str,
        result: Option<&str>,
        error: Option<&str>,
    ) -> Result<Run, RunError> {
        if matches!(state, RunState::Queued | RunState::Running) {
            return Err(RunError::Invalid("finish requires terminal state".into()));
        }
        let n = now();
        if self.connection.execute("UPDATE runs SET state=?1,exit_code=?2,stderr=?3,final_result=?4,error=?5,finished_at=?6,updated_at=?6 WHERE id=?7 AND state IN ('queued','running')",params![state.as_str(),code,stderr,result,error,n,id])?!=1{return Err(RunError::Invalid("terminal run is immutable".into()))}
        self.get_run(id)
    }
    pub fn finish_with_transition(
        &mut self,
        id: &str,
        state: RunState,
        code: Option<i32>,
        stderr: &str,
        result: Option<&str>,
        error: Option<&str>,
    ) -> Result<Run, RunError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (project, task, stage, baseline): (String, String, Option<String>, Option<i64>) = tx.query_row(
            "SELECT project_id,task_id,stage_at_start,workflow_event_id_before FROM runs WHERE id=?1 AND state IN ('queued','running')",
            [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))
            .optional()?.ok_or_else(|| RunError::Invalid("terminal run is immutable".into()))?;
        let observed: Option<i64> = match (stage.as_deref(), baseline) {
            (Some(stage), Some(baseline)) => tx.query_row(
                "SELECT event_id FROM workflow_events WHERE project_id=?1 AND task_id=?2 AND event_id>?3 AND from_stage=?4 AND action IN ('\"pass\"','\"fail\"') ORDER BY event_id LIMIT 1",
                params![project,task,baseline,stage], |r| r.get(0)).optional()?,
            _ => None,
        };
        let automation = stage.is_some() && baseline.is_some() && tx.query_row("SELECT prompt_kind IS NOT NULL AND prompt_version IS NOT NULL FROM runs WHERE id=?1", [id], |r| r.get::<_, bool>(0))?;
        let n = now();
        tx.execute("UPDATE runs SET state=?1,exit_code=?2,stderr=?3,final_result=?4,error=?5,finished_at=?6,updated_at=?6,observed_transition_event_id=?7,missing_transition=?8 WHERE id=?9 AND state IN ('queued','running')",
            params![state.as_str(),code,stderr,result,error,n,observed,automation && observed.is_none(),id])?;
        tx.commit()?;
        self.get_run(id)
    }
    pub fn append_event(
        &mut self,
        id: &str,
        kind: EventKind,
        display: Option<&str>,
        tool: Option<&str>,
        call: Option<&str>,
        raw: &str,
    ) -> Result<RunEvent, RunError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let seq: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sequence),0)+1 FROM run_events WHERE run_id=?1",
            [id],
            |r| r.get(0),
        )?;
        let n = now();
        tx.execute(
            "INSERT INTO run_events VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, seq, kind.as_str(), n, display, tool, call, raw],
        )?;
        tx.commit()?;
        Ok(self.list_events(id, seq - 1, 1)?.remove(0))
    }
    pub fn list_events(
        &self,
        id: &str,
        after: i64,
        limit: usize,
    ) -> Result<Vec<RunEvent>, RunError> {
        let mut s=self.connection.prepare("SELECT run_id,sequence,kind,event_at,display_text,tool_name,tool_call_id,raw_json FROM run_events WHERE run_id=?1 AND sequence>?2 ORDER BY sequence LIMIT ?3")?;
        Ok(
            s.query_map(params![id, after, limit.min(1000) as i64], event_row)?
                .collect::<Result<_, _>>()?,
        )
    }
    pub fn get_run(&self, id: &str) -> Result<Run, RunError> {
        self.connection
            .query_row(
                &format!("SELECT {RUN_COLUMNS} FROM runs WHERE id=?1"),
                [id],
                run_row,
            )
            .optional()?
            .ok_or(RunError::NotFound)
    }
    pub fn list_runs(&self, session: &str) -> Result<Vec<Run>, RunError> {
        let mut s = self.connection.prepare(&format!(
            "SELECT {RUN_COLUMNS} FROM runs WHERE session_id=?1 ORDER BY sequence"
        ))?;
        Ok(s.query_map([session], run_row)?.collect::<Result<_, _>>()?)
    }

    pub fn list_active_runs(&self, project: &str) -> Result<Vec<Run>, RunError> {
        let mut statement = self.connection.prepare("SELECT id,session_id,project_id,task_id,sequence,state,prompt,harness,harness_version,model,work_dir,request_id,started_at,finished_at,exit_code,stderr,final_result,stop_requested_at,error,stage_at_start,workflow_event_id_before,prompt_kind,prompt_version,observed_transition_event_id,missing_transition,created_at,updated_at FROM runs WHERE project_id=?1 AND state IN ('queued','running') ORDER BY created_at")?;
        Ok(statement
            .query_map([project], run_row)?
            .collect::<Result<_, _>>()?)
    }
    pub fn stale_runs(&self) -> Result<Vec<Run>, RunError> {
        let mut s = self.connection.prepare(&format!(
            "SELECT {RUN_COLUMNS} FROM runs WHERE state IN ('queued','running')"
        ))?;
        Ok(s.query_map([], run_row)?.collect::<Result<_, _>>()?)
    }
    pub fn update_request_id(&mut self, id: &str, request: &str) -> Result<(), RunError> {
        self.connection.execute(
            "UPDATE runs SET request_id=?1 WHERE id=?2 AND state IN ('queued','running')",
            params![request, id],
        )?;
        Ok(())
    }
}
fn session_row(r: &rusqlite::Row) -> rusqlite::Result<PhaseSession> {
    Ok(PhaseSession {
        id: r.get(0)?,
        project_id: r.get(1)?,
        task_id: r.get(2)?,
        phase: Phase::parse(&r.get::<_, String>(3)?).map_err(sql_err)?,
        harness: r.get(4)?,
        external_session_id: r.get(5)?,
        created_at: dt(r.get(6)?)?,
        updated_at: dt(r.get(7)?)?,
    })
}
fn run_row(r: &rusqlite::Row) -> rusqlite::Result<Run> {
    Ok(Run {
        id: r.get(0)?,
        session_id: r.get(1)?,
        project_id: r.get(2)?,
        task_id: r.get(3)?,
        sequence: r.get(4)?,
        state: RunState::parse(&r.get::<_, String>(5)?).map_err(sql_err)?,
        prompt: r.get(6)?,
        harness: r.get(7)?,
        harness_version: r.get(8)?,
        model: r.get(9)?,
        work_dir: PathBuf::from(r.get::<_, String>(10)?),
        request_id: r.get(11)?,
        pid: r.get::<_, Option<u32>>(12)?,
        ownership_token: r.get(13)?,
        started_at: r.get::<_, Option<String>>(14)?.map(dt).transpose()?,
        finished_at: r.get::<_, Option<String>>(15)?.map(dt).transpose()?,
        exit_code: r.get(16)?,
        stderr: r.get(17)?,
        final_result: r.get(18)?,
        stop_requested_at: r.get::<_, Option<String>>(19)?.map(dt).transpose()?,
        error: r.get(20)?,
        created_at: dt(r.get(21)?)?,
        updated_at: dt(r.get(22)?)?,
        stage_at_start: r
            .get::<_, Option<String>>(23)?
            .map(|x| Phase::parse(&x).map_err(sql_err))
            .transpose()?,
        workflow_event_id_before: r.get(24)?,
        prompt_kind: r
            .get::<_, Option<String>>(25)?
            .map(|x| Phase::parse(&x).map_err(sql_err))
            .transpose()?,
        prompt_version: r.get(26)?,
        observed_transition_event_id: r.get(27)?,
        missing_transition: r.get(28)?,
    })
}
const RUN_COLUMNS: &str = "id,session_id,project_id,task_id,sequence,state,prompt,harness,harness_version,model,work_dir,request_id,pid,ownership_token,started_at,finished_at,exit_code,stderr,final_result,stop_requested_at,error,created_at,updated_at,stage_at_start,workflow_event_id_before,prompt_kind,prompt_version,observed_transition_event_id,missing_transition";
fn columns_for(c: &Connection, table: &str) -> Result<Vec<String>, rusqlite::Error> {
    let mut s = c.prepare(&format!("PRAGMA table_info({table})"))?;
    s.query_map([], |r| r.get(1))?.collect()
}
fn event_row(r: &rusqlite::Row) -> rusqlite::Result<RunEvent> {
    Ok(RunEvent {
        run_id: r.get(0)?,
        sequence: r.get(1)?,
        kind: EventKind::parse(&r.get::<_, String>(2)?).map_err(sql_err)?,
        event_at: dt(r.get(3)?)?,
        display_text: r.get(4)?,
        tool_name: r.get(5)?,
        tool_call_id: r.get(6)?,
        raw_json: r.get(7)?,
    })
}
fn sql_err(e: RunError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
}
