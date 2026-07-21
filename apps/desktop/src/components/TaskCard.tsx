import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import type { Task } from "../api/types";

export function TaskCard({ task, onEdit }: { task: Task; onEdit(task: Task): void }) {
  const d = useDraggable({ id: task.id, data: { task } });
  return <article ref={d.setNodeRef} style={{ transform: CSS.Translate.toString(d.transform) }} className={`card ${d.isDragging ? "dragging" : ""}`}>
    <button className="drag" aria-label={`Move ${task.id}`} {...d.listeners} {...d.attributes}>⠿</button><span className="task-id">{task.id}</span><h3>{task.title}</h3><p>{task.content}</p><button onClick={() => onEdit(task)}>Edit</button>
  </article>;
}
