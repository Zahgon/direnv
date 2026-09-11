//! Helper functions to calculate SubResource Integrity hashes.
//!
//! <https://developer.mozilla.org/en-US/docs/Web/Security/Subresource_Integrity>

mod parse;
mod writer;

pub use parse::parse;
pub use writer::Writer;

/// A supported hashing algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algo {
    Sha256,
    Sha384,
    Sha512,
}

impl Algo {
    /// The algorithm's name as it appears in an SRI string.
    pub fn as_str(self) -> &'static str {
        match self {
            Algo::Sha256 => "sha256",
            Algo::Sha384 => "sha384",
            Algo::Sha512 => "sha512",
        }
    }
}

/// A SRI-hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hash {
    algo: String,
    sum: Vec<u8>,
}

impl Hash {
    pub(crate) fn new(algo: String, sum: Vec<u8>) -> Hash {
        Hash { algo, sum }
    }

    /// A hex-encoded representation of the sum.
    pub fn hex(&self) -> String {
        self.sum.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl std::fmt::Display for Hash {
    /// The SRI-encoded string, `<algo>-<standard base64 of the sum>`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        write!(f, "{}-{}", self.algo, STANDARD.encode(&self.sum))
    }
}
