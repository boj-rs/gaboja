use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::runtime;

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

fn spawn_cmd_tokio(cmd: &str) -> tokio::process::Command {
    if cfg!(target_os = "windows") {
        let mut command = tokio::process::Command::new("cmd");
        command.arg("/C").arg(cmd);
        command
    } else {
        let mut command = tokio::process::Command::new("sh");
        command.arg("-c").arg(cmd);
        command
    }
}

/// Runs the given command silently and returns the content of STDERR if it failed.
pub(crate) fn run_silent(cmd: &str) -> anyhow::Result<Option<String>> {
    let child = spawn_cmd(cmd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let output = child.wait_with_output()?;
    let stderr = if output.status.success() {
        None
    } else {
        Some(String::from_utf8_lossy(&output.stderr).to_string())
    };
    Ok(stderr)
}

/// Runs the given command and lets the user interact with it.
pub(crate) fn run_interactive(cmd: &str) -> anyhow::Result<()> {
    let mut child = spawn_cmd(cmd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
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
pub(crate) fn run_with_input_timed(
    cmd: &str,
    input: &str,
    timeout: Duration,
) -> anyhow::Result<Option<Output>> {
    let rt = runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?;
    rt.block_on(async {
        let mut child = spawn_cmd_tokio(cmd)
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let start_time = Instant::now();
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(input.as_bytes()).await?;
        drop(stdin);
        let result = tokio::time::timeout(timeout, child.wait_with_output()).await;
        let duration = start_time.elapsed();
        let result = match result {
            Ok(child_result) => child_result?,
            Err(_timeout_err) => return Ok(None),
        };
        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();
        let success = result.status.success();
        Ok::<Option<Output>, anyhow::Error>(Some(Output {
            stdout,
            stderr,
            success,
            duration,
        }))
    })
}
