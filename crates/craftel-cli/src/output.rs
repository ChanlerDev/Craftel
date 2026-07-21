pub fn json<T: serde::Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}
pub fn human_task(value: &craftel_core::domain::Task) -> String {
    format!("{}\t{}\t{}", value.id, value.stage, value.title)
}
pub fn human_project(value: &craftel_core::domain::Project) -> String {
    format!(
        "{}\t{}\t{}\t{}",
        value.id,
        value.name,
        value.work_dir.display(),
        if value.available {
            "available"
        } else {
            "missing"
        }
    )
}
