import { useState } from "react";
import type { CraftelApi } from "../api/craftel";
import { errorMessage } from "../api/craftel";

export function CreateTaskDialog({ api, projectId, onClose, onSaved }: { api: CraftelApi; projectId: string; onClose(): void; onSaved(): void }) {
  const [title, setTitle] = useState(""); const [content, setContent] = useState(""); const [error, setError] = useState("");
  const save = async (e: React.FormEvent) => { e.preventDefault(); if (!title.trim() || !content.trim()) { setError("Title and content are required."); return; } try { await api.createTask(projectId, title.trim(), content.trim()); onSaved(); onClose(); } catch (x) { setError(errorMessage(x)); } };
  return <div className="scrim"><form role="dialog" aria-modal="true" aria-labelledby="create-title" onSubmit={save}><h2 id="create-title">Create task</h2><label>Title<input autoFocus value={title} onChange={e => setTitle(e.target.value)} /></label><label>Content<textarea value={content} onChange={e => setContent(e.target.value)} /></label>{error && <p role="alert">{error}</p>}<div className="actions"><button type="button" onClick={onClose}>Cancel</button><button className="primary">Create task</button></div></form></div>;
}
