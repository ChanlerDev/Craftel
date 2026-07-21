import {act,render,screen,waitFor,within} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {vi} from "vitest";
import {TaskWorkspace} from "./TaskWorkspace";
import {deferred,document as doc,fakeApi,project,revision,run,session,task} from "../test/fake";

const renderWorkspace=(overrides={},stage:"defining"|"implementation"|"reviewing"|"done"="defining")=>{
 const api=fakeApi({listTasks:vi.fn().mockResolvedValue([task(stage)]),...overrides});
 return {api,...render(<TaskWorkspace api={api} project={project()} task={task(stage)} onBack={vi.fn()}/>)};
};

test("document race selects only the last response and groups the tree",async()=>{
 const a=doc("tasks/T0001/notes/A.md"),b=doc("tasks/T0001/plans/B.md");
 const late=deferred<typeof a>(); const read=vi.fn().mockResolvedValueOnce(a).mockReturnValueOnce(late.promise).mockResolvedValueOnce(b);
 renderWorkspace({listDocuments:vi.fn().mockResolvedValue([a,b]),readDocument:read});
 await screen.findByDisplayValue("# Draft");
 await userEvent.click(screen.getByRole("button",{name:"A.md"}));
 await userEvent.click(screen.getByRole("button",{name:"B.md"}));
 expect(await screen.findByRole("button",{name:"B.md"})).toHaveAttribute("aria-current","page");
 late.resolve(doc(a.relative_path,{body:"stale"})); await Promise.resolve();
 expect(screen.getByLabelText("Markdown document")).toHaveValue("# Draft");
 const tree=screen.getByRole("navigation",{name:"Task documents"});expect(within(tree).getByText("T0001/")).toBeInTheDocument();expect(within(tree).getByText("TASK.md")).toBeInTheDocument();expect(within(tree).getByText("notes/")).toBeInTheDocument();expect(within(tree).getByText("plans/")).toBeInTheDocument();expect(screen.queryByText("Representative content for this card")).toBeNull();
});

test("preview sanitizes scripts and raw HTML",async()=>{
 const body="# Safe\n<script>alert(1)</script><img src=x onerror=alert(2)>";renderWorkspace({listDocuments:vi.fn().mockResolvedValue([doc(undefined,{body})]),readDocument:vi.fn().mockResolvedValue(doc(undefined,{body}))});
 await userEvent.click(await screen.findByRole("button",{name:"Preview"}));
 expect(screen.getByRole("heading",{name:"Safe"})).toBeInTheDocument();expect(document.querySelector("script")).toBeNull();expect(document.querySelector("img")).toBeNull();
});

test("split mode edits Markdown with a live sanitized preview",async()=>{
 const d=doc();renderWorkspace({listDocuments:vi.fn().mockResolvedValue([d]),readDocument:vi.fn().mockResolvedValue(d)});await screen.findByLabelText("Markdown document");await userEvent.click(screen.getByRole("button",{name:"Split"}));const editor=screen.getByLabelText("Markdown document");await userEvent.clear(editor);await userEvent.type(editor,"# Live heading");expect(screen.getByRole("heading",{name:"Live heading"})).toBeInTheDocument();expect(screen.getByRole("button",{name:"Split"})).toHaveAttribute("aria-pressed","true");await userEvent.click(screen.getByRole("button",{name:"Edit"}));expect(screen.queryByLabelText("Live Markdown preview")).toBeNull();await userEvent.click(screen.getByRole("button",{name:"Preview"}));expect(screen.getByRole("heading",{name:"Live heading"})).toBeInTheDocument();
});

test("TASK.md renders the managed SQLite projection without edit controls",async()=>{
 renderWorkspace();await screen.findByRole("button",{name:/TASK.md/});const editor=document.querySelector<HTMLElement>(".editor")!;expect(within(editor).getByRole("heading",{name:"Useful title"})).toBeInTheDocument();expect(within(editor).getByText(/SQLite task metadata/)).toBeInTheDocument();expect(within(editor).queryByRole("button",{name:"Edit"})).toBeNull();expect(within(editor).queryByRole("button",{name:"Save"})).toBeNull();
});

test("save conflict preserves draft and offers reload and copy",async()=>{
 const d=doc();const write=vi.fn().mockRejectedValue({code:"conflict",message:"changed at /secret/private/file.md"});
 const clipboard=vi.fn().mockResolvedValue(undefined);Object.defineProperty(navigator,"clipboard",{configurable:true,value:{writeText:clipboard}});
 renderWorkspace({listDocuments:vi.fn().mockResolvedValue([d]),readDocument:vi.fn().mockResolvedValue(d),writeDocument:write});
 const editor=await screen.findByLabelText("Markdown document");await userEvent.clear(editor);await userEvent.type(editor,"my draft");await userEvent.click(screen.getByRole("button",{name:"Save"}));
 expect(await screen.findByText(/edits are preserved/)).toBeInTheDocument();expect(editor).toHaveValue("my draft");expect(screen.getAllByRole("alert").map(x=>x.textContent).join(" ")).not.toContain("secret");
 await userEvent.click(screen.getByRole("button",{name:"Copy edits"}));expect(clipboard).toHaveBeenCalledWith("my draft");expect(screen.getByRole("button",{name:"Reload"})).toBeEnabled();
});

test("document hints refresh changed metadata without replacing a dirty draft",async()=>{
 let hint:(p:unknown)=>void=()=>{};const d=doc(),changed={...d,content_hash:"hash-2",body:"external"};const api=fakeApi({listTasks:vi.fn().mockResolvedValue([task("defining")]),listDocuments:vi.fn().mockResolvedValueOnce([d]).mockResolvedValueOnce([changed]),readDocument:vi.fn().mockResolvedValue(d),subscribe:vi.fn(async(name,handler)=>{if(name==="document_changed")hint=handler;return()=>{}})});
 render(<TaskWorkspace api={api} project={project()} task={task("defining")} onBack={()=>{}}/>);const editor=await screen.findByLabelText("Markdown document");await userEvent.clear(editor);await userEvent.type(editor,"dirty");hint({project_id:"p1"});
 await waitFor(()=>expect(api.listDocuments).toHaveBeenCalledTimes(2));expect(editor).toHaveValue("dirty");
});

test("deleted restore uses Expected Missing and revision modal traps focus, closes, and returns focus",async()=>{
 const deleted=doc(undefined,{present:false});const restore=vi.fn().mockResolvedValue(doc());vi.spyOn(window,"confirm").mockReturnValue(true);
 renderWorkspace({listDocuments:vi.fn().mockResolvedValue([deleted]),readDocument:vi.fn().mockResolvedValue(deleted),listDocumentRevisions:vi.fn().mockResolvedValue([revision()]),restoreDocumentRevision:restore});
 const trigger=await screen.findByRole("button",{name:"Revisions"});await userEvent.click(trigger);const dialog=screen.getByRole("dialog");expect(within(dialog).getByText(/· deleted/)).toBeInTheDocument();
 expect(within(dialog).getAllByRole("button")[0]).toHaveFocus();await userEvent.tab({shift:true});expect(within(dialog).getByRole("button",{name:"Restore revision"})).toHaveFocus();
 await userEvent.click(within(dialog).getByRole("button",{name:"Restore revision"}));await waitFor(()=>expect(restore).toHaveBeenCalledWith("p1",deleted.relative_path,"rev-1",{state:"missing"}));
 await userEvent.keyboard("{Escape}");expect(screen.queryByRole("dialog")).not.toBeInTheDocument();expect(trigger).toHaveFocus();
});

test("controls load disabled, serialize activation, and name the current phase",async()=>{
 const start=deferred<ReturnType<typeof run>>();const startCurrentPhase=vi.fn(()=>start.promise);const loaded=deferred<ReturnType<typeof task>[]>();
 const {api}=renderWorkspace({listTasks:vi.fn(()=>loaded.promise),startCurrentPhase});expect(screen.queryByRole("button",{name:/Start/})).not.toBeInTheDocument();loaded.resolve([task("defining")]);
 const button=await screen.findByRole("button",{name:"Start Defining"});expect(screen.getByText("Define the contract")).toBeInTheDocument();await userEvent.dblClick(button);expect(startCurrentPhase).toHaveBeenCalledTimes(1);start.resolve(run("running"));
 await waitFor(()=>expect(api.listTasks).toHaveBeenCalled());
});

test("active controls are Stop-only and stop is idempotent",async()=>{
 const active=run("running");const stop=deferred<typeof active>();const stopRun=vi.fn(()=>stop.promise);
 renderWorkspace({listSessions:vi.fn().mockResolvedValue([session()]),listRuns:vi.fn().mockResolvedValue([active]),stopRun});
 const button=await screen.findByRole("button",{name:"Stop run"});expect(screen.queryByRole("button",{name:/Start/})).toBeNull();await userEvent.dblClick(button);expect(stopRun).toHaveBeenCalledTimes(1);stop.resolve(run("stopped"));
});

test("follow-up is current-phase terminal resumable only and review starts fresh",async()=>{
 const terminal=run();const {unmount}=renderWorkspace({listSessions:vi.fn().mockResolvedValue([session()]),listRuns:vi.fn().mockResolvedValue([terminal])});const composer=await screen.findByLabelText("Message the defining agent");expect(screen.getByRole("button",{name:"Move to Implementation"})).toBeEnabled();expect(screen.getByRole("button",{name:"Continue Defining"})).toBeDisabled();await userEvent.type(composer,"Tighten the acceptance criteria");expect(screen.getByRole("button",{name:"Continue Defining"})).toBeEnabled();unmount();
 renderWorkspace({listSessions:vi.fn().mockResolvedValue([session({external_session_id:null})]),listRuns:vi.fn().mockResolvedValue([terminal])},"reviewing");expect(await screen.findByRole("button",{name:"Start fresh Review"})).toBeInTheDocument();expect(screen.queryByLabelText("Follow-up")).toBeNull();
});

test("approved review waits for an explicit human next",async()=>{
 const approved=task("reviewing",{review_approved:true}),nextTask=vi.fn().mockResolvedValue(task("done"));const api=fakeApi({listTasks:vi.fn().mockResolvedValue([approved]),nextTask});
 render(<TaskWorkspace api={api} project={project()} task={approved} onBack={()=>{}}/>);expect(await screen.findByText("Approved · awaiting human")).toBeInTheDocument();await userEvent.click(screen.getByRole("button",{name:"Mark Done"}));expect(nextTask).toHaveBeenCalledWith("p1","T0001");
});

test("run change refreshes the review verdict and legal action",async()=>{
 let approved=false,runChanged:(payload:unknown)=>void=()=>{};const api=fakeApi({listTasks:vi.fn(async()=>[task("reviewing",{review_approved:approved})]),subscribe:vi.fn(async(name,handler)=>{if(name==="run_changed")runChanged=handler;return()=>{}})});
 render(<TaskWorkspace api={api} project={project()} task={task("reviewing")} onBack={()=>{}}/>);expect(await screen.findByRole("button",{name:"Start fresh Review"})).toBeInTheDocument();approved=true;act(()=>runChanged({project_id:"p1"}));expect(await screen.findByRole("button",{name:"Mark Done"})).toBeInTheDocument();
});

test("reviewing prefers review evidence over the specification",async()=>{
 const spec=doc("tasks/T0001/SPEC.md",{body:"# Specification"}),oldReview=doc("tasks/T0001/reviews/Z-old.md",{body:"# Old review",indexed_at:"2026-07-20T12:00:00Z"}),review=doc("tasks/T0001/reviews/A-new.md",{body:"# New review",indexed_at:"2026-07-22T12:00:00Z"});renderWorkspace({listDocuments:vi.fn().mockResolvedValue([spec,oldReview,review]),readDocument:vi.fn(async(_p,path)=>[spec,oldReview,review].find(value=>value.relative_path===path)!)},"reviewing");expect(await screen.findByLabelText("Markdown document")).toHaveValue("# New review");
});

test("stage transition replaces an automatic plan selection with review evidence",async()=>{
 let stage:"implementation"|"reviewing"="implementation",runChanged:(payload:unknown)=>void=()=>{};const plan=doc("tasks/T0001/PLAN.md",{body:"# Plan"}),review=doc("tasks/T0001/REVIEW.md",{body:"# Findings",indexed_at:"2026-07-22T12:00:00Z"}),api=fakeApi({listTasks:vi.fn(async()=>[task(stage)]),listDocuments:vi.fn().mockResolvedValue([plan,review]),readDocument:vi.fn(async(_p,path)=>path===review.relative_path?review:plan),subscribe:vi.fn(async(name,handler)=>{if(name==="run_changed")runChanged=handler;return()=>{}})});render(<TaskWorkspace api={api} project={project()} task={task("implementation")} onBack={()=>{}}/>);expect(await screen.findByLabelText("Markdown document")).toHaveValue("# Plan");stage="reviewing";act(()=>runChanged({project_id:"p1"}));expect(await screen.findByDisplayValue("# Findings")).toBeInTheDocument();
});

test("saved work no longer blocks switching to the next phase artifact",async()=>{
 let stage:"implementation"|"reviewing"="implementation",runChanged:(payload:unknown)=>void=()=>{};const plan=doc("tasks/T0001/PLAN.md",{body:"# Plan"}),saved={...plan,body:"approved plan",content_hash:"saved"},review=doc("tasks/T0001/REVIEW.md",{body:"# Findings",indexed_at:"2026-07-22T12:00:00Z"}),api=fakeApi({listTasks:vi.fn(async()=>[task(stage)]),listDocuments:vi.fn().mockResolvedValue([plan,review]),readDocument:vi.fn(async(_p,path)=>path===review.relative_path?review:plan),writeDocument:vi.fn().mockResolvedValue(saved),subscribe:vi.fn(async(name,handler)=>{if(name==="run_changed")runChanged=handler;return()=>{}})});render(<TaskWorkspace api={api} project={project()} task={task("implementation")} onBack={()=>{}}/>);const editor=await screen.findByLabelText("Markdown document");await userEvent.clear(editor);await userEvent.type(editor,"approved plan");await userEvent.click(screen.getByRole("button",{name:"Save"}));await waitFor(()=>expect(editor).toHaveValue("approved plan"));stage="reviewing";act(()=>runChanged({project_id:"p1"}));expect(await screen.findByDisplayValue("# Findings")).toBeInTheDocument();
});

test("editing during an automatic phase artifact read preserves the draft",async()=>{
 let stage:"implementation"|"reviewing"="implementation",runChanged:(payload:unknown)=>void=()=>{};const plan=doc("tasks/T0001/PLAN.md",{body:"# Plan"}),review=doc("tasks/T0001/REVIEW.md",{body:"# Findings",indexed_at:"2026-07-22T12:00:00Z"}),reviewRead=deferred<typeof review>(),readDocument=vi.fn(async(_p:string,path:string)=>path===review.relative_path?reviewRead.promise:plan),api=fakeApi({listTasks:vi.fn(async()=>[task(stage)]),listDocuments:vi.fn().mockResolvedValue([plan,review]),readDocument,subscribe:vi.fn(async(name,handler)=>{if(name==="run_changed")runChanged=handler;return()=>{}})});render(<TaskWorkspace api={api} project={project()} task={task("implementation")} onBack={()=>{}}/>);const editor=await screen.findByLabelText("Markdown document");stage="reviewing";act(()=>runChanged({project_id:"p1"}));await waitFor(()=>expect(readDocument).toHaveBeenCalledWith("p1",review.relative_path));await userEvent.clear(editor);await userEvent.type(editor,"unsaved plan");await act(async()=>reviewRead.resolve(review));expect(editor).toHaveValue("unsaved plan");
});

test("done is a read-only delivery context",async()=>{
 const review=doc("tasks/T0001/REVIEW.md",{body:"# Approved"});renderWorkspace({listDocuments:vi.fn().mockResolvedValue([review]),readDocument:vi.fn().mockResolvedValue(review),listDocumentRevisions:vi.fn().mockResolvedValue([revision()])},"done");expect(await screen.findByRole("heading",{name:"Approved"})).toBeInTheDocument();expect(screen.queryByRole("button",{name:"Edit"})).toBeNull();expect(screen.queryByRole("button",{name:"Save"})).toBeNull();expect(screen.queryByLabelText("Markdown document")).toBeNull();await userEvent.click(screen.getByRole("button",{name:"Revisions"}));expect(await screen.findByRole("dialog")).toBeInTheDocument();expect(screen.queryByRole("button",{name:"Restore revision"})).toBeNull();
});

test("narrow tabs expose selection semantics and keyboard activation",async()=>{renderWorkspace();const docs=screen.getByRole("tab",{name:"Documents"}),activity=screen.getByRole("tab",{name:"Activity"});expect(docs).toHaveAttribute("aria-selected","true");docs.focus();await userEvent.tab();expect(activity).toHaveFocus();await userEvent.keyboard(" ");expect(activity).toHaveAttribute("aria-selected","true");});

test("operation-specific safe errors retain the correct retry",async()=>{renderWorkspace({listDocuments:vi.fn().mockRejectedValue({message:"failed /home/me/private/TASK.md"}),listTasks:vi.fn().mockRejectedValue({message:"controls down"})});expect(await screen.findByRole("button",{name:"Retry"})).toBeInTheDocument();expect(await screen.findByRole("button",{name:"Retry controls"})).toBeInTheDocument();expect(screen.getAllByRole("alert").map(x=>x.textContent).join(" ")).not.toContain("private");});
