pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod app_paths;
pub mod documents;
pub mod domain;
pub mod service;
pub mod storage;

pub use service::{CraftelService, ServiceError};
