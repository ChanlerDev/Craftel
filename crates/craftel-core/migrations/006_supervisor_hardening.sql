CREATE UNIQUE INDEX IF NOT EXISTS phase_sessions_identity
ON phase_sessions(id, project_id, task_id);

CREATE TRIGGER IF NOT EXISTS runs_session_task_insert
BEFORE INSERT ON runs
WHEN NOT EXISTS (
    SELECT 1 FROM phase_sessions s
    WHERE s.id = NEW.session_id
      AND s.project_id = NEW.project_id
      AND s.task_id = NEW.task_id
)
BEGIN SELECT RAISE(ABORT, 'run session/task mismatch'); END;

CREATE TRIGGER IF NOT EXISTS runs_session_task_update
BEFORE UPDATE OF session_id, project_id, task_id ON runs
WHEN NOT EXISTS (
    SELECT 1 FROM phase_sessions s
    WHERE s.id = NEW.session_id
      AND s.project_id = NEW.project_id
      AND s.task_id = NEW.task_id
)
BEGIN SELECT RAISE(ABORT, 'run session/task mismatch'); END;

CREATE TABLE IF NOT EXISTS run_supervisor_lease (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    instance_id TEXT NOT NULL,
    acquired_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);
