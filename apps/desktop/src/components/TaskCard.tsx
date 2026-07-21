import { useDraggable } from "@dnd-kit/core";
import { useEffect, useRef, useState } from "react";
import { CSS } from "@dnd-kit/utilities";
import type { Task } from "../api/types";

export function TaskCard({ task, runState, onEdit, onOpen }: { task: Task;runState?:string; onEdit(task: Task): void; onOpen(task:Task):void }) {
  const d = useDraggable({ id: task.id, data: { task }, disabled: Boolean(runState) });
  const [menu,setMenu]=useState(false);const menuRef=useRef<HTMLDivElement>(null);
  useEffect(()=>{if(!menu)return;const close=(e:MouseEvent)=>{if(!menuRef.current?.contains(e.target as Node))setMenu(false)};document.addEventListener("mousedown",close);return()=>document.removeEventListener("mousedown",close)},[menu]);
  return <article ref={d.setNodeRef} style={{ transform: CSS.Translate.toString(d.transform) }} className={`card ${d.isDragging ? "dragging" : ""}`}>
    <button className="drag" aria-label={`Move ${task.id}`} disabled={Boolean(runState)} {...d.listeners} {...d.attributes}><span aria-hidden="true">⠿</span></button><span className="task-id">{task.id}</span>{runState&&<span className={`run-indicator run-${runState}`} role="status">● {runState}</span>}<h3>{task.title}</h3><div className="card-actions"><button className="open-card" onClick={() => onOpen(task)}>Open workspace <span aria-hidden="true">→</span></button><div className="card-menu" ref={menuRef}><button aria-haspopup="menu" aria-expanded={menu} aria-label={`Actions for ${task.id}`} onClick={()=>setMenu(v=>!v)}>•••</button>{menu&&<div role="menu"><button role="menuitem" onClick={()=>{setMenu(false);onEdit(task)}}>Edit task details</button></div>}</div></div>
  </article>;
}
