import { vi } from "vitest";
import type { CraftelApi } from "../api/craftel";
import type { Project, Stage, Task } from "../api/types";
export const now = "2026-07-21T12:00:00Z";
export const project = (extra: Partial<Project> = {}): Project => ({ id: "p1", name: "Alpha", work_dir: "/tmp/alpha", available: true, created_at: now, last_opened_at: now, ...extra });
export const task = (stage: Stage = "inbox", extra: Partial<Task> = {}): Task => ({ id: "T0001", project_id: "p1", title: "Useful title", content: "Representative content for this card", stage, relative_dir: ".craftel/tasks/T0001", review_approved: false, created_at: now, updated_at: now, ...extra });
export function fakeApi(overrides: Partial<CraftelApi> = {}): CraftelApi {
  return { listProjects: vi.fn().mockResolvedValue([]), selectProjectDirectory: vi.fn().mockResolvedValue(null), registerProject: vi.fn(), openProject: vi.fn(), removeProject: vi.fn(), listTasks: vi.fn().mockResolvedValue([]), createTask: vi.fn(), updateTask: vi.fn(), moveTask: vi.fn(), ...overrides };
}
