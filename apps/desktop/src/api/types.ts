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
