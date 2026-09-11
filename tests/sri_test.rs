//! Ported from `pkg/sri/sri_test.go`.

use std::io::Write;

use direnv::sri::{parse, Algo, Writer};

/// `TestWriter`
#[test]
fn test_writer() {
    let mut b: Vec<u8> = Vec::new();

    let s = "testdata";

    // Generated with:
    // `echo -n "testdata" | openssl dgst -sha256 -binary - | openssl base64 -A`
    let expected_hash = "sha256-gQ/y+yQqXe5CIPLLDmpRmJH7Z/L4KKbKtO+IlGM7H1A=";

    let mut w = Writer::new(&mut b, Algo::Sha256);

    // Check the writer
    let n = w.write(s.as_bytes()).expect("write error");
    assert_eq!(s.len(), n, "expected len {} but got {n}", s.len());

    // Check that the hash has been calculated properly
    let x = w.sum().to_string();
    assert_eq!(expected_hash, x, "hash mismatch");

    assert_eq!(s.as_bytes(), &b[..], "data has not been forwarded");
}

/// `TestParser`
#[test]
fn test_parser() {
    let expected_hash = "sha256-gQ/y+yQqXe5CIPLLDmpRmJH7Z/L4KKbKtO+IlGM7H1A=";

    let hash = parse(expected_hash).expect("parse error");

    assert_eq!(expected_hash, hash.to_string(), "hash mismatch");
}
