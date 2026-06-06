use std::{
    io::Read,
    process::{Command, Stdio},
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};
use tokio::task;

pub const DEFAULT_MRUBY_TIMEOUT: Duration = Duration::from_secs(2);
pub const DEFAULT_MRUBY_OUTPUT_LIMIT: u64 = 10 * 1024 * 1024;
static MRUBY_AVAILABLE: OnceLock<bool> = OnceLock::new();

pub fn is_mruby_available() -> bool {
    *MRUBY_AVAILABLE.get_or_init(|| {
        Command::new("mruby")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })
}

pub async fn safe_eval_mruby(script: String) -> Result<String, String> {
    task::spawn_blocking(move || eval_mruby(&script))
        .await
        .map_err(|error| error.to_string())?
}

pub fn eval_mruby(script: &str) -> Result<String, String> {
    eval_mruby_with_timeout(script, DEFAULT_MRUBY_TIMEOUT)
}

pub fn eval_mruby_with_timeout(script: &str, timeout: Duration) -> Result<String, String> {
    let mut child = Command::new("mruby")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to start mruby: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture mruby output".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture mruby errors".to_string())?;
    let stdout_reader = thread::spawn(move || read_all(stdout));
    let stderr_reader = thread::spawn(move || read_all(stderr));

    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err("mruby execution timed out".to_string());
        }
        thread::sleep(Duration::from_millis(5));
    };

    let stdout = stdout_reader
        .join()
        .map_err(|_| "mruby output reader panicked".to_string())??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| "mruby error reader panicked".to_string())??;

    if status.success() {
        String::from_utf8(stdout).map_err(|error| format!("mruby output was not UTF-8: {error}"))
    } else {
        let error = String::from_utf8_lossy(&stderr).trim().to_string();
        if error.is_empty() {
            Err(format!("mruby exited with {status}"))
        } else {
            Err(error)
        }
    }
}

fn read_all(reader: impl Read) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take(DEFAULT_MRUBY_OUTPUT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > DEFAULT_MRUBY_OUTPUT_LIMIT {
        Err("mruby output exceeded 10 MiB".to_string())
    } else {
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_and_puts_become_pikchr_output() {
        if !is_mruby_available() {
            return;
        }

        assert_eq!(
            eval_mruby(r#"print "box"; puts " \"hello\"""#).unwrap(),
            "box \"hello\"\n"
        );
    }

    #[test]
    fn runtime_errors_are_returned() {
        if !is_mruby_available() {
            return;
        }

        let error = eval_mruby("raise 'broken'").expect_err("raise should fail evaluation");
        assert!(error.contains("broken"), "{error}");
    }

    #[test]
    fn timeout_stops_non_terminating_mruby() {
        if !is_mruby_available() {
            return;
        }

        let error = eval_mruby_with_timeout("loop {}", Duration::from_millis(20))
            .expect_err("infinite mruby loop should exceed its time limit");
        assert_eq!(error, "mruby execution timed out");
    }
}
