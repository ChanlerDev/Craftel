import { describe,expect,it } from "vitest";
import type { Run,RunEvent } from "../api/types";
import { duration,mergeEvents } from "./RunActivity";

const event=(sequence:number,text=String(sequence)):RunEvent=>({run_id:"r1",sequence,kind:"assistant",event_at:"2026-01-01T00:00:00Z",display_text:text,tool_name:null,tool_call_id:null,raw_json:"{}"});
describe("durable run event reducer",()=>{
 it("merges duplicate and out-of-order pages by run and sequence",()=>expect(mergeEvents([event(2),event(1)],[event(2,"new"),event(3)]).map(e=>[e.sequence,e.display_text])).toEqual([[1,"1"],[2,"new"],[3,"3"]]));
 it("does not keep ticking terminal durations",()=>{const run={state:"succeeded",created_at:"2026-01-01T00:00:00Z",updated_at:"2026-01-01T00:00:09Z",started_at:"2026-01-01T00:00:02Z",finished_at:"2026-01-01T00:00:07Z"} as Run;expect(duration(run,Date.parse("2027-01-01"))).toBe(5)});
});
