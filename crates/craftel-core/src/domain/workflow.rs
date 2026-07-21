use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Inbox,
    Defining,
    Implementation,
    Reviewing,
    Done,
}

impl Stage {
    pub const ALL: [Self; 5] = [
        Self::Inbox,
        Self::Defining,
        Self::Implementation,
        Self::Reviewing,
        Self::Done,
    ];

    pub fn next(self, review_approved: bool) -> Result<Self, WorkflowError> {
        match (self, review_approved) {
            (Self::Inbox, _) => Ok(Self::Defining),
            (Self::Defining, _) => Ok(Self::Implementation),
            (Self::Implementation, _) => Ok(Self::Reviewing),
            (Self::Reviewing, true) => Ok(Self::Done),
            _ => Err(WorkflowError::InvalidAction {
                action: "next",
                stage: self,
            }),
        }
    }

    pub fn pass(self) -> Result<Transition, WorkflowError> {
        match self {
            Self::Defining => Ok(Transition::Stay),
            Self::Implementation => Ok(Transition::Move(Self::Reviewing)),
            Self::Reviewing => Ok(Transition::ReviewApproved),
            _ => Err(WorkflowError::InvalidAction {
                action: "pass",
                stage: self,
            }),
        }
    }

    pub fn fail(self) -> Result<Transition, WorkflowError> {
        match self {
            Self::Defining | Self::Implementation => Ok(Transition::Stay),
            Self::Reviewing => Ok(Transition::Move(Self::Implementation)),
            _ => Err(WorkflowError::InvalidAction {
                action: "fail",
                stage: self,
            }),
        }
    }

    pub fn apply(
        self,
        action: WorkflowAction,
        approved: bool,
    ) -> Result<(Self, WorkflowOutcome), WorkflowError> {
        match action {
            WorkflowAction::Move(target) => Ok((
                target,
                if target == self {
                    WorkflowOutcome::Stayed
                } else {
                    WorkflowOutcome::Moved
                },
            )),
            WorkflowAction::Next => Ok((self.next(approved)?, WorkflowOutcome::Moved)),
            WorkflowAction::Pass => match self.pass()? {
                Transition::Move(to) => Ok((to, WorkflowOutcome::Moved)),
                Transition::ReviewApproved => Ok((self, WorkflowOutcome::ReviewApproved)),
                Transition::Stay => Ok((self, WorkflowOutcome::Stayed)),
            },
            WorkflowAction::Fail => match self.fail()? {
                Transition::Move(to) => Ok((to, WorkflowOutcome::PhaseFailed)),
                Transition::Stay => Ok((self, WorkflowOutcome::PhaseFailed)),
                Transition::ReviewApproved => unreachable!(),
            },
        }
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Inbox => "inbox",
            Self::Defining => "defining",
            Self::Implementation => "implementation",
            Self::Reviewing => "reviewing",
            Self::Done => "done",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transition {
    Move(Stage),
    Stay,
    ReviewApproved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowAction {
    Move(Stage),
    Next,
    Pass,
    Fail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowOutcome {
    Moved,
    Stayed,
    ReviewApproved,
    PhaseFailed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub task_id: String,
    pub action: WorkflowAction,
    pub from_stage: Stage,
    pub to_stage: Stage,
    pub outcome: WorkflowOutcome,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum WorkflowError {
    #[error("cannot {action} a task in {stage}")]
    InvalidAction { action: &'static str, stage: Stage },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_transition_examples() {
        assert_eq!(Stage::Inbox.next(false), Ok(Stage::Defining));
        assert_eq!(Stage::Defining.pass(), Ok(Transition::Stay));
        assert_eq!(
            Stage::Implementation.pass(),
            Ok(Transition::Move(Stage::Reviewing))
        );
        assert_eq!(Stage::Reviewing.pass(), Ok(Transition::ReviewApproved));
        assert_eq!(
            Stage::Reviewing.fail(),
            Ok(Transition::Move(Stage::Implementation))
        );
        assert!(Stage::Reviewing.next(false).is_err());
        assert_eq!(Stage::Reviewing.next(true), Ok(Stage::Done));
    }

    #[test]
    fn complete_action_stage_matrix() {
        for stage in Stage::ALL {
            for target in Stage::ALL {
                let result = stage.apply(WorkflowAction::Move(target), false).unwrap();
                assert_eq!(result.0, target);
                assert_eq!(
                    result.1,
                    if target == stage {
                        WorkflowOutcome::Stayed
                    } else {
                        WorkflowOutcome::Moved
                    }
                );
            }
        }
        let valid_next = [Stage::Inbox, Stage::Defining, Stage::Implementation];
        for stage in Stage::ALL {
            assert_eq!(
                stage.apply(WorkflowAction::Next, false).is_ok(),
                valid_next.contains(&stage)
            );
            assert_eq!(
                stage.apply(WorkflowAction::Pass, false).is_ok(),
                matches!(
                    stage,
                    Stage::Defining | Stage::Implementation | Stage::Reviewing
                )
            );
            assert_eq!(
                stage.apply(WorkflowAction::Fail, false).is_ok(),
                matches!(
                    stage,
                    Stage::Defining | Stage::Implementation | Stage::Reviewing
                )
            );
        }
        assert_eq!(
            Stage::Reviewing.apply(WorkflowAction::Next, true),
            Ok((Stage::Done, WorkflowOutcome::Moved))
        );
        assert_eq!(
            Stage::Defining.apply(WorkflowAction::Fail, false),
            Ok((Stage::Defining, WorkflowOutcome::PhaseFailed))
        );
        assert_eq!(
            Stage::Defining.apply(WorkflowAction::Pass, false),
            Ok((Stage::Defining, WorkflowOutcome::Stayed))
        );
        assert_eq!(
            Stage::Implementation.apply(WorkflowAction::Fail, false),
            Ok((Stage::Implementation, WorkflowOutcome::PhaseFailed))
        );
    }

    #[test]
    fn stages_serialize_and_display_lowercase() {
        for stage in Stage::ALL {
            assert_eq!(
                serde_json::to_string(&stage).unwrap(),
                format!("\"{stage}\"")
            );
        }
    }
}
