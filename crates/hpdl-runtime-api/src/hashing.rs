//! Shared, panic-free hashing helpers for binding hashes.
//!
//! Both [`crate::EventEnvelope`] and [`crate::ProcessReceipt`] bind their fields
//! with SHA-256 over length-prefixed bytes under a domain separator. These
//! helpers keep that encoding identical and free of panicking indexing or
//! `format!`-in-a-loop (per the crate's strict lints). Each binding type supplies
//! its *own* domain separator, so hashes in different domains never collide.

use sha2::{Digest, Sha256};

/// Length-prefixed field feed (big-endian `u64` length then bytes) so that the
/// concatenation of fields is unambiguous.
pub(crate) fn feed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

/// Map a 0–15 nibble to its lowercase hex character without panicking indexing.
fn nibble_to_hex(n: u8) -> char {
    match n & 0x0f {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        9 => '9',
        10 => 'a',
        11 => 'b',
        12 => 'c',
        13 => 'd',
        14 => 'e',
        _ => 'f',
    }
}

/// Lowercase-hex encode bytes without `format!`-in-a-loop
/// (`clippy::format_collect`) and without panicking indexing.
pub(crate) fn to_lower_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(nibble_to_hex(b >> 4));
        out.push(nibble_to_hex(*b));
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn hex_is_lowercase_and_64_chars_for_sha256() {
        let digest = Sha256::digest(b"hello");
        let hex = to_lower_hex(&digest);
        assert_eq!(hex.len(), 64);
        assert!(hex
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    }

    #[test]
    fn feed_is_length_prefixed() {
        // Two different splits of the same bytes must hash differently because of
        // the length prefix (no ambiguity across concatenations).
        let mut a = Sha256::new();
        feed(&mut a, b"ab");
        feed(&mut a, b"c");
        let mut b = Sha256::new();
        feed(&mut b, b"a");
        feed(&mut b, b"bc");
        assert_ne!(a.finalize(), b.finalize());
    }
}
