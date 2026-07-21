import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { CraftelApi } from "./craftel";
import type { Document, DocumentProjectStatus, DocumentRevision, Project, Stage, Task, Run, PhaseSession, RunEvent } from "./types";

export const tauriApi: CraftelApi = {
  listProjects: () => invoke<Project[]>("list_projects"),
  async selectProjectDirectory() {
    const result = await open({ directory: true, multiple: false });
    return typeof result === "string" ? result : null;
  },
  registerProject: (name, path) => invoke("register_project", { name, path }),
  openProject: (id) => invoke("open_project", { id }),
  removeProject: (id) => invoke("remove_project", { id }),
  listTasks: (projectId) => invoke<Task[]>("list_tasks", { projectId }),
  createTask: (projectId, title, content) => invoke("create_task", { projectId, title, content }),
  updateTask: (projectId, taskId, title, content) => invoke("update_task", { projectId, taskId, title, content }),
  moveTask: (projectId, taskId, stage: Stage) => invoke("move_task", { projectId, taskId, stage }),
  listDocuments: (projectId, includeDeleted=false) => invoke<Document[]>("list_documents", { projectId, includeDeleted }),
  documentStatus: (projectId) => invoke<DocumentProjectStatus>("document_status", { projectId }),
  readDocument: (projectId,path) => invoke("read_document", {projectId,path}),
  searchDocuments: (projectId,query) => invoke<Document[]>("search_documents", {projectId,query}),
  writeDocument: (projectId,path,content,expectedState) => invoke("write_document", {projectId,path,content,expectedState}),
  listDocumentRevisions: (projectId,path) => invoke<DocumentRevision[]>("list_document_revisions", {projectId,path}),
  restoreDocumentRevision: (projectId,path,snapshotId,expectedState) => invoke("restore_document_revision", {projectId,path,snapshotId,expectedState}),
  startPhaseRun: (projectId,taskId,phase,prompt,_workDir) => invoke<Run>("start_phase_run",{projectId,taskId,phase,prompt}),
  stopRun: runId=>invoke<Run>("stop_run",{runId}), followUp:(sessionId,prompt)=>invoke<Run>("follow_up",{sessionId,prompt}),
  getSession:sessionId=>invoke<PhaseSession>("get_session",{sessionId}), listSessions:(projectId,taskId)=>invoke<PhaseSession[]>("list_sessions",{projectId,taskId}),
  listRuns:sessionId=>invoke<Run[]>("list_runs",{sessionId}), getRun:runId=>invoke<Run>("get_run",{runId}), listRunEvents:(runId,afterSequence,limit)=>invoke<RunEvent[]>("list_run_events",{runId,afterSequence,limit}),
};
