CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    work_dir TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    last_opened_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS task_counters (
    project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    next_number INTEGER NOT NULL CHECK (next_number > 0)
);

CREATE TABLE IF NOT EXISTS tasks (
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    task_number INTEGER NOT NULL,
    id TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    stage TEXT NOT NULL CHECK (stage IN ('inbox','defining','implementation','reviewing','done')),
    relative_dir TEXT NOT NULL,
    review_approved INTEGER NOT NULL DEFAULT 0 CHECK (review_approved IN (0,1)),
    projection_dirty INTEGER NOT NULL DEFAULT 1 CHECK (projection_dirty IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (project_id, id),
    UNIQUE (project_id, task_number)
);

CREATE TABLE IF NOT EXISTS workflow_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    action TEXT NOT NULL,
    from_stage TEXT NOT NULL,
    to_stage TEXT NOT NULL,
    outcome TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (project_id, task_id) REFERENCES tasks(project_id, id) ON DELETE CASCADE
);
