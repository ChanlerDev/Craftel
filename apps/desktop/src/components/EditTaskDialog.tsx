import { useState } from "react";
import type { CraftelApi } from "../api/craftel";
import { errorMessage } from "../api/craftel";
import type { Task } from "../api/types";

export function EditTaskDialog({ api, task, onClose, onSaved }: { api: CraftelApi; task: Task; onClose(): void; onSaved(task: Task): void }) {
  const [title, setTitle] = useState(task.title); const [content, setContent] = useState(task.content); const [error, setError] = useState("");
  const save = async (e: React.FormEvent) => { e.preventDefault(); if (!title.trim() || !content.trim()) { setError("Title and content are required."); return; } try { onSaved(await api.updateTask(task.project_id, task.id, title.trim(), content.trim())); onClose(); } catch (x) { setError(errorMessage(x)); } };
  return <div className="scrim"><form role="dialog" aria-modal="true" aria-labelledby="edit-title" onSubmit={save}><h2 id="edit-title">Edit {task.id}</h2><label>Title<input autoFocus value={title} onChange={e => setTitle(e.target.value)} /></label><label>Content<textarea value={content} onChange={e => setContent(e.target.value)} /></label><small>Task directory and SPEC.md are preserved.</small>{error && <p role="alert">{error}</p>}<div className="actions"><button type="button" onClick={onClose}>Cancel</button><button className="primary">Save</button></div></form></div>;
}
