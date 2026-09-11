use sha2::{Digest, Sha256, Sha384, Sha512};
use std::io;

use super::{Algo, Hash};

enum Hasher {
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
}

/// A writer that forwards the write while calculating the SRI hash.
pub struct Writer<W: io::Write> {
    w: W,
    algo: Algo,
    h: Hasher,
}

impl<W: io::Write> Writer<W> {
    /// Returns a SRI writer that forwards the write while calculating the SRI
    /// hash.
    pub fn new(w: W, algo: Algo) -> Writer<W> {
        let h = match algo {
            Algo::Sha256 => Hasher::Sha256(Sha256::new()),
            Algo::Sha384 => Hasher::Sha384(Sha384::new()),
            Algo::Sha512 => Hasher::Sha512(Sha512::new()),
        };
        Writer { w, algo, h }
    }

    /// Returns the calculated SRI hash.
    pub fn sum(self) -> Hash {
        let sum = match self.h {
            Hasher::Sha256(h) => h.finalize().to_vec(),
            Hasher::Sha384(h) => h.finalize().to_vec(),
            Hasher::Sha512(h) => h.finalize().to_vec(),
        };
        Hash::new(self.algo.as_str().to_string(), sum)
    }
}

impl<W: io::Write> io::Write for Writer<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // First write to the underlying storage.
        let n = self.w.write(buf)?;
        // This should always succeed.
        match &mut self.h {
            Hasher::Sha256(h) => h.update(buf),
            Hasher::Sha384(h) => h.update(buf),
            Hasher::Sha512(h) => h.update(buf),
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}
