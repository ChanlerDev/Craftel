import { useEffect, useState } from "react";
import type { CraftelApi } from "../api/craftel";
import { errorMessage } from "../api/craftel";
import type { GitWorkingCopySummary } from "../api/types";

function Diff({text}:{text:string}) {
  if(!text)return <div className="git-empty">No changes in this section.</div>;
  return <pre className="git-diff" aria-label="Git diff">{text.split("\n").map((line,index)=><span key={index} className={line.startsWith("+")&&!line.startsWith("+++")?"added":line.startsWith("-")&&!line.startsWith("---")?"removed":line.startsWith("@@")?"hunk":undefined}>{line}{"\n"}</span>)}</pre>;
}

export function GitChanges({api,projectId,refreshToken}:{api:CraftelApi;projectId:string;refreshToken:string}) {
  const [summary,setSummary]=useState<GitWorkingCopySummary|null>(null),[tab,setTab]=useState<"unstaged"|"staged">("unstaged"),[error,setError]=useState(""),[loading,setLoading]=useState(true);
  const load=async()=>{setLoading(true);try{setSummary(await api.gitWorkingCopySummary(projectId));setError("")}catch(e){setError(errorMessage(e))}finally{setLoading(false)}};
  useEffect(()=>{void load()},[projectId,refreshToken]);
  return <section className="git-changes" aria-labelledby="git-changes-title"><header><div><p className="eyebrow">DELIVERY EVIDENCE</p><h2 id="git-changes-title">Code changes</h2></div><button onClick={()=>void load()}>Refresh</button></header>{loading&&!summary?<p role="status">Loading working copy…</p>:error?<p role="alert">{error}</p>:summary&&!summary.is_repository?<div className="git-empty">This project directory is not a Git working tree.</div>:summary&&<><div className="git-meta"><span>Branch <strong>{summary.branch??"detached HEAD"}</strong></span>{summary.latest_commit&&<span>Latest commit <strong title={summary.latest_commit.hash}>{summary.latest_commit.hash.slice(0,8)}</strong> {summary.latest_commit.subject}</span>}</div>{summary.truncated&&<div className="notice" role="status">Diff output was truncated. Inspect the repository before delivery.</div>}<div className="git-tabs" role="tablist" aria-label="Code change type"><button role="tab" aria-selected={tab==="unstaged"} onClick={()=>setTab("unstaged")}>Unstaged</button><button role="tab" aria-selected={tab==="staged"} onClick={()=>setTab("staged")}>Staged</button></div><Diff text={tab==="unstaged"?summary.unstaged_diff:summary.staged_diff}/>{summary.untracked_paths.length>0&&<div className="git-untracked"><strong>Untracked</strong><ul>{summary.untracked_paths.map(path=><li key={path}>{path}</li>)}</ul></div>}</>}</section>;
}
