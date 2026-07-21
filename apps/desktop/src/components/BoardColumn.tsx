import { useDroppable } from "@dnd-kit/core";
import type { Stage, Task } from "../api/types";
import { TaskCard } from "./TaskCard";
const labels: Record<Stage, string> = { inbox: "Inbox", defining: "Defining", implementation: "Implementation", reviewing: "Reviewing", done: "Done" };
export function BoardColumn({ stage, tasks, onEdit }: { stage: Stage; tasks: Task[]; onEdit(t: Task): void }) {
  const drop = useDroppable({ id: stage });
  return <section ref={drop.setNodeRef} className={`column ${drop.isOver ? "over" : ""}`} aria-labelledby={`column-${stage}`}><header><h2 id={`column-${stage}`}>{labels[stage]}</h2><span>{tasks.length}</span></header><div>{tasks.map(t => <TaskCard key={t.id} task={t} onEdit={onEdit} />)}</div></section>;
}
