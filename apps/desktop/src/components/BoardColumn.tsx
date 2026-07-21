import { useDroppable } from "@dnd-kit/core";
import type { Stage, Task } from "../api/types";
import { TaskCard } from "./TaskCard";
const labels: Record<Stage, string> = { inbox: "Inbox", defining: "Defining", implementation: "Implementation", reviewing: "Reviewing", done: "Done" };
export function BoardColumn({ stage, tasks, active, onEdit, onOpen }: { stage: Stage; tasks: Task[]; active:Map<string,string>;onEdit(t: Task): void; onOpen(t:Task):void }) {
  const drop = useDroppable({ id: stage });
  const index=(["inbox","defining","implementation","reviewing","done"] as Stage[]).indexOf(stage)+1;
  return <section ref={drop.setNodeRef} className={`column column-${stage} ${drop.isOver ? "over" : ""}`} aria-labelledby={`column-${stage}`}><header><span className="column-index">{String(index).padStart(2,"0")}</span><h2 id={`column-${stage}`}>{labels[stage]}</h2><span className="column-count" aria-label={`${tasks.length} tasks`}>{tasks.length}</span></header><div className="card-list">{tasks.map(t => <TaskCard key={t.id} task={t} runState={active.get(t.id)} onEdit={onEdit} onOpen={onOpen} />)}{tasks.length===0&&<p className="column-empty">Drop a task here</p>}</div></section>;
}
