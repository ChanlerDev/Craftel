CREATE TABLE IF NOT EXISTS phase_sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    phase TEXT NOT NULL CHECK (phase IN ('defining','implementation','reviewing')),
    harness TEXT NOT NULL,
    external_session_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id, task_id) REFERENCES tasks(project_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS phase_sessions_task ON phase_sessions(project_id, task_id, phase, created_at DESC);

CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES phase_sessions(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    state TEXT NOT NULL CHECK (state IN ('queued','running','succeeded','failed','stopped','interrupted')),
    prompt TEXT NOT NULL,
    harness TEXT NOT NULL,
    harness_version TEXT,
    model TEXT,
    work_dir TEXT NOT NULL,
    request_id TEXT,
    pid INTEGER,
    ownership_token TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT,
    exit_code INTEGER,
    stderr TEXT NOT NULL DEFAULT '',
    final_result TEXT,
    stop_requested_at TEXT,
    error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(session_id, sequence),
    FOREIGN KEY (project_id, task_id) REFERENCES tasks(project_id, id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS runs_one_active_task ON runs(project_id, task_id) WHERE state IN ('queued','running');

CREATE TABLE IF NOT EXISTS run_events (
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    kind TEXT NOT NULL CHECK (kind IN ('user','assistant','tool_start','tool_complete','result','system','unknown')),
    event_at TEXT NOT NULL,
    display_text TEXT,
    tool_name TEXT,
    tool_call_id TEXT,
    raw_json TEXT NOT NULL,
    PRIMARY KEY (run_id, sequence)
);

CREATE TRIGGER IF NOT EXISTS runs_terminal_immutable
BEFORE UPDATE ON runs
WHEN OLD.state IN ('succeeded','failed','stopped','interrupted')
BEGIN SELECT RAISE(ABORT, 'terminal run is immutable'); END;
