use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use super::{Algo, Hash};

/// Parse a SRI hash.
pub fn parse(sri_hash: &str) -> Result<Hash, String> {
    let Some((algo_name, encoded)) = sri_hash.split_once('-') else {
        return Err(format!("sri: not a hash {sri_hash}"));
    };

    let algo = match algo_name {
        "sha256" => Algo::Sha256,
        "sha384" => Algo::Sha384,
        "sha512" => Algo::Sha512,
        other => return Err(format!("sri: unsupported algo {other}")),
    };

    let sum = STANDARD
        .decode(encoded)
        .map_err(|err| illegal_base64(&err))?;

    Ok(Hash::new(algo.as_str().to_string(), sum))
}

fn illegal_base64(err: &base64::DecodeError) -> String {
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
