import { DndContext, KeyboardSensor, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { useEffect, useState } from "react";
import type { CraftelApi } from "../api/craftel";
import { errorMessage } from "../api/craftel";
import { stages, type Stage, type Task } from "../api/types";
import { BoardColumn } from "./BoardColumn";
import { EditTaskDialog } from "./EditTaskDialog";

export async function moveTaskOptimistically(api: CraftelApi, projectId: string, task: Task, target: Stage, tasks: Task[], update: (tasks: Task[]) => void, showError: (message: string) => void) {
  update(tasks.map(value => value.id === task.id ? { ...value, stage: target } : value)); showError("");
  try { const moved = await api.moveTask(projectId, task.id, target); update(tasks.map(value => value.id === moved.id ? moved : value)); }
  catch (error) { update(tasks); showError(`Could not move ${task.id}: ${errorMessage(error)}`); }
}

export function Board({ api, projectId, refreshToken = 0 }: { api: CraftelApi; projectId: string; refreshToken?: number }) {
  const [tasks, setTasks] = useState<Task[]>([]); const [editing, setEditing] = useState<Task | null>(null); const [error, setError] = useState("");
  const sensors = useSensors(useSensor(PointerSensor), useSensor(KeyboardSensor));
  const load = async () => { try { setTasks(await api.listTasks(projectId)); setError(""); } catch (e) { setError(errorMessage(e)); } };
  useEffect(() => { void load(); }, [projectId, refreshToken]);
  const end = async ({ active, over }: DragEndEvent) => {
    if (!over || !stages.includes(over.id as Stage)) return;
    const target = over.id as Stage; const before = tasks; const task = tasks.find(t => t.id === active.id); if (!task || task.stage === target) return;
    await moveTaskOptimistically(api, projectId, task, target, before, setTasks, setError);
  };
  return <><DndContext sensors={sensors} onDragEnd={end}><div className="board">{stages.map(stage => <BoardColumn key={stage} stage={stage} tasks={tasks.filter(t => t.stage === stage)} onEdit={setEditing} />)}</div></DndContext>{error && <p className="toast" role="alert">{error}</p>}{editing && <EditTaskDialog api={api} task={editing} onClose={() => setEditing(null)} onSaved={saved => setTasks(v => v.map(t => t.id === saved.id ? saved : t))} />}</>;
}
