use std::io::{Write, read_to_string};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

fn spawn_cmd(cmd: &str) -> Command {
    if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.arg("/C").arg(cmd);
        command
    } else {
        let mut command = Command::new("sh");
        command.arg("-c").arg(cmd);
        command
    }
}

/// Runs the given command silently and returns the content of STDERR if it failed.
pub(crate) fn run_silent(cmd: &str) -> anyhow::Result<Option<String>> {
    let child = spawn_cmd(cmd).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn()?;
    let output = child.wait_with_output()?;
    let stderr = if output.status.success() { None } else { Some(String::from_utf8_lossy(&output.stderr).to_string()) };
    Ok(stderr)
}

/// Runs the given command and lets the user interact with it.
pub(crate) fn run_interactive(cmd: &str) -> anyhow::Result<()> {
    let mut child = spawn_cmd(cmd).stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit()).spawn()?;
    child.wait()?;
    Ok(())
}

pub(crate) struct Output {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) success: bool,
    pub(crate) duration: Duration,
}

/// Runs the given command with input provided and returns the output with duration.
/// When timeout is reached, the process is killed and None is returned.
/// https://stackoverflow.com/a/62133239/4595904
pub(crate) fn run_with_input_timed(cmd: &str, input: &str, timeout: Duration) -> anyhow::Result<Option<Output>> {
    let mut child = spawn_cmd(cmd).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let start_time = Instant::now();
    let mut stdin = child.stdin.take().unwrap();
    write!(stdin, "{}", input)?;
    drop(stdin);
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let Some(status) = child.wait_timeout(timeout)? else {
        child.kill()?;
        return Ok(None);
    };
    let duration = start_time.elapsed();
    Ok(Some(Output {
        stdout: read_to_string(&mut stdout)?,
        stderr: read_to_string(&mut stderr)?,
        success: status.success(),
        duration,
    }))
}