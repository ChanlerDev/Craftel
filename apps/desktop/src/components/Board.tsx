import { DndContext, KeyboardSensor, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { useEffect, useRef, useState } from "react";
import type { CraftelApi } from "../api/craftel";
import { errorMessage } from "../api/craftel";
import { stages, type Run, type Stage, type Task } from "../api/types";
import { BoardColumn } from "./BoardColumn";
import { EditTaskDialog } from "./EditTaskDialog";

const workflow = [
  ["Inbox", "Capture the work"], ["Defining", "Shape the specification"], ["Implementation", "Build the solution"],
  ["Reviewing", "Verify the outcome"], ["Done", "Approved and complete"],
] as const;

export function WorkflowRail() {
  return <section className="workflow" aria-labelledby="workflow-title">
    <div className="workflow-intro"><p className="eyebrow">WORKFLOW</p><h3 id="workflow-title" aria-label="From idea to approved work">Task flow</h3><p aria-hidden="true"><strong>Drag</strong> moves · <strong>Start</strong> runs</p><span className="sr-only">Drag to change stage. Starting an agent is always an explicit action.</span></div>
    <div className="workflow-flow"><div className="workflow-track">{workflow.map(([label], index) => <div className="workflow-step" data-testid="workflow-stage" key={label}><span className="stage-number">{String(index + 1).padStart(2, "0")}</span><strong>{label}</strong>{index < workflow.length - 1 && <span className="workflow-arrow" aria-hidden="true">→</span>}</div>)}</div>
      <div className="return-path"><span>Changes requested</span><span aria-hidden="true">Reviewing → Implementation</span></div></div>
  </section>;
}

export async function moveTaskOptimistically(api: CraftelApi, projectId: string, task: Task, target: Stage, tasks: Task[], update: (tasks: Task[]) => void, showError: (message: string) => void) {
  update(tasks.map(value => value.id === task.id ? { ...value, stage: target } : value)); showError("");
  try { const moved = await api.moveTask(projectId, task.id, target); update(tasks.map(value => value.id === moved.id ? moved : value)); }
  catch (error) { update(tasks); showError(`Could not move ${task.id}: ${errorMessage(error)}`); }
}

export function Board({ api, projectId, refreshToken = 0, onOpen=()=>{} }: { api: CraftelApi; projectId: string; refreshToken?: number; onOpen?:(task:Task)=>void }) {
  const [tasks, setTasks] = useState<Task[]>([]); const [editing, setEditing] = useState<Task | null>(null); const [error, setError] = useState("");
  const [active,setActive]=useState(new Map<string,Run>());const generation=useRef(0);
  const sensors = useSensors(useSensor(PointerSensor), useSensor(KeyboardSensor));
  const load = async () => {const mine=++generation.current;try { const [values,runs]=await Promise.all([api.listTasks(projectId),api.listActiveRuns(projectId)]);if(mine!==generation.current)return;setTasks(values);setActive(new Map(runs.map(r=>[r.task_id,r])));setError(""); } catch (e) {if(mine===generation.current)setError(errorMessage(e)); } };
  useEffect(() => {let dead=false,off:(()=>void)|undefined;void(async()=>{await load();if(dead)return;const unsubscribe=await api.subscribe("run_changed",()=>{if(!dead)void load()});if(dead)unsubscribe();else off=unsubscribe;await load()})();return()=>{dead=true;++generation.current;off?.()}; }, [projectId, refreshToken]);
  const end = async ({ active, over }: DragEndEvent) => {
    if (!over || !stages.includes(over.id as Stage)) return;
    const target = over.id as Stage; const before = tasks; const task = tasks.find(t => t.id === active.id); if (!task || task.stage === target) return;
    await moveTaskOptimistically(api, projectId, task, target, before, setTasks, setError);
  };
  return <><WorkflowRail/><div className="scroll-hint"><span>Workflow board</span><span>Scroll horizontally to see every stage <b aria-hidden="true">→</b></span></div><DndContext sensors={sensors} onDragEnd={end}><div className="board">{stages.map(stage => <BoardColumn key={stage} stage={stage} tasks={tasks.filter(t => t.stage === stage)} active={new Map([...active].map(([id,run])=>[id,run.state]))} onEdit={setEditing} onOpen={onOpen} />)}</div></DndContext>{error && <p className="toast" role="alert">{error} <button onClick={()=>void load()}>Retry</button></p>}{editing && <EditTaskDialog api={api} task={editing} onClose={() => setEditing(null)} onSaved={saved => setTasks(v => v.map(t => t.id === saved.id ? saved : t))} />}</>;
}
