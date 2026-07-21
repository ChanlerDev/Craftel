use std::path::{Component, Path};
pub(crate) fn eligible(path: &Path) -> bool {
    let file = path
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or_default();
    if path.components().any(|c| matches!(c, Component::ParentDir))
        || path.extension().and_then(|x| x.to_str()) != Some("md")
        || file.ends_with(".tmp.md")
        || file.ends_with(".temp.md")
        || file.starts_with('~')
        || file.ends_with('~')
        || path
            .components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
    {
        return false;
    }
    let p: Vec<_> = path
        .components()
        .filter_map(|c| {
            if let Component::Normal(v) = c {
                v.to_str()
            } else {
                None
            }
        })
        .collect();
    match p.as_slice() {
        ["craftel", "INDEX.md"] => true,
        ["craftel", "tasks", t, "SPEC.md"] => t.starts_with('T'),
        ["craftel", "tasks", t, d, r @ ..] => {
            t.starts_with('T')
                && [
                    "decisions",
                    "discussions",
                    "notes",
                    "subtasks",
                    "plans",
                    "reviews",
                ]
                .contains(d)
                && !r.is_empty()
        }
        _ => false,
    }
}
pub(crate) fn task_id(path: &Path) -> Option<String> {
    path.components()
        .nth(2)?
        .as_os_str()
        .to_str()?
        .split('-')
        .next()
        .filter(|x| x.starts_with('T'))
        .map(str::to_string)
}
