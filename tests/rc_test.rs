//! Ported from `internal/cmd/rc_test.go`.

use direnv::cmd::rc::each_dir;

/// `TestSomething`
#[test]
fn test_something() {
    let paths = each_dir("/foo/b//bar/");
    assert_eq!(4, paths.len());
    // TODO: fix me for windows
    let paths = each_dir("/");
    assert!(paths.len() == 1 && paths[0] == "/");
}
