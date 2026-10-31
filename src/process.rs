use std::io::{self, Read};
use std::process::{Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// What a finished command wrote and how it exited.
pub struct Captured {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug)]
pub enum RunError {
    Io(io::Error),
    TimedOut,
}

/// Run `argv` with stdin null and its output captured, killing it after `timeout`.
pub fn run_captured(argv: &[String], timeout: Duration) -> Result<Captured, RunError> {
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| RunError::Io(io::Error::new(io::ErrorKind::InvalidInput, "no program")))?;

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(RunError::Io)?;

    let stdout = read_to_end_in_background(child.stdout.take());
    let stderr = read_to_end_in_background(child.stderr.take());
    let deadline = Instant::now() + timeout;

    let status = loop {
        if let Some(status) = child.try_wait().map_err(RunError::Io)? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();

            return Err(RunError::TimedOut);
        }
        thread::sleep(Duration::from_millis(10));
    };

    Ok(Captured {
        status,
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

fn read_to_end_in_background(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();

        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    })
}

#[cfg(all(test, unix))]
mod tests {
    use crate::process::{run_captured, RunError};
    use std::time::Duration;

    fn run_sh(script: &str, timeout: Duration) -> Result<crate::process::Captured, RunError> {
        let argv = ["sh", "-c", script].map(String::from);
        run_captured(&argv, timeout)
    }

    #[test]
    fn success_captures_stdout_and_stderr() {
        let captured = run_sh("echo out; echo err >&2", Duration::from_secs(5)).unwrap();

        assert!(captured.status.success());
        assert_eq!(captured.stdout, b"out\n");
        assert_eq!(captured.stderr, b"err\n");
    }

    #[test]
    fn non_zero_exit_is_reported_in_the_status() {
        let captured = run_sh("exit 3", Duration::from_secs(5)).unwrap();

        assert_eq!(captured.status.code(), Some(3));
    }

    #[test]
    fn a_command_past_its_timeout_is_killed() {
        let result = run_sh("sleep 5", Duration::from_millis(100));

        assert!(matches!(result, Err(RunError::TimedOut)));
    }

    #[test]
    fn a_missing_program_is_an_io_error() {
        let argv = ["ink-no-such-program".to_string()];
        let result = run_captured(&argv, Duration::from_secs(5));

        assert!(matches!(result, Err(RunError::Io(_))));
    }
}
