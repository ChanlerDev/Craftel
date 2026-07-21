import { useEffect, useState } from "react";
import type { CraftelApi } from "../api/craftel";
import { errorMessage } from "../api/craftel";
import type { Project } from "../api/types";

export function ProjectSwitcher({ api, selected, onSelect }: { api: CraftelApi; selected: Project | null; onSelect(p: Project | null): void }) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [error, setError] = useState("");
  const load = async () => { try { const values = await api.listProjects(); setProjects(values); if (!selected && values[0]) onSelect(values[0]); } catch (e) { setError(errorMessage(e)); } };
  useEffect(() => { void load(); }, []); // API is an injected application dependency.
  const add = async () => {
    try {
      const path = await api.selectProjectDirectory(); if (!path) return;
      const fallback = path.split(/[\\/]/).filter(Boolean).at(-1) ?? "Project";
      const name = window.prompt("Project name", fallback)?.trim(); if (!name) return;
      const project = await api.registerProject(name, path); await api.openProject(project.id); await load(); onSelect(project);
    } catch (e) { setError(errorMessage(e)); }
  };
  const choose = async (id: string) => { try { onSelect(await api.openProject(id)); } catch (e) { setError(errorMessage(e)); } };
  const remove = async (project: Project) => { try { await api.removeProject(project.id); if (selected?.id === project.id) onSelect(null); await load(); } catch (e) { setError(errorMessage(e)); } };
  return <aside className="projects" aria-label="Projects">
    <div className="brand">CRAFTEL</div><button className="primary" onClick={add}>Open Project</button>
    {projects.map(p => <div className={`project ${selected?.id === p.id ? "active" : ""}`} key={p.id}>
      <button onClick={() => void choose(p.id)}>{p.name}</button>
      {!p.available && <div className="missing"><strong>Directory missing</strong><span>Locate this folder later, or remove its registration.</span><button onClick={() => void remove(p)}>Remove Registration — files are untouched</button></div>}
    </div>)}
    {error && <p role="alert">{error}</p>}
  </aside>;
}
