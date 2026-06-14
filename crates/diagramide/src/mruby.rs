use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Mutex, OnceLock,
        mpsc::{self, Receiver, RecvTimeoutError},
    },
    thread,
    time::Duration,
};
use tokio::task;

pub const DEFAULT_MRUBY_TIMEOUT: Duration = Duration::from_secs(2);
pub const DEFAULT_MRUBY_OUTPUT_LIMIT: usize = 10 * 1024 * 1024;

const MRUBY_COMMANDS: &[&str] = &["mruby", "/opt/homebrew/bin/mruby", "/usr/local/bin/mruby"];

const WORKER_SCRIPT: &str = r##"
$diagramide_write = STDOUT.method(:write)

module Kernel
  def print(*args)
    $diagramide_output << args.map(&:to_s).join
    nil
  end

  def puts(*args)
    args = [""] if args.empty?
    args.each { |arg| $diagramide_output << arg.to_s << "\n" }
    nil
  end
end

loop do
  header = STDIN.gets
  break unless header

  script = STDIN.read(header.to_i)
  STDIN.read(1)
  $diagramide_output = ""

  begin
    Object.new.instance_eval(script)
    status = "OK"
    payload = $diagramide_output
    if payload.bytesize > 10_485_760
      status = "ERR"
      payload = "mruby output exceeded 10 MiB"
    end
  rescue Exception => error
    status = "ERR"
    payload = "#{error.class}: #{error.message}"
  end

  $diagramide_write.call("#{status} #{payload.bytesize}\n")
  $diagramide_write.call(payload)
  $diagramide_write.call("\n")
  STDOUT.flush
end
"##;

static MRUBY_COMMAND: OnceLock<Option<&'static str>> = OnceLock::new();
static MRUBY_WORKER: OnceLock<Mutex<Option<MrubyWorker>>> = OnceLock::new();

struct MrubyWorker {
    child: Child,
    stdin: ChildStdin,
    responses: Receiver<Result<Result<String, String>, String>>,
}

impl MrubyWorker {
    fn spawn() -> Result<Self, String> {
        let command = mruby_command().ok_or_else(|| "mruby executable not found".to_string())?;
        let mut child = Command::new(command)
            .arg("-e")
            .arg(WORKER_SCRIPT)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Failed to start mruby: {error}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to open mruby input".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture mruby output".to_string())?;
        let (response_tx, responses) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let response = read_response(&mut reader);
                let disconnected = response.is_err();
                if response_tx.send(response).is_err() || disconnected {
                    break;
                }
            }
        });

        Ok(Self {
            child,
            stdin,
            responses,
        })
    }

    fn eval(&mut self, script: &str, timeout: Duration) -> Result<Result<String, String>, String> {
        writeln!(self.stdin, "{}", script.len()).map_err(|error| error.to_string())?;
        self.stdin
            .write_all(script.as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .and_then(|_| self.stdin.flush())
            .map_err(|error| error.to_string())?;

        match self.responses.recv_timeout(timeout) {
            Ok(response) => response,
            Err(RecvTimeoutError::Timeout) => Err("mruby execution timed out".to_string()),
            Err(RecvTimeoutError::Disconnected) => {
                Err("mruby worker stopped unexpectedly".to_string())
            },
        }
    }
}

impl Drop for MrubyWorker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn is_mruby_available() -> bool {
    mruby_command().is_some()
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
    let worker = MRUBY_WORKER.get_or_init(|| Mutex::new(None));
    let mut worker = worker
        .lock()
        .map_err(|_| "mruby worker lock was poisoned".to_string())?;

    if worker.is_none() {
        *worker = Some(MrubyWorker::spawn()?);
    }

    let response = worker
        .as_mut()
        .expect("mruby worker was initialized")
        .eval(script, timeout);
    match response {
        Ok(result) => result,
        Err(error) => {
            *worker = None;
            Err(error)
        },
    }
}

fn mruby_command() -> Option<&'static str> {
    *MRUBY_COMMAND.get_or_init(|| {
        MRUBY_COMMANDS.iter().copied().find(|command| {
            Command::new(command)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        })
    })
}

fn read_response(reader: &mut impl BufRead) -> Result<Result<String, String>, String> {
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|error| error.to_string())?;
    if header.is_empty() {
        return Err("mruby worker stopped unexpectedly".to_string());
    }

    let (status, length) = header
        .trim_end()
        .split_once(' ')
        .ok_or_else(|| "Invalid response from mruby worker".to_string())?;
    let length: usize = length
        .parse()
        .map_err(|_| "Invalid response length from mruby worker".to_string())?;
    if length > DEFAULT_MRUBY_OUTPUT_LIMIT {
        return Err("mruby output exceeded 10 MiB".to_string());
    }

    let mut payload = vec![0; length];
    reader
        .read_exact(&mut payload)
        .map_err(|error| error.to_string())?;
    let mut newline = [0];
    reader
        .read_exact(&mut newline)
        .map_err(|error| error.to_string())?;
    if newline[0] != b'\n' {
        return Err("Invalid response from mruby worker".to_string());
    }

    let payload = String::from_utf8(payload)
        .map_err(|error| format!("mruby output was not UTF-8: {error}"))?;
    match status {
        "OK" => Ok(Ok(payload)),
        "ERR" => Ok(Err(payload)),
        _ => Err("Invalid response from mruby worker".to_string()),
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
    fn top_level_helper_methods_are_callable() {
        if !is_mruby_available() {
            return;
        }

        let script = r##"
def print_text(text)
  text.strip.lines.each do |line|
    puts %Q[L: text "#{line}" rjust]
  end
end

print_text "Hello
World
"
"##;

        assert_eq!(
            eval_mruby(script).unwrap(),
            "L: text \"Hello\n\" rjust\nL: text \"World\" rjust\n"
        );
    }

    #[test]
    fn evaluations_use_fresh_scopes() {
        if !is_mruby_available() {
            return;
        }

        eval_mruby("LOCAL_TO_ONE_RENDER = 42").unwrap();
        let error = eval_mruby("print LOCAL_TO_ONE_RENDER")
            .expect_err("constants from a previous render must not be visible");
        assert!(error.contains("LOCAL_TO_ONE_RENDER"), "{error}");
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
    fn timeout_restarts_worker() {
        if !is_mruby_available() {
            return;
        }

        let error = eval_mruby_with_timeout("loop {}", Duration::from_millis(20))
            .expect_err("infinite mruby loop should exceed its time limit");
        assert_eq!(error, "mruby execution timed out");
        assert_eq!(eval_mruby("print 42").unwrap(), "42");
    }
}
