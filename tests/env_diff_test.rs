//! Ported from `internal/cmd/env_diff_test.go`.

use std::collections::HashMap;

use direnv::cmd::consts::{DIRENV_BASH, DIRENV_DIFF};
use direnv::cmd::env::Env;
use direnv::cmd::env_diff::{build_env_diff, ignored_env, load_env_diff, EnvDiff};

/// `TestEnvDiff`
#[test]
fn test_env_diff() {
    let diff = EnvDiff {
        prev: HashMap::from([("FOO".to_string(), "bar".to_string())]),
        next: HashMap::from([("BAR".to_string(), "baz".to_string())]),
    };

    let out = diff.serialize();

    let diff2 = load_env_diff(&out).expect("parse error");

    assert_eq!(1, diff2.prev.len(), "len(diff2.prev) != 1");
    assert_eq!(1, diff2.next.len(), "len(diff2.next) != 0");
}

/// `TestEnvDiffEmptyValue` — issue #114.
///
/// Check that empty environment variables correctly appear in the diff.
#[test]
fn test_env_diff_empty_value() {
    let before = Env::new();
    let after: Env = HashMap::from([("FOO".to_string(), String::new())]).into();

    let diff = build_env_diff(&before, &after);

    assert_eq!(
        after.0, diff.next,
        "diff.Next != after ({:?} != {:?})",
        diff.next, after.0
    );
}

/// `TestIgnoredEnv`
#[test]
fn test_ignored_env() {
    assert!(ignored_env(DIRENV_BASH));
    assert!(!ignored_env(DIRENV_DIFF));
    assert!(ignored_env("_"));
    assert!(ignored_env("__fish_foo"));
    assert!(ignored_env("__fishx"));
}
