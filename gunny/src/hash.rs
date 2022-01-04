//! Hashing utilities.

use sha2::{Digest, Sha256};
use subtle_encoding::hex;

/// Compute the SHA256 hash of the given string and return its lowercase
/// hexadecimal representation.
pub fn sha256<S: AsRef<str>>(s: S) -> String {
    let s = s.as_ref();
    let digest = Sha256::digest(s);
    String::from_utf8(hex::encode(digest)).unwrap()
}
