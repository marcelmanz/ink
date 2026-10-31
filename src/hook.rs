use std::time::Duration;

use crate::editor::split_command;
use crate::process::{run_captured, RunError};

/// Builds command arguments from a `template`, substituting `{}` with `value`.
/// If `{}` isn't found, `value` is appended to the end. Returns `None` if the template is empty.
pub fn hook_argv(template: &str, value: &str) -> Option<Vec<String>> {
    let mut args = split_command(template, !cfg!(windows));

    if args.is_empty() {
        return None;
    }

    let has_placeholder = args.iter().any(|word| word.contains("{}"));

    if has_placeholder {
        for word in &mut args {
            *word = word.replace("{}", value);
        }
    } else {
        args.push(value.to_string());
    }

    Some(args)
}

/// Run a hook and return its stdout, or a short reason it failed.
pub fn run_hook(argv: &[String], timeout: Duration) -> Result<String, String> {
    let captured = match run_captured(argv, timeout) {
        Ok(captured) => captured,
        Err(RunError::TimedOut) => return Err("timed out".into()),
        Err(RunError::Io(e)) => return Err(format!("{}: {e}", argv[0])),
    };

    if !captured.status.success() {
        let stderr = String::from_utf8_lossy(&captured.stderr);
        let first_line = stderr.lines().map(str::trim).find(|line| !line.is_empty());

        return Err(match (first_line, captured.status.code()) {
            (Some(line), _) => line.to_string(),
            (None, Some(code)) => format!("exit {code}"),
            (None, None) => "killed".into(),
        });
    }

    let stdout = String::from_utf8_lossy(&captured.stdout).into_owned();
    if stdout.trim().is_empty() {
        return Err("no output".into());
    }

    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use crate::hook::hook_argv;

    #[test]
    fn placeholder_is_replaced_by_the_value() {
        assert_eq!(
            hook_argv("wiki {}", "/wiki/Rust").unwrap(),
            ["wiki", "/wiki/Rust"]
        );
    }

    #[test]
    fn placeholder_inside_a_word_stays_in_that_word() {
        assert_eq!(
            hook_argv("wiki --page={}", "Rust").unwrap(),
            ["wiki", "--page=Rust"]
        );
    }

    #[test]
    fn value_is_appended_without_a_placeholder() {
        assert_eq!(
            hook_argv("wiki --lang en", "Rust").unwrap(),
            ["wiki", "--lang", "en", "Rust"]
        );
    }

    #[test]
    fn value_is_never_split() {
        let value = r#"a b "c" 'd'; rm -rf ~"#;
        assert_eq!(hook_argv("wiki {}", value).unwrap(), ["wiki", value]);
        assert_eq!(hook_argv("wiki", value).unwrap(), ["wiki", value]);
    }

    #[test]
    fn empty_template_has_no_argv() {
        assert_eq!(hook_argv("", "x"), None);
        assert_eq!(hook_argv("   ", "x"), None);
    }

    #[cfg(unix)]
    fn run_sh(script: &str) -> Result<String, String> {
        let argv = ["sh", "-c", script].map(String::from);
        crate::hook::run_hook(&argv, std::time::Duration::from_secs(5))
    }

    #[cfg(unix)]
    #[test]
    fn success_returns_stdout() {
        assert_eq!(run_sh("echo '# Page'"), Ok("# Page\n".into()));
    }

    #[cfg(unix)]
    #[test]
    fn failure_reason_is_the_first_stderr_line() {
        assert_eq!(
            run_sh("echo boom >&2; echo more >&2; exit 1"),
            Err("boom".into())
        );
    }

    #[cfg(unix)]
    #[test]
    fn failure_without_stderr_reports_the_exit_code() {
        assert_eq!(run_sh("exit 3"), Err("exit 3".into()));
    }

    #[cfg(unix)]
    #[test]
    fn blank_stdout_is_no_output() {
        assert_eq!(run_sh("echo"), Err("no output".into()));
    }

    #[cfg(unix)]
    #[test]
    fn timeout_is_reported() {
        let argv = ["sh", "-c", "sleep 5"].map(String::from);
        let result = crate::hook::run_hook(&argv, std::time::Duration::from_millis(100));
        assert_eq!(result, Err("timed out".into()));
    }
}
