//! Request-proof transport framing. Native adapters supply identity and signature semantics.
use crate::client::error::{Error, Result};
use sha2::{Digest, Sha256};

pub struct Protocol {
    pub domain: &'static str,
    pub scheme: &'static str,
    pub signature_bytes: usize,
    pub validate_account: fn(&str) -> Result<()>,
    pub verify: fn(&str, &[u8; 32], &[u8]) -> Result<()>,
}

fn digest(protocol: &Protocol, issued_at: u64, path: &str, body: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(format!("{}\n{issued_at}\n{path}\n", protocol.domain));
    hash.update(body);
    hash.finalize().into()
}

fn encode(protocol: &Protocol, account: &str, issued_at: u64, signature: &[u8]) -> Result<String> {
    (protocol.validate_account)(account)?;
    if signature.len() != protocol.signature_bytes {
        return Err(Error::config("Invalid native signature size"));
    }
    let signature: String = signature.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!(
        "Bearer {}.{account}.{issued_at}.{signature}",
        protocol.scheme
    ))
}

pub fn authorize(
    protocol: Protocol,
    account: &str,
    issued_at: u64,
    path: &str,
    body: &[u8],
    sign: impl FnOnce(&[u8; 32]) -> Result<Vec<u8>>,
) -> Result<String> {
    let digest = digest(&protocol, issued_at, path, body);
    let signature = sign(&digest)?;
    (protocol.verify)(account, &digest, &signature)?;
    encode(&protocol, account, issued_at, &signature)
}

/// Verify identity only. The caller independently admits the signer for this operation.
pub fn verify(
    protocol: Protocol,
    proof: &str,
    now: u64,
    path: &str,
    body: &[u8],
) -> Result<String> {
    if proof.len() > 512 {
        return Err(Error::config("Native request proof exceeds limit"));
    }
    let parts: Vec<_> = proof.split('.').collect();
    if parts.len() != 4 || parts[0] != protocol.scheme {
        return Err(Error::config("Invalid native request proof"));
    }
    let issued_at = parts[2]
        .parse::<u64>()
        .map_err(|_| Error::config("Invalid request time"))?;
    if now.abs_diff(issued_at) > 120 {
        return Err(Error::config("Native request proof is not current"));
    }
    let encoded = parts[3];
    if encoded.len() != protocol.signature_bytes * 2
        || !encoded.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(Error::config("Invalid hexadecimal identity"));
    }
    let signature = encoded
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            // The ASCII and size checks above make each pair valid UTF-8 hexadecimal.
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII hexadecimal"), 16)
                .expect("checked hexadecimal")
        })
        .collect::<Vec<_>>();
    (protocol.verify)(
        parts[1],
        &digest(&protocol, issued_at, path, body),
        &signature,
    )?;
    Ok(parts[1].into())
}
