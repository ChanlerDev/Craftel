use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Maximum bytes retained from each Git diff or untracked-path invocation.
pub const GIT_OUTPUT_LIMIT_BYTES: usize = 512 * 1024;
const GIT_METADATA_LIMIT_BYTES: usize = 64 * 1024;
const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitCommitSummary {
    pub hash: String,
    pub subject: String,
    pub committed_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitWorkingCopySummary {
    pub is_repository: bool,
    pub branch: Option<String>,
    pub latest_commit: Option<GitCommitSummary>,
    pub staged_diff: String,
    pub unstaged_diff: String,
    pub untracked_paths: Vec<String>,
    /// True when any diff or the untracked path stream exceeded the per-command limit.
    pub truncated: bool,
}

impl GitWorkingCopySummary {
    fn non_repository() -> Self {
        Self {
            is_repository: false,
            branch: None,
            latest_commit: None,
            staged_diff: String::new(),
            unstaged_diff: String::new(),
            untracked_paths: Vec::new(),
            truncated: false,
        }
    }
}

pub fn working_copy_summary(work_dir: &Path) -> io::Result<GitWorkingCopySummary> {
    let probe = git_output(
        work_dir,
        &["rev-parse", "--is-inside-work-tree"],
        GIT_METADATA_LIMIT_BYTES,
    )?;
    if !probe.status.success() || probe.stdout != b"true\n" {
        return Ok(GitWorkingCopySummary::non_repository());
    }

    let branch = git_text(work_dir, &["branch", "--show-current"])?;
    let commit = git_optional_text(work_dir, &["log", "-1", "--format=%H%x00%s%x00%cI"])?;
    let latest_commit = if commit.is_empty() {
        None
    } else {
        let mut fields = commit.splitn(3, '\0');
        Some(GitCommitSummary {
            hash: fields.next().unwrap_or_default().to_owned(),
            subject: fields.next().unwrap_or_default().to_owned(),
            committed_at: fields.next().unwrap_or_default().to_owned(),
        })
    };
    let (staged_diff, staged_truncated) = git_bounded(
        work_dir,
        &["diff", "--cached", "--no-ext-diff", "--no-textconv"],
    )?;
    let (unstaged_diff, unstaged_truncated) =
        git_bounded(work_dir, &["diff", "--no-ext-diff", "--no-textconv"])?;
    let (untracked, untracked_truncated) = git_bounded(
        work_dir,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?;
    let untracked_paths = untracked
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect();
    Ok(GitWorkingCopySummary {
        is_repository: true,
        branch: (!branch.is_empty()).then_some(branch),
        latest_commit,
        staged_diff,
        unstaged_diff,
        untracked_paths,
        truncated: staged_truncated || unstaged_truncated || untracked_truncated,
    })
}

fn git_text(work_dir: &Path, args: &[&str]) -> io::Result<String> {
    let output = git_output(work_dir, args, GIT_METADATA_LIMIT_BYTES)?;
    if output.truncated || !output.status.success() {
        return Err(io::Error::other("git metadata command failed"));
    }
    String::from_utf8(output.stdout)
        .map(|text| text.trim_end_matches('\n').to_owned())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "git returned non-UTF-8 output"))
}

fn git_optional_text(work_dir: &Path, args: &[&str]) -> io::Result<String> {
    let output = git_output(work_dir, args, GIT_METADATA_LIMIT_BYTES)?;
    if output.truncated {
        return Err(io::Error::other("git metadata output exceeded the limit"));
    }
    if !output.status.success() {
        return Ok(String::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end_matches('\n')
        .to_owned())
}

fn git_bounded(work_dir: &Path, args: &[&str]) -> io::Result<(String, bool)> {
    let output = git_output(work_dir, args, GIT_OUTPUT_LIMIT_BYTES)?;
    if !output.truncated && !output.status.success() {
        return Err(io::Error::other("git working-copy command failed"));
    }
    Ok((
        String::from_utf8_lossy(&output.stdout).into_owned(),
        output.truncated,
    ))
}

#[derive(Debug)]
struct BoundedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    truncated: bool,
}

fn git_output(work_dir: &Path, args: &[&str], limit: usize) -> io::Result<BoundedOutput> {
    let mut command = Command::new("git");
    command.args(args).current_dir(work_dir);
    command_output(command, limit, GIT_COMMAND_TIMEOUT)
}

fn command_output(
    mut command: Command,
    limit: usize,
    timeout: Duration,
) -> io::Result<BoundedOutput> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let (sender, receiver) = mpsc::sync_channel(1);
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout
            .by_ref()
            .take((limit + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sender.send(result);
    });
    let mut running = RunningChild::new(child, reader);
    let deadline = Instant::now() + timeout;
    let mut output = None;
    let mut status = None;
    loop {
        if output.is_none() {
            match receiver.try_recv() {
                Ok(Ok(bytes)) => output = Some(bytes),
                Ok(Err(error)) => return Err(error),
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(io::Error::other("git output reader failed"));
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if status.is_none() {
            status = running.child.try_wait()?;
        }
        if output.as_ref().is_some_and(|bytes| bytes.len() > limit) {
            let mut stdout = output.take().expect("checked output");
            stdout.truncate(limit);
            let status = running.terminate()?;
            return Ok(BoundedOutput {
                status,
                stdout,
                truncated: true,
            });
        }
        if let Some(status) = status
            && let Some(mut stdout) = output.take()
        {
            let truncated = stdout.len() > limit;
            stdout.truncate(limit);
            return Ok(BoundedOutput {
                status,
                stdout,
                truncated,
            });
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "git command timed out",
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

struct RunningChild {
    child: Child,
    reader: Option<JoinHandle<()>>,
}

impl RunningChild {
    fn new(child: Child, reader: JoinHandle<()>) -> Self {
        Self {
            child,
            reader: Some(reader),
        }
    }

    fn terminate(&mut self) -> io::Result<ExitStatus> {
        match self.child.try_wait() {
            Ok(Some(status)) => Ok(status),
            Ok(None) => {
                let kill_error = self.child.kill().err();
                match self.child.wait() {
                    Ok(status) => Ok(status),
                    Err(error) => Err(kill_error.unwrap_or(error)),
                }
            }
            Err(probe_error) => {
                let _ = self.child.kill();
                let _ = self.child.wait();
                Err(probe_error)
            }
        }
    }
}

impl Drop for RunningChild {
    fn drop(&mut self) {
        let _ = self.terminate();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_runner_truncates_and_reaps() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf 123456"]);
        let output = command_output(command, 3, Duration::from_secs(1)).unwrap();
        assert_eq!(output.stdout, b"123");
        assert!(output.truncated);
    }

    #[test]
    fn command_runner_times_out_and_reaps() {
        let mut command = Command::new("sh");
        command.args(["-c", "while :; do :; done"]);
        let started = Instant::now();
        let error = command_output(command, 3, Duration::from_millis(30)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
