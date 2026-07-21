ALTER TABLE runs ADD COLUMN stage_at_start TEXT CHECK (stage_at_start IN ('defining','implementation','reviewing'));
ALTER TABLE runs ADD COLUMN workflow_event_id_before INTEGER;
ALTER TABLE runs ADD COLUMN prompt_kind TEXT CHECK (prompt_kind IN ('defining','implementation','reviewing'));
ALTER TABLE runs ADD COLUMN prompt_version INTEGER;
ALTER TABLE runs ADD COLUMN observed_transition_event_id INTEGER REFERENCES workflow_events(event_id);
ALTER TABLE runs ADD COLUMN missing_transition INTEGER NOT NULL DEFAULT 0 CHECK (missing_transition IN (0,1));

CREATE INDEX IF NOT EXISTS workflow_events_attribution
ON workflow_events(project_id, task_id, event_id);
