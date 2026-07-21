use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub work_dir: PathBuf,
    pub available: bool,
    pub created_at: DateTime<Utc>,
    pub last_opened_at: DateTime<Utc>,
}
