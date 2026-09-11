//! A compressed environment format using json+zlib+base64.
//!
//! It provides a quickly designed format to export the whole environment back
//! into itself. `DIRENV_DIFF` and `DIRENV_WATCHES` travel through the user's
//! environment in this encoding, so a payload written by one build of direnv
//! must be readable by any other.

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};

use crate::deps::gojson::{self, JsonValue};

/// Encodes the value into the gzenv format.
pub fn marshal(value: &JsonValue) -> String {
    let json_data = gojson::marshal(value);

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    // We assume the zlib writer would never fail.
    let _ = encoder.write_all(json_data.as_bytes());
    let zlib_data = match encoder.finish() {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Warning: failed to close zlib writer: {err}");
            Vec::new()
        }
    };

    URL_SAFE.encode(zlib_data)
}

/// Restores the gzenv format back into a JSON value.
pub fn unmarshal(gzenv: &str) -> Result<JsonValue, String> {
    let gzenv = gzenv.trim();

    let data = URL_SAFE
        .decode(gzenv)
        .map_err(|err| format!("unmarshal() base64 decoding: {}", base64_error(&err)))?;

    // Go's `zlib.NewReader` validates the two header bytes eagerly and reports
    // that as a distinct failure from a truncated or corrupt payload.
    check_zlib_header(&data).map_err(|err| format!("unmarshal() zlib opening: {err}"))?;

    let mut reader = ZlibDecoder::new(&data[..]);
    let mut env_data = Vec::new();
    // G110: Potential DoS vulnerability via decompression bomb - the payload
    // comes from direnv's own environment, exactly as in the original.
    reader
        .read_to_end(&mut env_data)
        .map_err(|err| format!("unmarshal() zlib decoding: {}", inflate_error(&err)))?;

    let text = String::from_utf8_lossy(&env_data);
    gojson::parse(&text).map_err(|err| format!("unmarshal() json parsing: {err}"))
}

/// Go reports a truncated stream as `unexpected EOF`; the DEFLATE decoders
/// word it differently.
fn inflate_error(err: &std::io::Error) -> String {
    if err.kind() == std::io::ErrorKind::UnexpectedEof {
        return "unexpected EOF".to_string();
    }
    err.to_string()
}

/// The header checks `compress/zlib` performs before returning a reader.
fn check_zlib_header(data: &[u8]) -> Result<(), &'static str> {
    if data.len() < 2 {
        return Err("unexpected EOF");
    }
    let (cmf, flg) = (data[0] as u32, data[1] as u32);
    if cmf & 0x0f != 8 || cmf >> 4 > 7 || (cmf << 8 | flg) % 31 != 0 {
        return Err("zlib: invalid header");
    }
    Ok(())
}

/// Go spells a base64 failure `illegal base64 data at input byte N`.
fn base64_error(err: &base64::DecodeError) -> String {
    match err {
        base64::DecodeError::InvalidByte(offset, _)
        | base64::DecodeError::InvalidLastSymbol(offset, _) => {
            format!("illegal base64 data at input byte {offset}")
        }
        base64::DecodeError::InvalidLength(len) => {
            format!("illegal base64 data at input byte {len}")
        }
        base64::DecodeError::InvalidPadding => "illegal base64 data at input byte 0".to_string(),
    }
}
