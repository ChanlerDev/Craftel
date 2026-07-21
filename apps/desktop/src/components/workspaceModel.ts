import type { Task } from "../api/types";

export type WorkspaceAction = "advance" | "start" | "stop" | "follow";
export interface WorkspaceModel {
  status: string;
  description: string;
  artifact: string;
  primary: { kind: WorkspaceAction; label: string } | null;
  composer: { label: string; placeholder: string; actionLabel: string } | null;
  tone: "neutral" | "active" | "warning" | "success";
}

export function workspaceModel(task: Task, state: { active: boolean; reviewReturned: boolean; resumable: boolean }): WorkspaceModel {
  if (state.active) return {
    status: `${task.stage === "reviewing" ? "Review" : task.stage === "defining" ? "Defining" : "Implementation"} in progress`,
    description: "The agent is running. Events and tool activity stream in the phase panel; Stop is the only available intervention.",
    artifact: task.stage === "defining" ? "SPEC.md" : task.stage === "reviewing" ? "Review packet" : "Implementation evidence",
    primary: { kind: "stop", label: "Stop run" }, composer: null, tone: "active",
  };
  if (task.stage === "inbox") return {
    status: "Ready to define", description: "Review the task brief, then move this task into Defining. Moving stages never starts an agent.", artifact: "Task brief",
    primary: { kind: "advance", label: "Move to Defining" }, composer: null, tone: "neutral",
  };
  if (task.stage === "defining") return {
    status: state.resumable ? "Review the contract" : "Define the contract", description: state.resumable ? "Inspect SPEC.md. Move forward when it is an approved, testable contract, or continue the same conversation to refine it." : "Start a defining session. The agent will turn the task context into a durable SPEC.md contract.", artifact: "SPEC.md",
    primary: { kind: state.resumable ? "advance" : "start", label: state.resumable ? "Move to Implementation" : "Start Defining" },
    composer: state.resumable ? { label: "Message the defining agent", placeholder: "Clarify the requirement or ask for another SPEC.md revision", actionLabel: "Continue Defining" } : null, tone: "neutral",
  };
  if (task.stage === "implementation") return {
    status: state.reviewReturned ? "Changes requested" : "Ready to implement",
    description: state.reviewReturned ? "A formal review returned findings. Continue the implementation session, address the review document, then request a fresh review." : "Start implementation from the approved specification. Moving into this stage did not start an agent.",
    artifact: state.reviewReturned ? "Review findings + implementation plan" : "SPEC.md + implementation plan",
    primary: { kind: state.resumable ? "follow" : "start", label: state.reviewReturned ? "Continue Implementation" : state.resumable ? "Continue Implementation" : "Start Implementation" },
    composer: state.resumable ? { label: "Message the implementation agent", placeholder: "Describe the fix, constraint, or continuation", actionLabel: "Continue Implementation" } : null, tone: state.reviewReturned ? "warning" : "neutral",
  };
  if (task.stage === "reviewing" && task.review_approved) return {
    status: "Approved · awaiting human", description: "Automated review passed. Inspect the review packet and delivery evidence; only a human can mark this task Done.", artifact: "Latest review packet",
    primary: { kind: "advance", label: "Mark Done" }, composer: null, tone: "success",
  };
  if (task.stage === "reviewing") return {
    status: "Ready for fresh review", description: "Start a formal review in a new clean session. Approval remains here until human delivery; changes requested return to Implementation.", artifact: "New review packet",
    primary: { kind: "start", label: "Start fresh Review" }, composer: null, tone: "neutral",
  };
  return { status: "Delivered", description: "This task completed automated review and the final human handoff.", artifact: "Task documents and review evidence", primary: null, composer: null, tone: "success" };
}
