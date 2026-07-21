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
export interface DocumentRevision { id:string; project_id:string; relative_path:string; content_hash:string; content:number[]; captured_at:string; cause:"scan"|"watch"|"edit"|"restore"; sequence:number; }
export interface DocumentChanged { project_id:string; path:string; change:string; }
export interface DocumentProjectStatus { project_id:string; state:"healthy"|"error"; error:string|null; updated_at:string; }
export type ExpectedDocumentState = {state:"present";hash:string}|{state:"missing"};
export type Phase="defining"|"implementation"|"reviewing";
export type RunState="queued"|"running"|"succeeded"|"failed"|"stopped"|"interrupted";
export interface PhaseSession {id:string;project_id:string;task_id:string;phase:Phase;harness:string;external_session_id:string|null;created_at:string;updated_at:string}
export interface Run {id:string;session_id:string;project_id:string;task_id:string;sequence:number;state:RunState;prompt:string;harness:string;harness_version:string|null;model:string|null;work_dir:string;request_id:string|null;started_at:string|null;finished_at:string|null;exit_code:number|null;stderr:string;final_result:string|null;stop_requested_at:string|null;error:string|null;stage_at_start:Phase|null;workflow_event_id_before:number|null;prompt_kind:Phase|null;prompt_version:number|null;observed_transition_event_id:number|null;missing_transition:boolean;created_at:string;updated_at:string}
export interface RunEvent {run_id:string;sequence:number;kind:string;event_at:string;display_text:string|null;tool_name:string|null;tool_call_id:string|null;raw_json:string}
