ALTER TABLE document_snapshots ADD COLUMN sequence INTEGER;
UPDATE document_snapshots SET sequence = rowid WHERE sequence IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS document_snapshot_sequence
  ON document_snapshots(project_id, relative_path, sequence);
CREATE TABLE IF NOT EXISTS document_mutation_leases (
  project_id TEXT NOT NULL,
  relative_path TEXT NOT NULL,
  owner TEXT NOT NULL,
  acquired_at TEXT NOT NULL,
  expires_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z',
  PRIMARY KEY(project_id, relative_path),
  FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);
