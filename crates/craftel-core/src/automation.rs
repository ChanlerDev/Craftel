use crate::runs::Phase;
use std::path::Path;

pub const PROMPT_VERSION: i64 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutomationPrompt {
    pub kind: Phase,
    pub version: i64,
    pub text: String,
}

pub fn build_prompt(phase: Phase, task: &str, project: &str, work_dir: &Path) -> AutomationPrompt {
    let stage = phase.as_str();
    let work = work_dir.display();
    let workflow = match phase {
        Phase::Defining => {
            "Use the installed define-spec workflow. Override its review-turn pause: proceed autonomously from the approved design and context, write the specification, and verify it."
        }
        Phase::Implementation => {
            "Use the installed implement-spec workflow and the current agent-owned documents. Inspect the specification, supporting artifacts, code, and tests; implement and verify the task autonomously."
        }
        Phase::Reviewing => {
            "Perform a fresh evidence-based formal review in this newly created review session. Inspect the specification, supporting artifacts, code, and tests, and write a review record. Do not reuse implementation context and do not implement fixes in this review session."
        }
    };
    let text = format!(
        "CRAFTEL autonomous phase prompt v{PROMPT_VERSION}\n\nTask ID: {task}\nProject ID: {project}\nAbsolute work directory: {work}\nCurrent stage: {stage}\n\n{workflow}\n\nDocument ownership: TASK.md is generated and owned by CRAFTEL; do not edit TASK.md. Put agent-authored work in SPEC.md or supporting decisions/, discussions/, notes/, subtasks/, plans/, and reviews/ directories. Inspect all required artifacts, code, and tests before deciding the outcome.\n\nOn success, run exactly:\ncraftel pass {task} --project {project}\n\nOn failure, run exactly:\ncraftel fail {task} --project {project}\n\nInvoke exactly one of those commands before exiting. Do not infer or merely describe the transition. Do not commit, push, open a pull request, release, deploy, or deliver anything."
    );
    AutomationPrompt {
        kind: phase,
        version: PROMPT_VERSION,
        text,
    }
}

#[cfg(test)]
mod prompt {
    use super::*;
    #[test]
    fn contracts_are_versioned_and_contain_exact_commands() {
        for phase in [Phase::Defining, Phase::Implementation, Phase::Reviewing] {
            let p = build_prompt(phase, "T0001", "project-id", Path::new("/tmp/work"));
            assert_eq!(p.kind, phase);
            assert_eq!(p.version, PROMPT_VERSION);
            assert!(p.text.contains("CRAFTEL autonomous phase prompt v1"));
            assert!(p.text.contains("craftel pass T0001 --project project-id"));
            assert!(p.text.contains("craftel fail T0001 --project project-id"));
            assert!(p.text.contains("do not edit TASK.md"));
            assert!(
                p.text.contains(
                    "Do not commit, push, open a pull request, release, deploy, or deliver"
                )
            );
        }
    }
}
