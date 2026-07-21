export const stages = ["inbox", "defining", "implementation", "reviewing", "done"] as const;
export type Stage = (typeof stages)[number];

export interface Project {
  id: string; name: string; work_dir: string; available: boolean;
  created_at: string; last_opened_at: string;
}

export interface Task {
  id: string; project_id: string; title: string; content: string; stage: Stage;
  relative_dir: string; review_approved: boolean; created_at: string; updated_at: string;
}
export interface Document { project_id:string; relative_path:string; task_id:string|null; title:string; body:string; content_hash:string; present:boolean; indexed_at:string; }
export interface DocumentRevision { id:string; project_id:string; relative_path:string; content_hash:string; content:number[]; captured_at:string; cause:"scan"|"watch"|"edit"|"restore"; }
export interface DocumentChanged { project_id:string; path:string; change:string; }
export interface DocumentProjectStatus { project_id:string; state:"healthy"|"error"; error:string|null; updated_at:string; }
export type ExpectedDocumentState = {state:"present";hash:string}|{state:"missing"};
