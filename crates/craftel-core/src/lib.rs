pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod app_paths;
pub mod automation;
pub mod documents;
pub mod domain;
pub mod git_summary;
pub mod harness;
pub mod run_service;
pub mod runs;
pub mod service;
pub mod storage;

pub use service::{CraftelService, ServiceError};
