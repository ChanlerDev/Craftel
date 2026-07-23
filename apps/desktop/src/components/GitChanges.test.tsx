import {render,screen} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {vi} from "vitest";
import {fakeApi} from "../test/fake";
import {GitChanges} from "./GitChanges";

test("shows branch commit unstaged staged and untracked delivery evidence",async()=>{
 const api=fakeApi({gitWorkingCopySummary:vi.fn().mockResolvedValue({is_repository:true,branch:"feature/task",latest_commit:{hash:"1234567890abcdef",subject:"Implement task",committed_at:"2026-07-21T00:00:00Z"},unstaged_diff:"@@ -1 +1 @@\n-old\n+new\n",staged_diff:"@@ -0 +1 @@\n+staged\n",untracked_paths:["new.md"],truncated:false})});render(<GitChanges api={api} projectId="p1" refreshToken="one"/>);expect(await screen.findByText("feature/task")).toBeInTheDocument();expect(screen.getByText("12345678")).toBeInTheDocument();expect(screen.getByLabelText("Git diff")).toHaveTextContent("+new");expect(screen.getByText("new.md")).toBeInTheDocument();await userEvent.click(screen.getByRole("tab",{name:"Staged"}));expect(screen.getByLabelText("Git diff")).toHaveTextContent("+staged");
});
