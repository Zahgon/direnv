//! Ported from `internal/cmd/shell_test.go`.

use direnv::cmd::shell::detect_shell;
use direnv::cmd::shell_bash::bash_escape;

/// `TestBashEscape`
#[test]
fn test_bash_escape() {
    assert_eq(r"''", &bash_escape(""));
    assert_eq(r"$'escape\'quote'", &bash_escape("escape'quote"));
    assert_eq(r"$'foo\r\n\tbar'", &bash_escape("foo\r\n\tbar"));
    assert_eq(r"$'foo bar'", &bash_escape("foo bar"));
    assert_eq(r"$'\xc3\xa9'", &bash_escape("é"));
}

/// `TestShellDetection`
#[test]
fn test_shell_detection() {
    assert_not_nil(detect_shell("-bash"));
    assert_not_nil(detect_shell("-/bin/bash"));
    assert_not_nil(detect_shell("-/usr/local/bin/bash"));
    assert_not_nil(detect_shell("-zsh"));
    assert_not_nil(detect_shell("-/bin/zsh"));
    assert_not_nil(detect_shell("-/usr/local/bin/zsh"));
}

fn assert_not_nil(shell: Option<&'static dyn direnv::cmd::shell::Shell>) {
    assert!(shell.is_some(), "Expected not to be nil");
}

fn assert_eq(expected: &str, actual: &str) {
    assert_eq!(
        expected, actual,
        "Expected \"{expected}\" to equal \"{actual}\""
    );
}
