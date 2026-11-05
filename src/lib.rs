#![cfg_attr(not(feature = "std"), no_std)]
//! Shared host and owner-capability primitives. Portable policy rules live in package `allowit`.

extern crate alloc;

pub mod admin;
#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "policy-source")]
pub mod policy;
#[cfg(feature = "policy-source")]
pub mod policy_source;
#[cfg(feature = "service")]
pub mod service;
#[cfg(feature = "transport")]
pub mod transport;
#[cfg(feature = "vendor")]
pub mod vendor;

/// SHA-256 of the exact bytes, as lowercase hexadecimal text.
pub fn digest(bytes: &[u8]) -> alloc::string::String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = alloc::string::String::with_capacity(64);
    for byte in hash {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 15) as usize] as char);
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn request_commitment_hashes_exact_input_bytes() {
        assert_eq!(
            super::digest(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        );
        assert_ne!(super::digest(b"abc"), super::digest(b"abc\n"));
    }
}
