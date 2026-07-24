import { useEffect, useRef, useState } from "react";
import type { CraftelApi } from "../api/craftel";
import { errorMessage } from "../api/craftel";
import type { Project } from "../api/types";
import { DirectoryPickerDialog } from "./DirectoryPickerDialog";

export function ProjectSwitcher({ api, selected, canSelect = () => true, onSelect }: { api: CraftelApi; selected: Project | null; canSelect?(): boolean; onSelect(p: Project | null): void }) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [error, setError] = useState("");
  const [adding, setAdding] = useState(false);
  const mounted = useRef(true);
  const loadRequest = useRef(0);
  const selectedRef = useRef(selected);
  const openButton = useRef<HTMLButtonElement>(null);
  selectedRef.current = selected;
  const load = async (selectFirst = true) => { const current = ++loadRequest.current; try { const values = await api.listProjects(); if (!mounted.current || current !== loadRequest.current) return; setProjects(values); if (selectFirst && !selectedRef.current && values[0]) onSelect(values[0]); } catch (e) { if (mounted.current && current === loadRequest.current) setError(errorMessage(e)); } };
  useEffect(() => { mounted.current = true; void load(); return () => { mounted.current = false; loadRequest.current += 1; }; }, []); // API is an injected application dependency.
  const add = async (path: string) => {
    if (!canSelect()) return false;
    const name = path.split(/[\\/]/).filter(Boolean).at(-1) ?? "Project";
    const project = await api.registerProject(name, path);
    const opened = await api.openProject(project.id);
    await load(false);
    if (!mounted.current) return false;
    onSelect(opened);
    setAdding(false);
    return true;
  };
  const choose = async (id: string) => { if (!canSelect()) return; try { onSelect(await api.openProject(id)); } catch (e) { setError(errorMessage(e)); } };
  const remove = async (project: Project) => { if (selected?.id === project.id && !canSelect()) return; try { await api.removeProject(project.id); if (selected?.id === project.id) onSelect(null); await load(); } catch (e) { setError(errorMessage(e)); } };
  return <aside className="projects" aria-label="Projects">
    <div className="brand">CRAFTEL</div><button ref={openButton} className="primary" onClick={() => { setError(""); setAdding(true); }}>Open Project</button>
    {projects.map(p => <div className={`project ${selected?.id === p.id ? "active" : ""}`} key={p.id}>
      <button onClick={() => void choose(p.id)}>{p.name}</button>
      {!p.available && <div className="missing"><strong>Directory missing</strong><span>Locate this folder later, or remove its registration.</span><button onClick={() => void remove(p)}>Remove Registration — files are untouched</button></div>}
    </div>)}
    {error && <p role="alert">{error}</p>}
    {adding && <DirectoryPickerDialog api={api} opener={openButton} onCancel={() => setAdding(false)} onAdd={add} />}
  </aside>;
}
