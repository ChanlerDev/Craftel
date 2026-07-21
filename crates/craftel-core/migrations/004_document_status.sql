CREATE TABLE IF NOT EXISTS document_project_status (
 project_id TEXT PRIMARY KEY,
 state TEXT NOT NULL CHECK(state IN ('healthy','error')),
 error TEXT,
 updated_at TEXT NOT NULL,
 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);
