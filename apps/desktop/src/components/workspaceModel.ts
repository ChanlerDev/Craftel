import type { Task } from "../api/types";

export type WorkspaceAction = "advance" | "start" | "stop" | "follow" | "request_changes";
export interface WorkspaceModel {
  status: string;
  description: string;
  artifact: string;
  primary: { kind: WorkspaceAction; label: string } | null;
  secondary: { kind: WorkspaceAction; label: string }[];
  composer: { label: string; placeholder: string; actionLabel: string } | null;
  tone: "neutral" | "active" | "warning" | "success";
}

export function workspaceModel(task: Task, state: { active: boolean; reviewReturned: boolean; resumable: boolean }): WorkspaceModel {
  if (state.active) return {
    status: `${task.stage === "reviewing" ? "Review" : task.stage === "defining" ? "Defining" : "Implementation"} in progress`,
    description: "The agent is running. Events and tool activity stream in the phase panel; Stop is the only available intervention.",
    artifact: task.stage === "defining" ? "SPEC.md" : task.stage === "reviewing" ? "Review packet" : "Implementation evidence",
    primary: { kind: "stop", label: "Stop run" }, secondary: [], composer: null, tone: "active",
  };
  if (task.stage === "inbox") return {
    status: "Ready to define", description: "Review the task brief, then move this task into Defining. Moving stages never starts an agent.", artifact: "Task brief",
    primary: { kind: "advance", label: "Move to Defining" }, secondary: [], composer: null, tone: "neutral",
  };
  if (task.stage === "defining") return {
    status: state.resumable ? "Review the contract" : "Define the contract", description: state.resumable ? "Inspect SPEC.md. Move forward when it is an approved, testable contract, or continue the same conversation to refine it." : "Start a defining session. The agent will turn the task context into a durable SPEC.md contract.", artifact: "SPEC.md",
    primary: { kind: state.resumable ? "advance" : "start", label: state.resumable ? "Move to Implementation" : "Start Defining" },
    secondary: [],
    composer: state.resumable ? { label: "Message the defining agent", placeholder: "Clarify the requirement or ask for another SPEC.md revision", actionLabel: "Continue Defining" } : null, tone: "neutral",
  };
  if (task.stage === "implementation") return {
    status: state.reviewReturned ? "Changes requested" : "Ready to implement",
    description: state.reviewReturned ? "A formal review returned findings. Continue the implementation session, address the review document, then request a fresh review." : "Start implementation from the approved specification. Moving into this stage did not start an agent.",
    artifact: state.reviewReturned ? "Review findings + implementation plan" : "SPEC.md + implementation plan",
    primary: { kind: state.resumable ? "follow" : "start", label: state.reviewReturned ? "Continue Implementation" : state.resumable ? "Continue Implementation" : "Start Implementation" },
    secondary: [],
    composer: state.resumable ? { label: "Message the implementation agent", placeholder: "Describe the fix, constraint, or continuation", actionLabel: "Continue Implementation" } : null, tone: state.reviewReturned ? "warning" : "neutral",
  };
  if (task.stage === "reviewing" && task.review_approved) return {
    status: "Approved · awaiting human", description: "Automated review passed. Inspect the review packet and delivery evidence; only a human can mark this task Done.", artifact: "Latest review packet",
    primary: { kind: "advance", label: "Mark Done" }, secondary: [{kind:"request_changes",label:"Request changes"}], composer: null, tone: "success",
  };
  if (task.stage === "reviewing") return {
    status: "Ready for human review", description: "Inspect the code changes and delivery evidence. Complete the task, request another implementation pass, or optionally run a formal review in a fresh session.", artifact: "Code changes + task documents",
    primary: { kind: "advance", label: "Mark Done" }, secondary: [{kind:"request_changes",label:"Request changes"},{kind:"start",label:"Run formal review"}], composer: null, tone: "neutral",
  };
  return { status: "Delivered", description: "This task completed the final human handoff.", artifact: "Task documents and review evidence", primary: null, secondary: [], composer: null, tone: "success" };
}
