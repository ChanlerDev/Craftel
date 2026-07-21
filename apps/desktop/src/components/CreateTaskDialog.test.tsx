import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { CreateTaskDialog } from "./CreateTaskDialog";
import { fakeApi, task } from "../test/fake";
test("requires non-blank title and content then creates and refreshes", async () => { const api = fakeApi({ createTask: vi.fn().mockResolvedValue(task()) }); const saved = vi.fn(); render(<CreateTaskDialog api={api} projectId="p1" onClose={vi.fn()} onSaved={saved} />); await userEvent.click(screen.getByRole("button", { name: "Create task" })); expect(screen.getByRole("alert")).toHaveTextContent("required"); await userEvent.type(screen.getByLabelText("Title"), " New task "); await userEvent.type(screen.getByLabelText("Content"), " Details "); await userEvent.click(screen.getByRole("button", { name: "Create task" })); expect(api.createTask).toHaveBeenCalledWith("p1", "New task", "Details"); expect(saved).toHaveBeenCalledOnce(); });
