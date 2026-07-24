import { useEffect, useState } from "react";
import type { CraftelApi } from "./api/craftel";
import type { Project, Task } from "./api/types";
import { Board } from "./components/Board";
import { CreateTaskDialog } from "./components/CreateTaskDialog";
import { ProjectSwitcher } from "./components/ProjectSwitcher";
import { TaskWorkspace } from "./components/TaskWorkspace";
import { UpdateControl } from "./components/UpdateControl";
import { nativeUpdateService } from "./api/updater";

export function App({ api }: { api: CraftelApi }) {
  const [project, selectProject] = useState<Project | null>(null); const [task,setTask]=useState<Task|null>(null); const [creating, setCreating] = useState(false); const [refresh, setRefresh] = useState(0); const [dirty,setDirty]=useState(false);
  useEffect(()=>{const warn=(event:BeforeUnloadEvent)=>{if(dirty)event.preventDefault()};window.addEventListener("beforeunload",warn);return()=>window.removeEventListener("beforeunload",warn)},[dirty]);
  const allowLeave=()=>!dirty||window.confirm("Discard unsaved document drafts?");
  const setProject=(p:Project|null)=>{setDirty(false);setTask(null);selectProject(p);};
  const leaveTask=()=>{if(!allowLeave())return;setDirty(false);setTask(null)};
  return <div className="shell"><ProjectSwitcher api={api} selected={project} canSelect={allowLeave} onSelect={setProject} /><main><UpdateControl service={nativeUpdateService} />
    {!project ? <div className="empty"><p className="eyebrow">LOCAL-FIRST WORKSPACE</p><h1>Turn a working directory into a CRAFTEL project.</h1><p>Open Project to register a local directory, then organize durable tasks through a five-stage workflow.</p></div> : !project.available ? <div className="empty"><h1>{project.name} is unavailable</h1><p>Its working directory could not be found. Locate it later or remove the registration; files are never deleted.</p></div> : task?<TaskWorkspace api={api} project={project} task={task} onBack={leaveTask} onDirtyChange={setDirty}/>:<><header className="top"><div><p className="eyebrow">PROJECT</p><h1>{project.name}</h1><small>{project.work_dir}</small></div><button className="primary" onClick={() => setCreating(true)}>New task</button></header><Board api={api} projectId={project.id} refreshToken={refresh} onOpen={setTask}/>{creating && <CreateTaskDialog api={api} projectId={project.id} onClose={() => setCreating(false)} onSaved={() => setRefresh(v => v + 1)} />}</>}
  </main></div>;
}
