//! Tests for behaviour the Go build got from its standard library and from
//! `golang.org/x/mod/semver`, and that this port now implements itself.
//!
//! None of this was covered by the original's own suite - it was covered by
//! someone else's. Every expectation below is a value taken from Go.

use direnv::deps::goexpand;
use direnv::deps::gojson::{self, JsonValue};
use direnv::deps::gopath;
use direnv::deps::goregexp;
use direnv::deps::gosemver;
use direnv::deps::gotemplate::{execute, HookContext};
use direnv::deps::gotime;

// --- golang.org/x/mod/semver ------------------------------------------------

/// `semver.IsValid` accepts a bare major and a major.minor, and requires the
/// leading `v`.
#[test]
fn semver_is_valid() {
    for good in ["v2", "v2.37", "v2.37.1", "v2.37.1-rc.1", "v2.37.1+build.5"] {
        assert!(gosemver::is_valid(good), "{good} should be valid");
    }
    for bad in ["2.37.1", "", "v", "v2.37.1.2", "v02.1.0", "vx", "v2.37.1-"] {
        assert!(!gosemver::is_valid(bad), "{bad} should be invalid");
    }
}

/// `semver.Compare` orders numerically per field, sorts a pre-release below its
/// release, and ignores build metadata.
#[test]
fn semver_compare() {
    assert_eq!(0, gosemver::compare("v2.37.1", "v2.37.1"));
    assert_eq!(-1, gosemver::compare("v2.37.1", "v2.37.2"));
    assert_eq!(1, gosemver::compare("v2.37.1", "v2.9.9"));
    assert_eq!(0, gosemver::compare("v2", "v2.0.0"));
    assert_eq!(-1, gosemver::compare("v1.0.0-rc.1", "v1.0.0"));
    assert_eq!(-1, gosemver::compare("v1.0.0-rc.1", "v1.0.0-rc.2"));
    assert_eq!(-1, gosemver::compare("v1.0.0-1", "v1.0.0-alpha"));
    assert_eq!(0, gosemver::compare("v1.0.0+a", "v1.0.0+b"));
    // An invalid version sorts below a valid one; two invalids are equal.
    assert_eq!(-1, gosemver::compare("nope", "v1.0.0"));
    assert_eq!(0, gosemver::compare("nope", "also-nope"));
}

// --- time.ParseDuration / Duration.String -----------------------------------

#[test]
fn duration_parses_the_go_grammar() {
    assert_eq!(Ok(5 * gotime::SECOND), gotime::parse_duration("5s"));
    assert_eq!(Ok(0), gotime::parse_duration("0"));
    assert_eq!(
        Ok(2 * gotime::HOUR + 45 * gotime::MINUTE),
        gotime::parse_duration("2h45m")
    );
    assert_eq!(Ok(-90 * gotime::MINUTE), gotime::parse_duration("-1.5h"));
    assert_eq!(
        Ok(300 * gotime::MILLISECOND),
        gotime::parse_duration("300ms")
    );
    assert_eq!(Ok(1_000), gotime::parse_duration("1us"));
    assert_eq!(Ok(1_000), gotime::parse_duration("1\u{00b5}s"));
}

#[test]
fn duration_rejects_the_way_go_does() {
    assert_eq!(
        Err("time: missing unit in duration \"5\"".to_string()),
        gotime::parse_duration("5")
    );
    assert_eq!(
        Err("time: unknown unit \"y\" in duration \"3y\"".to_string()),
        gotime::parse_duration("3y")
    );
    assert_eq!(
        Err("time: invalid duration \"\"".to_string()),
        gotime::parse_duration("")
    );
    assert_eq!(
        Err("time: invalid duration \"abc\"".to_string()),
        gotime::parse_duration("abc")
    );
}

/// The spelling `direnv status` prints for `warn_timeout`.
#[test]
fn duration_string_matches_go() {
    assert_eq!("0s", gotime::duration_string(0));
    assert_eq!("5s", gotime::duration_string(5 * gotime::SECOND));
    assert_eq!("1h0m0s", gotime::duration_string(gotime::HOUR));
    assert_eq!(
        "2h45m0s",
        gotime::duration_string(2 * gotime::HOUR + 45 * gotime::MINUTE)
    );
    assert_eq!("1.5s", gotime::duration_string(1_500 * gotime::MILLISECOND));
    assert_eq!("300ms", gotime::duration_string(300 * gotime::MILLISECOND));
    assert_eq!("1\u{00b5}s", gotime::duration_string(gotime::MICROSECOND));
    assert_eq!("15ns", gotime::duration_string(15));
    assert_eq!("-1m30s", gotime::duration_string(-90 * gotime::SECOND));
}

/// `time.Unix(sec, 0).MarshalText()` renders whole seconds without a fraction.
#[test]
fn unix_marshal_text_is_strict_rfc3339() {
    let rendered = gotime::unix_marshal_text(0);
    assert!(rendered.starts_with("19"), "{rendered}");
    assert!(!rendered.contains('.'), "no fraction for a whole second");
    let zone = &rendered[19..];
    assert!(
        zone == "Z" || (zone.len() == 6 && (zone.starts_with('+') || zone.starts_with('-'))),
        "unexpected zone {zone:?}"
    );
}

// --- os.Expand --------------------------------------------------------------

#[test]
fn expand_handles_gos_awkward_cases() {
    let map = |name: &str| format!("<{name}>");
    assert_eq!("<FOO>", goexpand::expand("$FOO", map));
    assert_eq!("<FOO>", goexpand::expand("${FOO}", map));
    assert_eq!("<FOO>/bar", goexpand::expand("$FOO/bar", map));
    // "${}" is valid-but-empty syntax: the characters are eaten.
    assert_eq!("", goexpand::expand("${}", map));
    // A dangling "${" is eaten too.
    assert_eq!("", goexpand::expand("${", map));
    // A "$" not followed by a name is left alone.
    assert_eq!("$", goexpand::expand("$", map));
    assert_eq!("$ x", goexpand::expand("$ x", map));
    // Shell special single-character names.
    assert_eq!("<*>", goexpand::expand("$*", map));
    assert_eq!("<1>", goexpand::expand("$1", map));
    assert_eq!("<-> ", goexpand::expand("${-} ", map));
    // A name that is not a shell special still scans to the closing brace.
    assert_eq!("<:->", goexpand::expand("${:-}", map));
}

// --- path/filepath ----------------------------------------------------------

#[test]
fn clean_is_lexical() {
    assert_eq!(".", gopath::clean(""));
    assert_eq!("/foo/b/bar", gopath::clean("/foo/b//bar/"));
    assert_eq!("/foo", gopath::clean("/foo/bar/.."));
    assert_eq!("/", gopath::clean("/.."));
    assert_eq!("../..", gopath::clean("../../"));
    assert_eq!("a/c", gopath::clean("a/b/../c"));
    // Purely lexical: `docs` need not exist.
    assert_eq!("file_times.rs", gopath::clean("docs/../file_times.rs"));
}

#[test]
fn base_and_dir_match_go() {
    assert_eq!(".", gopath::base(""));
    assert_eq!("/", gopath::base("/"));
    assert_eq!("bar", gopath::base("/foo/bar/"));
    assert_eq!("bash", gopath::base("-/bin/bash"));
    assert_eq!("/foo", gopath::dir("/foo/bar"));
    assert_eq!("/", gopath::dir("/foo"));
    assert_eq!(".", gopath::dir("foo"));
}

#[test]
fn rel_matches_go() {
    assert_eq!(Ok(".".to_string()), gopath::rel("/a/b", "/a/b"));
    assert_eq!(Ok("c".to_string()), gopath::rel("/a/b", "/a/b/c"));
    assert_eq!(Ok("../c".to_string()), gopath::rel("/a/b", "/a/c"));
    assert_eq!(Ok("../../..".to_string()), gopath::rel("/a/b/c/d", "/a"));
    assert!(gopath::rel("/a", "b").is_err());
}

// --- text/template ----------------------------------------------------------

#[test]
fn hook_template_substitutes_self_path() {
    let ctx = HookContext {
        self_path: "/usr/local/bin/direnv".to_string(),
    };
    assert_eq!(
        Ok("x /usr/local/bin/direnv y".to_string()),
        execute("hook", "x {{.SelfPath}} y", &ctx)
    );
    assert_eq!(
        Ok("/usr/local/bin/direnv".to_string()),
        execute("hook", "{{ .SelfPath }}", &ctx)
    );
    // Braces that are not an action are left alone.
    assert_eq!(
        Ok("${PROMPT_COMMAND:-}".to_string()),
        execute("hook", "${PROMPT_COMMAND:-}", &ctx)
    );
    assert!(execute("hook", "{{.Nope}}", &ctx).is_err());
}

// --- encoding/json ----------------------------------------------------------

/// Go escapes HTML by default, so `<`, `>` and `&` are never literal in any of
/// direnv's JSON output.
#[test]
fn json_escapes_html_like_go() {
    let value = JsonValue::Object(vec![(
        "k".to_string(),
        JsonValue::String("a<b>c&d".to_string()),
    )]);
    assert_eq!(
        "{\"k\":\"a\\u003cb\\u003ec\\u0026d\"}",
        gojson::marshal(&value)
    );
}

#[test]
fn json_control_characters_match_go() {
    let value = JsonValue::String("a\nb\tc\rd\u{0001}e\u{2028}".to_string());
    assert_eq!("\"a\\nb\\tc\\rd\\u0001e\\u2028\"", gojson::marshal(&value));
}

#[test]
fn json_indent_and_encode_match_go() {
    let value = JsonValue::Object(vec![
        ("b".to_string(), JsonValue::String("2".to_string())),
        ("a".to_string(), JsonValue::String("1".to_string())),
    ]);
    // MarshalIndent has no trailing newline; Encoder.Encode adds one.
    assert_eq!(
        "{\n  \"b\": \"2\",\n  \"a\": \"1\"\n}",
        gojson::marshal_indent(&value, "  ")
    );
    assert_eq!("{\"b\":\"2\",\"a\":\"1\"}\n", gojson::encode(&value));
    // Empty containers stay on one line, as in Go.
    assert_eq!(
        "{}",
        gojson::marshal_indent(&JsonValue::Object(vec![]), "  ")
    );
    assert_eq!(
        "[]",
        gojson::marshal_indent(&JsonValue::Array(vec![]), "  ")
    );
}

/// Go unmarshals every JSON number into a float64 and marshals whole values
/// back without a fraction - a watch's mtime must survive a `show_dump`.
#[test]
fn json_numbers_round_trip_like_go() {
    let parsed = gojson::parse("{\"modtime\":1788929604,\"exists\":true}").expect("parse");
    assert_eq!(
        "{\"exists\":true,\"modtime\":1788929604}",
        gojson::marshal(&parsed)
    );
}

// --- regexp -----------------------------------------------------------------

/// Go treats a `{` that does not start a valid repetition as a literal brace.
#[test]
fn regexp_brace_leniency_matches_go() {
    assert_eq!("a\\{", goregexp::translate("a{"));
    assert_eq!("a{2}", goregexp::translate("a{2}"));
    assert_eq!("a{2,}", goregexp::translate("a{2,}"));
    assert_eq!("a{2,3}", goregexp::translate("a{2,3}"));
    assert_eq!("a\\{2,3", goregexp::translate("a{2,3"));
    assert_eq!("a\\{}", goregexp::translate("a{}"));
    // Inside a character class the brace is already a literal.
    assert_eq!("[a{]", goregexp::translate("[a{]"));
    // `\p{...}` and `\x{...}` carry their own braces.
    assert_eq!("\\p{Greek}", goregexp::translate("\\p{Greek}"));
    assert_eq!("\\x{1F600}", goregexp::translate("\\x{1F600}"));
}

/// Go's Perl classes are ASCII-only, so `\w` must not match `é`.
#[test]
fn regexp_perl_classes_are_ascii() {
    assert_eq!("[0-9A-Za-z_]", goregexp::translate("\\w"));
    assert_eq!("[^0-9A-Za-z_]", goregexp::translate("\\W"));
    assert_eq!("[0-9]", goregexp::translate("\\d"));
    assert_eq!("[a0-9A-Za-z_]", goregexp::translate("[a\\w]"));
    assert_eq!("(?-u:\\b)x", goregexp::translate("\\bx"));
}

/// Perl quoting and octal escapes exist in Go's engine and not in the crate's.
#[test]
fn regexp_quoting_and_octal_match_go() {
    assert_eq!("a\\*b", goregexp::translate("\\Qa*b\\E"));
    assert_eq!("a\\*b", goregexp::translate("\\Qa*b"));
    assert_eq!("\\x{a}", goregexp::translate("\\012"));
    assert_eq!("\\x{0}", goregexp::translate("\\0"));
    // A lone non-zero digit is a back-reference in Go, not an octal escape.
    assert_eq!("\\1", goregexp::translate("\\1"));
}

/// Go allows a repeated capture-group name; the crate does not, and direnv
/// never reads the names.
#[test]
fn regexp_named_groups_are_flattened() {
    assert_eq!("(x)(y)", goregexp::translate("(?P<n>x)(?P<n>y)"));
    assert_eq!("(x)", goregexp::translate("(?<n>x)"));
    assert_eq!("(?:x)", goregexp::translate("(?:x)"));
    assert_eq!("(?i)x", goregexp::translate("(?i)x"));
}

/// Perl, and so Go, refuses to stack repetition operators.
#[test]
fn regexp_rejects_nested_repetition_like_go() {
    assert_eq!(
        Err("error parsing regexp: invalid nested repetition operator: `**`".to_string()),
        goregexp::compile_pattern("a**")
    );
    assert_eq!(
        Err("error parsing regexp: invalid nested repetition operator: `{2}{3}`".to_string()),
        goregexp::compile_pattern("a{2}{3}")
    );
    // A single trailing `?` is the non-greedy marker, not a second operator.
    assert!(goregexp::compile_pattern("a*?").is_ok());
    assert!(goregexp::compile_pattern("a+?").is_ok());
    assert!(goregexp::compile_pattern("[*]").is_ok());
}
