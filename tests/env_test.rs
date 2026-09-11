//! Ported from `internal/cmd/env_test.go`.

use std::collections::HashMap;

use direnv::cmd::env::{load_env, Env};

/// `TestEnv`
#[test]
fn test_env() {
    let env: Env = HashMap::from([("FOO".to_string(), "bar".to_string())]).into();

    let out = env.serialize();

    let env2 = load_env(&out).expect("parse error");

    assert_eq!("bar", env2.get_or_empty("FOO"), "FOO != bar");
    assert_eq!(1, env2.len(), "len != 1");
}
