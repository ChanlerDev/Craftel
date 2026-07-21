import type { Project, Stage, Task } from "./types";

export interface CraftelApi {
  listProjects(): Promise<Project[]>;
  selectProjectDirectory(): Promise<string | null>;
  registerProject(name: string, path: string): Promise<Project>;
  openProject(id: string): Promise<Project>;
  removeProject(id: string): Promise<void>;
  listTasks(projectId: string): Promise<Task[]>;
  createTask(projectId: string, title: string, content: string): Promise<Task>;
  updateTask(projectId: string, taskId: string, title: string, content: string): Promise<Task>;
  moveTask(projectId: string, taskId: string, stage: Stage): Promise<Task>;
}

export function errorMessage(error: unknown): string {
  if (typeof error === "object" && error && "message" in error) return String(error.message);
  return error instanceof Error ? error.message : String(error);
}
