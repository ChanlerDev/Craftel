mod indexer;
mod path;
mod repository;
mod slug;
mod task_document;
mod watcher;

pub use indexer::reconcile_project;
pub(crate) use path::eligible;
pub use repository::{
    Document, DocumentCause, DocumentChange, DocumentChanged, DocumentError, DocumentProjectStatus,
    DocumentRepository, DocumentSnapshot, ExpectedDocumentState,
};
pub use slug::slugify;
pub use task_document::{atomic_write_task, initialize_index, initialize_spec};
pub use watcher::ProjectWatcher;
