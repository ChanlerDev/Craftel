# CRAFTEL Agent Guidance

- `TASK.md` files are generated projections owned by CRAFTEL. Never edit them directly; update fixed metadata through CRAFTEL.
- Put agent-authored details in `SPEC.md` or supporting `decisions/`, `discussions/`, `notes/`, `subtasks/`, `plans/`, and `reviews/` directories.
- Use `craftel next`, `craftel pass`, or `craftel fail` for normal workflow changes. Use explicit `craftel move` only for manual board correction.
- Moving a task changes its stage only and never starts an agent or external process.
- No Git push, pull request, release, delivery, or deployment action is implicit in a workflow operation. Perform such actions only when explicitly requested.
