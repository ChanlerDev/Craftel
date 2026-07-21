import type { Document, DocumentProjectStatus, DocumentRevision, ExpectedDocumentState, Project, Stage, Task, PhaseSession, Run, RunEvent } from "./types";

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
}

export function errorMessage(error: unknown): string {
  if (typeof error === "object" && error && "message" in error) return String(error.message);
  return error instanceof Error ? error.message : String(error);
}
