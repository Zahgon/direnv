//! Ported from `internal/cmd/cmd_version_test.go`.

use direnv::cmd::cmd_version::ensure_v_prefixed;
use direnv::deps::gosemver;

/// `TestVersionDotTxt`
#[test]
fn test_version_dot_txt() {
    let bs = std::fs::read_to_string("version.txt").expect("failed to read version.txt");
    let version = bs.trim();

    assert!(
        gosemver::is_valid(&ensure_v_prefixed(version)),
        "version.txt does not contain a valid semantic version: {version:?}"
    );
}
