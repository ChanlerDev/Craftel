import { describe, expect, it } from "vitest";
import { task } from "../test/fake";
import { workspaceModel } from "./workspaceModel";

describe("stage-aware workspace model", () => {
  it.each([
    [task("inbox"), false, false, false, "Ready to define", "Move to Defining"],
    [task("defining"), false, false, false, "Define the contract", "Start Defining"],
    [task("defining"), true, false, false, "Defining in progress", "Stop run"],
    [task("implementation"), false, false, false, "Ready to implement", "Start Implementation"],
    [task("implementation"), false, true, false, "Changes requested", "Continue Implementation"],
    [task("reviewing"), false, false, false, "Ready for fresh review", "Start fresh Review"],
    [task("reviewing", { review_approved: true }), false, false, false, "Approved · awaiting human", "Mark Done"],
    [task("done"), false, false, false, "Delivered", null],
  ])("maps a valid task state to one primary action", (value, active, returned, resumable, status, action) => {
    const model = workspaceModel(value, { active, reviewReturned: returned, resumable });
    expect(model.status).toBe(status);
    expect(model.primary?.label ?? null).toBe(action);
  });

  it("offers conversation continuation without presenting it as a new phase", () => {
    const model = workspaceModel(task("defining"), { active: false, reviewReturned: false, resumable: true });
    expect(model.composer).toEqual({ label: "Message the defining agent", placeholder: "Clarify the requirement or ask for another SPEC.md revision", actionLabel: "Continue Defining" });
    expect(model.primary?.label).toBe("Move to Implementation");
  });
});
