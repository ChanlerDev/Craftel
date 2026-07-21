import {act,render,screen,waitFor} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {vi} from "vitest";
import {RunActivity} from "./RunActivity";
import {deferred,fakeApi,run,runEvent,session} from "../test/fake";

test("exhausts event pages and renders paired tools, unknown events, and assistant output",async()=>{
 const first=Array.from({length:100},(_,i)=>runEvent(i+1));const list=vi.fn().mockResolvedValueOnce(first).mockResolvedValueOnce([
  runEvent(101,{kind:"tool_complete",tool_call_id:"call",display_text:"result"}),runEvent(102,{kind:"tool_start",tool_call_id:"call",tool_name:"Shell",display_text:"input"}),runEvent(103,{kind:"unknown",display_text:"mystery"})]).mockResolvedValue([]);
 render(<RunActivity api={fakeApi({listSessions:vi.fn().mockResolvedValue([session()]),listRuns:vi.fn().mockResolvedValue([run()]),listRunEvents:list})} projectId="p1" taskId="T0001"/>);
 expect(await screen.findByText("event 1")).toBeInTheDocument();expect(await screen.findByText("Shell · completed")).toBeInTheDocument();expect(screen.getByText("Unknown event")).toBeInTheDocument();expect(list).toHaveBeenCalledWith("r1",100,100);
});

test("fetches, subscribes, then catches up and inserts window events",async()=>{
 const order:string[]=[];const api=fakeApi({listSessions:vi.fn(async()=>{order.push("fetch");return[session()]}),listRuns:vi.fn().mockResolvedValue([run()]),listRunEvents:vi.fn().mockResolvedValue([runEvent(1,{display_text:"caught up"})]),subscribe:vi.fn(async()=>{order.push("subscribe");return()=>{}})});
 render(<RunActivity api={api} projectId="p1" taskId="T0001"/>);expect(await screen.findByText("caught up")).toBeInTheDocument();expect(order.indexOf("subscribe")).toBeGreaterThan(order.indexOf("fetch"));
});

test("coalesces duplicate hints, retries a sequence fetch failure, and does not duplicate events",async()=>{
 let hints:((p:unknown)=>void)[]=[];const page=deferred<ReturnType<typeof runEvent>[]>();const list=vi.fn().mockResolvedValueOnce([]).mockResolvedValueOnce([]).mockReturnValueOnce(page.promise).mockRejectedValueOnce(new Error("gap unavailable")).mockResolvedValue([runEvent(1)]);
 render(<RunActivity api={fakeApi({listSessions:vi.fn().mockResolvedValue([session()]),listRuns:vi.fn().mockResolvedValue([run()]),listRunEvents:list,subscribe:vi.fn(async(_,h)=>{hints.push(h);return()=>{}})})} projectId="p1" taskId="T0001"/>);
 await screen.findByText("Run 1");act(()=>{hints.forEach(h=>{h({});h({})})});page.resolve([runEvent(1)]);
 await waitFor(()=>expect(screen.getAllByText("event 1")).toHaveLength(1));act(()=>hints[0]({}));await waitFor(()=>expect(list.mock.calls.length).toBeGreaterThanOrEqual(4));
});

test("late subscriptions unlisten immediately and mounted listeners clean up",async()=>{
 const late=deferred<()=>void>(),off=vi.fn();const subscribe=vi.fn().mockReturnValueOnce(late.promise).mockResolvedValue(off);const {unmount}=render(<RunActivity api={fakeApi({subscribe})} projectId="p1" taskId="T0001"/>);await waitFor(()=>expect(subscribe).toHaveBeenCalled());unmount();late.resolve(off);await waitFor(()=>expect(off).toHaveBeenCalled());
});

test("shows restart history, reverse completion cannot move selection off newest, and terminal notices",async()=>{
 const old=run("interrupted",{id:"old",sequence:1,missing_transition:true,error:"x".repeat(600)}),newest=run("succeeded",{id:"new",sequence:2});
 render(<RunActivity api={fakeApi({listSessions:vi.fn().mockResolvedValue([session()]),listRuns:vi.fn().mockResolvedValue([old,newest])})} projectId="p1" taskId="T0001"/>);
 expect(await screen.findByRole("heading",{name:"Run 2"})).toBeInTheDocument();await userEvent.click(screen.getByRole("button",{name:/Run 1/}));expect(screen.getByText(/without a workflow transition/)).toBeInTheDocument();expect(screen.getByText(/not resumed automatically/)).toBeInTheDocument();expect(screen.getByRole("alert").textContent!.length).toBeLessThanOrEqual(500);
});

test("keeps only the latest formal review current and folds older reviews into history",async()=>{
 const oldSession=session({id:"review-1",phase:"reviewing",updated_at:"2026-07-21T11:00:00Z"}),newSession=session({id:"review-2",phase:"reviewing",updated_at:"2026-07-21T13:00:00Z"}),oldRun=run("succeeded",{id:"old-review",session_id:oldSession.id}),newRun=run("succeeded",{id:"new-review",session_id:newSession.id,sequence:2});
 render(<RunActivity api={fakeApi({listSessions:vi.fn().mockResolvedValue([oldSession,newSession]),listRuns:vi.fn(async id=>id===newSession.id?[newRun]:[oldRun])})} projectId="p1" taskId="T0001" phase="reviewing"/>);expect(await screen.findByRole("heading",{name:"Run 2"})).toBeInTheDocument();expect(screen.getByText("Earlier phase history · 1")).toBeInTheDocument();
});

test("only running durations tick",async()=>{
 vi.useFakeTimers();vi.setSystemTime(new Date("2026-07-21T12:00:05Z"));const active=run("running",{started_at:"2026-07-21T12:00:00Z",created_at:"2026-07-21T12:00:00Z"});
 render(<RunActivity api={fakeApi({listSessions:vi.fn().mockResolvedValue([session()]),listRuns:vi.fn().mockResolvedValue([active])})} projectId="p1" taskId="T0001"/>);await act(async()=>{await vi.runAllTicks()});expect(screen.getByText("5s")).toBeInTheDocument();act(()=>vi.advanceTimersByTime(2000));expect(screen.getByText("7s")).toBeInTheDocument();vi.useRealTimers();
});
