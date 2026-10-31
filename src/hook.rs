use crate::editor::split_command;

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
}
