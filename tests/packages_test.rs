//! Coverage for the standalone packages the Go suite left untested.
//!
//! `xdg` and `gzenv` had no `_test.go` at all, and `pkg/sri` covered only the
//! happy path for SHA-256. The gzenv and xdg expectations were checked against
//! the Go build at commit b00e451; the SRI digests come from `openssl dgst`,
//! the same source the original's own test cites.

use std::collections::HashMap;
use std::io::Write;

use direnv::deps::gojson::{self, JsonValue};
use direnv::gzenv;
use direnv::sri;
use direnv::xdg;

fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// --- xdg --------------------------------------------------------------------

#[test]
fn xdg_prefers_the_explicit_home() {
    let e = env(&[("XDG_DATA_HOME", "/x/data"), ("HOME", "/home/u")]);
    assert_eq!("/x/data/direnv", xdg::data_dir(&e, "direnv"));

    let e = env(&[("XDG_CONFIG_HOME", "/x/cfg"), ("HOME", "/home/u")]);
    assert_eq!("/x/cfg/direnv", xdg::config_dir(&e, "direnv"));

    let e = env(&[("XDG_CACHE_HOME", "/x/cache"), ("HOME", "/home/u")]);
    assert_eq!("/x/cache/direnv", xdg::cache_dir(&e, "direnv"));
}

#[test]
fn xdg_falls_back_to_home() {
    let e = env(&[("HOME", "/home/u")]);
    assert_eq!("/home/u/.local/share/direnv", xdg::data_dir(&e, "direnv"));
    assert_eq!("/home/u/.config/direnv", xdg::config_dir(&e, "direnv"));
    assert_eq!("/home/u/.cache/direnv", xdg::cache_dir(&e, "direnv"));
}

#[test]
fn xdg_is_empty_without_home() {
    // An empty value counts as unset, exactly as in the original.
    let e = env(&[("HOME", ""), ("XDG_DATA_HOME", "")]);
    assert_eq!("", xdg::data_dir(&e, "direnv"));
    assert_eq!("", xdg::config_dir(&env(&[]), "direnv"));
    assert_eq!("", xdg::cache_dir(&env(&[]), "direnv"));
}

// --- gzenv ------------------------------------------------------------------

#[test]
fn gzenv_round_trips() {
    let value = JsonValue::Object(vec![
        ("FOO".to_string(), JsonValue::String("bar".to_string())),
        ("AMP".to_string(), JsonValue::String("a&b".to_string())),
    ]);
    let encoded = gzenv::marshal(&value);
    // The URL alphabet, with padding.
    assert!(!encoded.contains('+') && !encoded.contains('/'));
    let decoded = gzenv::unmarshal(&encoded).expect("round trip");
    assert_eq!(
        "{\"AMP\":\"a\\u0026b\",\"FOO\":\"bar\"}",
        gojson::marshal(&decoded)
    );
    // Surrounding whitespace is trimmed before decoding.
    assert_eq!(
        decoded,
        gzenv::unmarshal(&format!("  {encoded}  ")).unwrap()
    );
}

#[test]
fn gzenv_reports_failures_the_way_go_does() {
    assert_eq!(
        Err("unmarshal() base64 decoding: illegal base64 data at input byte 0".to_string()),
        gzenv::unmarshal("!!!")
    );
    assert_eq!(
        Err("unmarshal() zlib opening: zlib: invalid header".to_string()),
        gzenv::unmarshal("aGVsbG8=")
    );
    assert_eq!(
        Err("unmarshal() zlib opening: unexpected EOF".to_string()),
        gzenv::unmarshal("eA==")
    );
    assert_eq!(
        Err("unmarshal() zlib decoding: unexpected EOF".to_string()),
        gzenv::unmarshal("eJw=")
    );
}

// --- sri --------------------------------------------------------------------

#[test]
fn sri_supports_every_algorithm() {
    for (algo, expected) in [
        (
            sri::Algo::Sha256,
            "sha256-gQ/y+yQqXe5CIPLLDmpRmJH7Z/L4KKbKtO+IlGM7H1A=",
        ),
        (
            sri::Algo::Sha384,
            "sha384-SN5oSMR+DBSCs8oTBnjAk/K4YgNM6YTt2sIWJnx3R417hCYJajv79w6fXvrGmWwa",
        ),
        (
            sri::Algo::Sha512,
            "sha512-dvTKSPXuqQRx/AV54vshB44GZBpyMzlYJVUOVinvyn8G3TC804fd8vvBFL7qs/DdmV61dDdRvXJz0OUU7LOTmw==",
        ),
    ] {
        let mut sink: Vec<u8> = Vec::new();
        let mut writer = sri::Writer::new(&mut sink, algo);
        writer.write_all(b"testdata").expect("write");
        let hash = writer.sum();
        assert_eq!(expected, hash.to_string());
        // The parser round-trips its own rendering.
        assert_eq!(expected, sri::parse(expected).expect("parse").to_string());
        // And the hex form is what names the CAS file.
        assert_eq!(hash.hex(), sri::parse(expected).unwrap().hex());
    }
}

#[test]
fn sri_rejects_bad_hashes_the_way_go_does() {
    assert_eq!(Err("sri: not a hash nope".to_string()), sri::parse("nope"));
    assert_eq!(
        Err("sri: unsupported algo md5".to_string()),
        sri::parse("md5-abcd")
    );
    assert!(sri::parse("sha256-!!!").is_err());
}
