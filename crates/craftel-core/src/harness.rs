use crate::runs::EventKind;
use serde_json::Value;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

pub const STDERR_LIMIT: usize = 1024 * 1024;
pub const STDERR_TRUNCATED: &str = "\n[CRAFTEL: stderr truncated at 1 MiB]\n";

#[derive(Clone, Debug)]
pub struct ParsedEvent {
    pub kind: EventKind,
    pub display_text: Option<String>,
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
    pub raw_json: String,
    pub external_session_id: Option<String>,
    pub request_id: Option<String>,
    pub model: Option<String>,
    pub final_result: Option<String>,
}
#[derive(Default)]
pub struct NdjsonParser {
    pending: Vec<u8>,
}
impl NdjsonParser {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<ParsedEvent> {
        self.pending.extend_from_slice(bytes);
        let mut out = Vec::new();
        while let Some(i) = self.pending.iter().position(|b| *b == b'\n') {
            let line = self.pending.drain(..=i).collect::<Vec<_>>();
            let line = &line[..line.len() - 1];
            out.push(parse_line(line))
        }
        out
    }
    pub fn finish(&mut self) -> Option<ParsedEvent> {
        // A stream-json record is complete only at a newline. Cursor can be
        // terminated in the middle of a JSON value; never turn that tail into
        // a durable event payload.
        self.pending.clear();
        None
    }
}
fn parse_line(line: &[u8]) -> ParsedEvent {
    let raw = String::from_utf8_lossy(line).into_owned();
    let Ok(v) = serde_json::from_slice::<Value>(line) else {
        return ParsedEvent {
            kind: EventKind::Unknown,
            display_text: Some(raw.clone()),
            tool_name: None,
            tool_call_id: None,
            raw_json: raw,
            external_session_id: None,
            request_id: None,
            model: None,
            final_result: None,
        };
    };
    let typ = v.get("type").and_then(Value::as_str).unwrap_or("");
    let subtype = v.get("subtype").and_then(Value::as_str).unwrap_or("");
    let kind = match (typ, subtype) {
        ("user", _) => EventKind::User,
        ("assistant", _) => EventKind::Assistant,
        ("tool_call", "started") | ("tool_start", _) => EventKind::ToolStart,
        ("tool_call", "completed") | ("tool_result", _) => EventKind::ToolComplete,
        ("result", _) => EventKind::Result,
        ("system", _) => EventKind::System,
        _ => EventKind::Unknown,
    };
    let text = v
        .get("text")
        .or_else(|| v.get("message").and_then(|m| m.get("content")))
        .or_else(|| v.get("result"))
        .and_then(|x| {
            if let Some(s) = x.as_str() {
                Some(s.to_owned())
            } else {
                x.as_array().map(|a| {
                    a.iter()
                        .filter_map(|x| x.get("text").and_then(Value::as_str))
                        .collect::<String>()
                })
            }
        });
    ParsedEvent {
        kind,
        display_text: text.clone(),
        tool_name: v
            .pointer("/tool_call/name")
            .or_else(|| v.get("tool_name"))
            .or_else(|| v.get("name"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        tool_call_id: v
            .get("call_id")
            .or_else(|| v.pointer("/tool_call/call_id"))
            .or_else(|| v.pointer("/tool_call/id"))
            .or_else(|| v.get("tool_call_id"))
            .or_else(|| v.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        raw_json: raw,
        external_session_id: v
            .get("session_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        request_id: v
            .get("request_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        model: (kind == EventKind::System)
            .then(|| v.get("model").and_then(Value::as_str).map(str::to_owned))
            .flatten(),
        final_result: (kind == EventKind::Result).then_some(text).flatten(),
    }
}

#[derive(Clone)]
pub struct CursorHarness {
    executable: PathBuf,
}
impl CursorHarness {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }
    pub fn executable(&self) -> &Path {
        &self.executable
    }
    pub fn version(&self) -> io::Result<String> {
        let mut child = Command::new(&self.executable)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Cursor version discovery timed out",
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        if !status.success() {
            return Err(io::Error::other("Cursor version discovery failed"));
        }
        let mut stdout = Vec::new();
        child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("missing version output"))?
            .read_to_end(&mut stdout)?;
        let version = String::from_utf8_lossy(&stdout).trim().to_owned();
        if version.is_empty() {
            return Err(io::Error::other("Cursor returned an empty version"));
        }
        Ok(version)
    }
    pub fn argv(prompt: &str, resume: Option<&str>) -> Vec<String> {
        let mut a = vec!["-p".into(), "--force".into()];
        if let Some(id) = resume {
            a.push(format!("--resume={id}"))
        }
        a.extend([
            "--output-format".into(),
            "stream-json".into(),
            "--stream-partial-output".into(),
            prompt.into(),
        ]);
        a
    }
    pub fn spawn(
        &self,
        prompt: &str,
        resume: Option<&str>,
        cwd: &Path,
        ownership_token: &str,
    ) -> io::Result<Child> {
        let mut command = Command::new(&self.executable);
        command
            .args(Self::argv(prompt, resume))
            .current_dir(cwd)
            .env("CRAFTEL_OWNERSHIP_TOKEN", ownership_token)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        unsafe {
            command.pre_exec(|| {
                if libc::setpgid(0, 0) == 0 {
                    Ok(())
                } else {
                    Err(io::Error::last_os_error())
                }
            });
        }
        command.spawn()
    }
}
pub fn append_bounded(target: &mut Vec<u8>, bytes: &[u8]) {
    if target.len() >= STDERR_LIMIT {
        return;
    }
    let content_limit = STDERR_LIMIT.saturating_sub(STDERR_TRUNCATED.len());
    let room = content_limit.saturating_sub(target.len());
    target.extend_from_slice(&bytes[..bytes.len().min(room)]);
    if bytes.len() > room && !target.ends_with(STDERR_TRUNCATED.as_bytes()) {
        target.extend_from_slice(STDERR_TRUNCATED.as_bytes())
    }
}
