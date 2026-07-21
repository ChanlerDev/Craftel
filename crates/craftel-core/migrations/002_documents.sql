CREATE TABLE IF NOT EXISTS document_index (
 project_id TEXT NOT NULL, relative_path TEXT NOT NULL, task_id TEXT,
 title TEXT NOT NULL, body TEXT NOT NULL, content_hash TEXT NOT NULL,
 file_mtime TEXT NOT NULL, file_size INTEGER NOT NULL, present INTEGER NOT NULL,
 indexed_at TEXT NOT NULL, PRIMARY KEY(project_id, relative_path),
 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS document_snapshots (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL, relative_path TEXT NOT NULL,
 content_hash TEXT NOT NULL, content BLOB NOT NULL, captured_at TEXT NOT NULL,
 cause TEXT NOT NULL CHECK(cause IN ('scan','watch','edit','restore')),
 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS document_snapshots_identity ON document_snapshots(project_id,relative_path,captured_at DESC,id DESC);
CREATE VIRTUAL TABLE IF NOT EXISTS document_search USING fts5(project_id UNINDEXED, relative_path UNINDEXED, title, body);
CREATE TRIGGER IF NOT EXISTS document_index_ai AFTER INSERT ON document_index BEGIN
 INSERT INTO document_search(rowid,project_id,relative_path,title,body) VALUES(new.rowid,new.project_id,new.relative_path,new.title,new.body);
END;
CREATE TRIGGER IF NOT EXISTS document_index_ad AFTER DELETE ON document_index BEGIN DELETE FROM document_search WHERE rowid=old.rowid; END;
CREATE TRIGGER IF NOT EXISTS document_index_au AFTER UPDATE ON document_index BEGIN
 DELETE FROM document_search WHERE rowid=old.rowid;
 INSERT INTO document_search(rowid,project_id,relative_path,title,body) SELECT new.rowid,new.project_id,new.relative_path,new.title,new.body WHERE new.present=1;
END;
