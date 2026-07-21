use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Stage, WorkflowAction, WorkflowError, WorkflowEvent, WorkflowOutcome};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub content: String,
    pub stage: Stage,
    pub relative_dir: PathBuf,
    pub review_approved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn apply_action(
        &mut self,
        action: WorkflowAction,
        timestamp: DateTime<Utc>,
    ) -> Result<WorkflowEvent, WorkflowError> {
        let from = self.stage;
        let (to, outcome) = self.stage.apply(action, self.review_approved)?;
        self.stage = to;
        self.review_approved = match (action, outcome) {
            (WorkflowAction::Pass, WorkflowOutcome::ReviewApproved) => true,
            (WorkflowAction::Move(_), _) => false,
            _ if from == Stage::Reviewing && to != Stage::Reviewing => false,
            _ => self.review_approved,
        };
        self.updated_at = timestamp;
        Ok(WorkflowEvent {
            task_id: self.id.clone(),
            action,
            from_stage: from,
            to_stage: to,
            outcome,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(stage: Stage, approved: bool) -> Task {
        let now = Utc::now();
        Task {
            id: "T0001".into(),
            project_id: "p".into(),
            title: "t".into(),
            content: "c".into(),
            stage,
            relative_dir: "craftel/tasks/T0001-t".into(),
            review_approved: approved,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn approval_is_scoped_to_one_review_cycle() {
        let mut reviewed = task(Stage::Reviewing, true);
        reviewed
            .apply_action(WorkflowAction::Move(Stage::Reviewing), Utc::now())
            .unwrap();
        assert!(!reviewed.review_approved);
        reviewed.review_approved = true;
        reviewed
            .apply_action(WorkflowAction::Move(Stage::Implementation), Utc::now())
            .unwrap();
        assert!(!reviewed.review_approved);
        reviewed
            .apply_action(WorkflowAction::Move(Stage::Reviewing), Utc::now())
            .unwrap();
        assert!(!reviewed.review_approved);
    }
}
