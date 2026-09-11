//! Ported from `internal/cmd/file_times_test.go`.
//!
//! The original's fixture is the package's own source file, read relative to
//! the package directory. Cargo runs an integration test with the crate root as
//! the working directory, so the same idea points at `src/cmd/file_times.rs`.

use direnv::cmd::file_times::{FileTime, FileTimes};
use direnv::deps::gojson;

const FIXTURE: &str = "src/cmd/file_times.rs";

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs() as i64
}

/// `TestUpdate`
#[test]
fn test_update() {
    let mut times = FileTimes::new();
    let _ = times.update(FIXTURE);
    assert_eq!(1, times.list.len(), "Length of updated list not 1");

    assert!(times.list[0].exists, "Existing file marked not existing");
}

/// `TestFTJsons`
#[test]
fn test_ft_jsons() {
    let ft = FileTime {
        path: "something.txt".to_string(),
        modtime: now(),
        exists: true,
    };
    let marshalled = gojson::marshal(&ft.to_json());
    assert_ne!("{}", marshalled, "{ft:?} marshals as empty object");
}

/// `TestRoundTrip`
#[test]
fn test_round_trip() {
    let mut watches = FileTimes::new();
    let _ = watches.update(FIXTURE);

    let mut rt_chk = FileTimes::new();
    let _ = rt_chk.unmarshal(&watches.marshal());

    compare_fts(&watches, &rt_chk, "length", |ft| ft.list.len().to_string());
    compare_fts(&watches, &rt_chk, "first path", |ft| {
        ft.list[0].path.clone()
    });
}

fn compare_fts<F>(left: &FileTimes, right: &FileTimes, desc: &str, compare: F)
where
    F: Fn(&FileTimes) -> String,
{
    let (lc, rc) = (compare(left), compare(right));
    assert_eq!(
        lc, rc,
        "FileTimes didn't round trip. Original {desc} was: {lc} RT {desc} was: {rc}"
    );
}

/// `TestCanonicalAdds`
#[test]
fn test_canonical_adds() {
    let mut fts = FileTimes::new();
    let _ = fts.new_time("src/cmd/docs/../file_times.rs", 0, true);
    let _ = fts.new_time(FIXTURE, 0, true);
    assert!(fts.list.len() <= 1, "Double add of the same file");
}

/// `TestCheckPasses`
#[test]
fn test_check_passes() {
    let mut fts = FileTimes::new();
    let _ = fts.update(FIXTURE);
    let err = fts.check();
    assert!(err.is_ok(), "Check that should pass fails with: {err:?}");
}

/// `TestCheckStale`
#[test]
fn test_check_stale() {
    let mut fts = FileTimes::new();
    let _ = fts.new_time(FIXTURE, 0, true);
    assert!(
        fts.check().is_err(),
        "Check that should fail because stale passes"
    );
}

/// `TestCheckAppeared`
#[test]
fn test_check_appeared() {
    let mut fts = FileTimes::new();
    let _ = fts.new_time(FIXTURE, 0, false);
    assert!(
        fts.check().is_err(),
        "Check that should fail because appeared passes"
    );
}

/// `TestCheckGone`
#[test]
fn test_check_gone() {
    let mut fts = FileTimes::new();
    let _ = fts.new_time("nosuchfileevarright.rs", now() + 1000, true);
    assert!(
        fts.check().is_err(),
        "Check that should fail because gone passes"
    );
}
