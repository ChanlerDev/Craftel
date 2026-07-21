use crate::domain::Task;
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};
use uuid::Uuid;

#[derive(Serialize)]
struct FrontMatter<'a> {
    id: &'a str,
    title: &'a str,
    status: crate::domain::Stage,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub fn atomic_write_task(path: &Path, task: &Task) -> std::io::Result<()> {
    let yaml = serde_yaml::to_string(&FrontMatter {
        id: &task.id,
        title: &task.title,
        status: task.stage,
        created_at: task.created_at,
        updated_at: task.updated_at,
    })
    .map_err(std::io::Error::other)?;
    let body = format!(
        "---\n{yaml}---\n\n# {}\n\n## Content\n\n{}\n\n## Artifacts\n\n- Specification: [SPEC.md](./SPEC.md)\n- Latest plan: Not created\n- Latest review: Not created\n\n> This file is managed by CRAFTEL. Use the `craftel` CLI to update task\n> metadata and workflow state. Put agent-authored details in `SPEC.md` and\n> supporting document directories.\n",
        task.title, task.content
    );
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("TASK.md has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".TASK.md.{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(body.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn initialize_spec(path: &Path) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(b"# Specification\n\nTask metadata: [TASK.md](./TASK.md)\n")?;
    file.sync_all()
}

pub fn initialize_index(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    fs::create_dir_all(
        path.parent()
            .ok_or_else(|| std::io::Error::other("INDEX has no parent"))?,
    )?;
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(b"# CRAFTEL Tasks\n")?;
            file.sync_all()
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}
