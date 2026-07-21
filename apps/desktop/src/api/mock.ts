import type { CraftelApi } from "./craftel";
import type { Project, Stage, Task } from "./types";

const now = new Date().toISOString();
const project: Project = { id: "demo", name: "Product workspace", work_dir: "/work/craftel", available: true, created_at: now, last_opened_at: now };
const samples: Array<[Stage, string, string]> = [
  ["inbox", "Capture onboarding ideas", "Collect the open questions and rough constraints before defining the work."],
  ["defining", "Design project import and recovery behavior for moved working directories", "Document edge cases, acceptance criteria, and an intentionally long excerpt that demonstrates how cards remain readable without taking over the board."],
  ["implementation", "Build durable task projection", "Keep SQLite authoritative and replace TASK.md atomically."],
  ["reviewing", "Audit keyboard workflows", "Verify all board operations remain available without a pointer."],
  ["done", "Create application foundation", "The workspace, storage, and command boundary are complete."],
];
let tasks: Task[] = samples.map(([stage, title, content], i) => ({ id: `T000${i + 1}`, project_id: project.id, title, content, stage, relative_dir: `.craftel/tasks/T000${i + 1}`, review_approved: false, created_at: now, updated_at: now }));
export const mockApi: CraftelApi = {
  listProjects: async () => [project], selectProjectDirectory: async () => null,
  registerProject: async () => project, openProject: async () => project, removeProject: async () => {},
  listTasks: async () => [...tasks],
  createTask: async (projectId, title, content) => { const task: Task = { ...tasks[0], id: `T${String(tasks.length + 1).padStart(4, "0")}`, project_id: projectId, title, content, stage: "inbox", updated_at: now }; tasks = [...tasks, task]; return task; },
  updateTask: async (_p, id, title, content) => { const task = { ...tasks.find(t => t.id === id)!, title, content, updated_at: now }; tasks = tasks.map(t => t.id === id ? task : t); return task; },
  moveTask: async (_p, id, stage) => { const task = { ...tasks.find(t => t.id === id)!, stage, updated_at: now }; tasks = tasks.map(t => t.id === id ? task : t); return task; },
};
