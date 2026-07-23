import type { Document, DocumentProjectStatus, DocumentRevision, ExpectedDocumentState, GitWorkingCopySummary, Project, Stage, Task, PhaseSession, Run, RunEvent } from "./types";

export interface CraftelApi {
  listProjects(): Promise<Project[]>;
  selectProjectDirectory(): Promise<string | null>;
  registerProject(name: string, path: string): Promise<Project>;
  openProject(id: string): Promise<Project>;
  removeProject(id: string): Promise<void>;
  gitWorkingCopySummary(projectId:string):Promise<GitWorkingCopySummary>;
  listTasks(projectId: string): Promise<Task[]>;
  createTask(projectId: string, title: string, content: string): Promise<Task>;
  updateTask(projectId: string, taskId: string, title: string, content: string): Promise<Task>;
  moveTask(projectId: string, taskId: string, stage: Stage): Promise<Task>;
  nextTask(projectId: string, taskId: string): Promise<Task>;
  listDocuments(projectId:string, includeDeleted?:boolean):Promise<Document[]>;
  documentStatus(projectId:string):Promise<DocumentProjectStatus>;
  readDocument(projectId:string,path:string):Promise<Document>;
  searchDocuments(projectId:string,query:string):Promise<Document[]>;
  writeDocument(projectId:string,path:string,content:string,expectedState:ExpectedDocumentState):Promise<Document>;
  listDocumentRevisions(projectId:string,path:string):Promise<DocumentRevision[]>;
  restoreDocumentRevision(projectId:string,path:string,snapshotId:string,expectedState:ExpectedDocumentState):Promise<Document>;
  startCurrentPhase(projectId:string,taskId:string):Promise<Run>;
  stopRun(runId:string):Promise<Run>;
  getSession(sessionId:string):Promise<PhaseSession>; listSessions(projectId:string,taskId:string):Promise<PhaseSession[]>;
  listRuns(sessionId:string):Promise<Run[]>; getRun(runId:string):Promise<Run>; listRunEvents(runId:string,afterSequence:number,limit:number):Promise<RunEvent[]>;
  listActiveRuns(projectId:string):Promise<Run[]>;
  followUp(sessionId:string,prompt:string):Promise<Run>;
  subscribe(event:"document_changed"|"run_event"|"run_changed", handler:(payload:unknown)=>void):Promise<()=>void>;
}

export function errorMessage(error: unknown): string {
  const raw=typeof error === "object" && error && "message" in error ? String(error.message) : error instanceof Error ? error.message : "The operation failed";
  return raw.replace(/(?:[A-Za-z]:)?[\\/](?:[^\s:]+[\\/])+[^\s:]*/g,"a local file").slice(0,500);
}

export function errorCode(error: unknown): string {
  return typeof error === "object" && error && "code" in error ? String(error.code) : "io";
}
