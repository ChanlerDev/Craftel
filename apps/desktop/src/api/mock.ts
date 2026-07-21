import type { CraftelApi } from "./craftel";
import type { Document, DocumentRevision, ExpectedDocumentState, Phase, PhaseSession, Project, Run, RunEvent, RunState, Stage, Task } from "./types";

const now="2026-07-21T14:30:00.000Z", project:Project={id:"demo",name:"Product workspace",work_dir:"/work/craftel",available:true,created_at:now,last_opened_at:now};
const samples:Array<[Stage,string,string]>=[
  ["inbox","Capture onboarding ideas","Collect open questions before defining the work."],
  ["defining","Design project import and recovery","Explore constraints and produce a durable specification."],
  ["implementation","Build durable task projection","Keep SQLite authoritative and replace documents atomically."],
  ["reviewing","Audit keyboard workflows","Review the completed implementation and its accessibility evidence."],
  ["done","Create application foundation","The workspace, storage, and command boundary are complete."],
];
let tasks:Task[]=samples.map(([stage,title,content],i)=>({id:`T000${i+1}`,project_id:project.id,title,content,stage,relative_dir:`craftel/tasks/T000${i+1}-${title.toLowerCase().replaceAll(/[^a-z0-9]+/g,"-").replaceAll(/(^-|-$)/g,"")}`,review_approved:stage==="reviewing",created_at:now,updated_at:now}));
const markdown=`# Recovery specification

> This development fixture is intentionally detailed so typography, scrolling, and sanitized preview rendering can be reviewed.

## Goal

Recover moved workspaces without losing task history. The scanner remains authoritative, writes are atomic, and conflicts preserve the editor's draft.

## Acceptance criteria

- Detect a missing working directory and explain the recovery action.
- Re-index **SPEC.md**, plans, and review notes deterministically.
- Reject stale writes using the last observed content hash.
- Preserve revision history when a deleted document is restored.

| Scenario | Expected result |
| --- | --- |
| Workspace moved | User chooses the new directory |
| Document changed externally | Draft remains visible with reload and copy actions |
| Deleted plan restored | New restore revision is appended |

## Security and privacy

Paths, account names, tokens, and remote URLs shown here are synthetic. <script>alert("unsafe")</script> Raw HTML must not execute. [Unsafe links](javascript:alert('no')) must be removed by the sanitizer.

### Review checklist

1. Navigate the document tree using only the keyboard.
2. Compare scan, edit, and restore revisions.
3. Start a phase, inspect live activity, stop it, then send a follow-up.
4. Confirm assistant messages and tool calls remain in sequence.

\`\`\`ts
async function replaceAtomically(path: string, body: string) {
  await writeTemporaryFile(path, body);
  await renameOverDestination(path);
}
\`\`\`

## Decision record

We prefer explicit expected-state checks over last-write-wins. This makes concurrent edits visible and recoverable. The interface should remain useful on narrow screens, with document and activity panels available as tabs. Long content also verifies that controls remain discoverable after substantial scrolling.
`;
const hash=(body:string)=>`sha256-${body.length.toString(16).padStart(8,"0")}`;
let documents:Document[]=[
  {project_id:project.id,relative_path:`${tasks[1].relative_dir}/SPEC.md`,task_id:"T0002",title:"Recovery specification",body:markdown,content_hash:hash(markdown),present:true,indexed_at:now},
  {project_id:project.id,relative_path:`${tasks[2].relative_dir}/plans/implementation.md`,task_id:"T0003",title:"Implementation plan",body:"# Implementation plan\n\n- Add expected-state checks\n- Record revisions\n- Verify atomic replacement\n",content_hash:hash("plan"),present:true,indexed_at:now},
  {project_id:project.id,relative_path:`${tasks[3].relative_dir}/reviews/accessibility.md`,task_id:"T0004",title:"Review findings",body:"# Review findings\n\nKeyboard paths pass. Verify interrupted run recovery and missing transitions before approval.\n",content_hash:hash("review"),present:true,indexed_at:now},
  {project_id:project.id,relative_path:`${tasks[1].relative_dir}/notes/deleted-plan.md`,task_id:"T0002",title:"Deleted recovery plan",body:"# Earlier recovery plan\n\nThis deleted document can be inspected and restored from revision history.\n",content_hash:hash("deleted"),present:false,indexed_at:now},
];
const bytes=(s:string)=>Array.from(new TextEncoder().encode(s));
let revisions:DocumentRevision[]=documents.flatMap((d,i)=>[
  {id:`revision-${i}-2`,project_id:project.id,relative_path:d.relative_path,content_hash:d.content_hash,content:bytes(d.body),captured_at:`2026-07-21T13:${20+i}:00.000Z`,cause:d.present?"edit":"watch",sequence:2},
  {id:`revision-${i}-1`,project_id:project.id,relative_path:d.relative_path,content_hash:hash("Earlier draft"),content:bytes(`# Earlier draft\n\nRevision evidence for ${d.title}.\n`),captured_at:`2026-07-20T10:${20+i}:00.000Z`,cause:"scan",sequence:1},
]);
const session=(id:string,task_id:string,phase:Phase,external=true):PhaseSession=>({id,project_id:project.id,task_id,phase,harness:"cursor",external_session_id:external?`external-${id}`:null,created_at:now,updated_at:now});
let sessions:PhaseSession[]=[session("session-defining-active","T0002","defining"),session("session-defining-complete","T0003","defining"),session("session-implementation","T0003","implementation"),session("session-review-approved","T0004","reviewing",false)];
const makeRun=(id:string,session_id:string,task_id:string,sequence:number,state:RunState,transition:"observed"|"missing"|null=null):Run=>({id,session_id,project_id:project.id,task_id,sequence,state,prompt:`Exercise ${state} phase behavior`,harness:"cursor",harness_version:"1.4.0",model:"review-model",work_dir:project.work_dir,request_id:`request-${id}`,started_at:state==="queued"?null:"2026-07-21T14:00:00.000Z",finished_at:["queued","running"].includes(state)?null:"2026-07-21T14:04:00.000Z",exit_code:state==="succeeded"?0:state==="failed"?1:null,stderr:state==="failed"?"Synthetic harness failure (no private paths).":"",final_result:state==="succeeded"?"Phase evidence completed.":null,stop_requested_at:state==="stopped"?"2026-07-21T14:03:00.000Z":null,error:state==="interrupted"?"Application restarted while this run was active.":state==="failed"?"Harness exited unsuccessfully.":null,stage_at_start:sessions.find(s=>s.id===session_id)!.phase,workflow_event_id_before:42,prompt_kind:sessions.find(s=>s.id===session_id)!.phase,prompt_version:1,observed_transition_event_id:transition==="observed"?43:null,missing_transition:transition==="missing",created_at:`2026-07-21T14:0${sequence}:00.000Z`,updated_at:now});
let runs:Run[]=[makeRun("run-defining-active","session-defining-active","T0002",2,"running"),makeRun("run-defining-failed","session-defining-active","T0002",1,"failed"),makeRun("run-defining-pass","session-defining-complete","T0003",1,"succeeded","observed"),makeRun("run-implementation-interrupted","session-implementation","T0003",1,"interrupted"),{...makeRun("run-review-approved","session-review-approved","T0004",1,"succeeded","observed"),final_result:"Automated review passed. Keyboard workflows and recovery evidence are ready for human delivery."}];
const event=(run_id:string,sequence:number,kind:string,display_text:string|null,tool_name:string|null=null,tool_call_id:string|null=null):RunEvent=>({run_id,sequence,kind,event_at:`2026-07-21T14:00:0${sequence}.000Z`,display_text,tool_name,tool_call_id,raw_json:JSON.stringify({kind,fixture:true})});
let events:RunEvent[]=runs.flatMap(r=>[event(r.id,1,"assistant","I’ll inspect the phase contract and current task artifacts."),event(r.id,2,"tool_start","Reading indexed documents…","read_documents","call-1"),event(r.id,3,"tool_complete","Found eligible documents and revision history.","read_documents","call-1"),event(r.id,4,"assistant","I updated the primary artifact and recorded the evidence for this attempt.")]);
const listeners=new Map<string,Set<(p:unknown)=>void>>();
const emit=(name:string,payload:unknown)=>listeners.get(name)?.forEach(fn=>fn(payload));
const conflict=()=>Object.assign(new Error("This document changed since it was opened."),{code:"conflict"});
const assertExpected=(doc:Document|undefined,expected:ExpectedDocumentState)=>{if(expected.state==="missing" ? doc?.present : !doc?.present||doc.content_hash!==expected.hash)throw conflict()};

export const mockApi:CraftelApi={
  listProjects:async()=>[project],selectProjectDirectory:async()=>null,registerProject:async()=>project,openProject:async()=>project,removeProject:async()=>{},listTasks:async()=>[...tasks],
  createTask:async(projectId,title,content)=>{const task:Task={...tasks[0],id:`T${String(tasks.length+1).padStart(4,"0")}`,project_id:projectId,title,content,stage:"inbox",updated_at:now};tasks=[...tasks,task];return task},
  updateTask:async(_p,id,title,content)=>{const task={...tasks.find(t=>t.id===id)!,title,content,updated_at:now};tasks=tasks.map(t=>t.id===id?task:t);return task},moveTask:async(_p,id,stage)=>{const task={...tasks.find(t=>t.id===id)!,stage,review_approved:false,updated_at:now};tasks=tasks.map(t=>t.id===id?task:t);return task},nextTask:async(_p,id)=>{const current=tasks.find(t=>t.id===id)!;const next:Record<Stage,Stage|null>={inbox:"defining",defining:"implementation",implementation:"reviewing",reviewing:current.review_approved?"done":null,done:null};if(!next[current.stage])throw new Error("Task cannot advance from its current state");const updated={...current,stage:next[current.stage]!,review_approved:false,updated_at:new Date().toISOString()};tasks=tasks.map(t=>t.id===id?updated:t);emit("run_changed",{project_id:project.id,task_id:id});return{...updated}},
  listDocuments:async(_p,includeDeleted=false)=>documents.filter(d=>includeDeleted||d.present).map(d=>({...d})),documentStatus:async()=>({project_id:project.id,state:"healthy",error:null,updated_at:now}),readDocument:async(_p,path)=>{const d=documents.find(x=>x.relative_path===path);if(!d)throw new Error("Document not found");return{...d}},searchDocuments:async(_p,q)=>documents.filter(d=>d.present&&`${d.title} ${d.body}`.toLowerCase().includes(q.toLowerCase())),
  writeDocument:async(_p,path,content,expected)=>{const old=documents.find(d=>d.relative_path===path);assertExpected(old,expected);const saved={...old!,body:content,content_hash:hash(content),indexed_at:new Date().toISOString()};revisions.push({id:`revision-${revisions.length+1}`,project_id:project.id,relative_path:path,content_hash:saved.content_hash,content:bytes(content),captured_at:saved.indexed_at,cause:"edit",sequence:revisions.filter(r=>r.relative_path===path).length+1});documents=documents.map(d=>d.relative_path===path?saved:d);emit("document_changed",{project_id:project.id,path,change:"write"});return{...saved}},
  listDocumentRevisions:async(_p,path)=>revisions.filter(r=>r.relative_path===path).map(r=>({...r,content:[...r.content]})),restoreDocumentRevision:async(_p,path,id,expected)=>{const old=documents.find(d=>d.relative_path===path),rev=revisions.find(r=>r.id===id&&r.relative_path===path);if(!rev)throw new Error("Revision not found");assertExpected(old,expected);const body=new TextDecoder().decode(new Uint8Array(rev.content)),saved:Document={...old!,body,present:true,content_hash:hash(body),indexed_at:new Date().toISOString()};documents=documents.map(d=>d.relative_path===path?saved:d);revisions.push({...rev,id:`revision-${revisions.length+1}`,content_hash:saved.content_hash,captured_at:saved.indexed_at,cause:"restore",sequence:revisions.filter(r=>r.relative_path===path).length+1});emit("document_changed",{project_id:project.id,path,change:"restore"});return{...saved}},
  startCurrentPhase:async(_p,taskId)=>{const task=tasks.find(t=>t.id===taskId)!;if(runs.some(r=>r.task_id===taskId&&["queued","running"].includes(r.state)))throw new Error("Task already has an active run");if(!["defining","implementation","reviewing"].includes(task.stage))throw new Error("Task is not in an eligible phase");let s=task.stage==="reviewing"?undefined:sessions.find(x=>x.task_id===taskId&&x.phase===task.stage);if(!s){s=session(`session-${sessions.length+1}`,taskId,task.stage as Phase,task.stage!=="reviewing");sessions.push(s)}const r=makeRun(`run-${Date.now()}`,s.id,taskId,runs.filter(x=>x.session_id===s!.id).length+1,"running");runs.push(r);events.push(event(r.id,1,"assistant","New mock phase started."));if(task.stage==="defining"&&!documents.some(d=>d.task_id===taskId&&d.relative_path.endsWith("/SPEC.md"))){const body=`# ${task.title}\n\n## Context\n\n${task.content}\n\n## Acceptance criteria\n\n- Clarify the expected user outcome.\n- Record testable constraints before implementation.\n`;const spec:Document={project_id:project.id,relative_path:`${task.relative_dir}/SPEC.md`,task_id:taskId,title:`${task.title} specification`,body,content_hash:hash(body),present:true,indexed_at:new Date().toISOString()};documents.push(spec);emit("document_changed",{project_id:project.id,path:spec.relative_path,change:"write"})}emit("run_changed",{project_id:project.id,run_id:r.id});return{...r}},
  stopRun:async id=>{const found=runs.find(r=>r.id===id);if(!found)throw new Error("Run not found");const stopped={...found,state:"stopped" as const,stop_requested_at:new Date().toISOString(),finished_at:new Date().toISOString(),updated_at:new Date().toISOString()};runs=runs.map(r=>r.id===id?stopped:r);emit("run_changed",{project_id:project.id,run_id:id});return{...stopped}},getSession:async id=>({...sessions.find(s=>s.id===id)!}),listSessions:async(_p,taskId)=>sessions.filter(s=>s.task_id===taskId).map(s=>({...s})),listRuns:async id=>runs.filter(r=>r.session_id===id).map(r=>({...r})),getRun:async id=>({...runs.find(r=>r.id===id)!}),listRunEvents:async(id,after,limit)=>events.filter(e=>e.run_id===id&&e.sequence>after).slice(0,limit).map(e=>({...e})),listActiveRuns:async()=>runs.filter(r=>r.state==="queued"||r.state==="running").map(r=>({...r})),
  followUp:async(sessionId,prompt)=>{const s=sessions.find(x=>x.id===sessionId);if(!s?.external_session_id)throw new Error("Session cannot be resumed");const r=makeRun(`follow-up-${Date.now()}`,sessionId,s.task_id,runs.filter(x=>x.session_id===sessionId).length+1,"running");r.prompt=prompt;runs.push(r);events.push(event(r.id,1,"user",prompt),event(r.id,2,"assistant",`I’ll continue the ${s.phase} work and update its artifact.`));emit("run_changed",{project_id:project.id,run_id:r.id});emit("run_event",{project_id:project.id,run_id:r.id});return{...r}},
  subscribe:async(name,handler)=>{const set=listeners.get(name)??new Set();set.add(handler);listeners.set(name,set);return()=>set.delete(handler)},
};
