//! The custom wallet build.
//!
//! A Custom wallet runs code that is built from the exact policy source. The owner's policy
//! signature binds that source, and the wallet stores its hash. The owner's client builds the
//! code from that source and hands it to the server, which uploads it when the owner deploys; an
//! upload gives no authority. The owner's one creation signature names the code by its hash, so
//! it covers the build. A client that builds compares its own build with that hash
//! ([`check_code_hash`]). The build is reproducible, so anyone can compare the code on the
//! network with the source.
//!
//! The rail of the network supplies the build and the code hash through [`CustomWalletBuild`].
//!
//! This module names no rail.
use super::error::{Error, Result};
use std::path::Path;

/// The stable codes of the refusals of a custom wallet build. Hosts match the code, never the
/// message.
pub mod code {
    /// No Custom policy with this id awaits a build.
    pub const UNKNOWN_POLICY: &str = "custom_policy_unknown";
    /// The code differs from the client's own build of the source.
    pub const CODE_HASH_MISMATCH: &str = "custom_code_hash_mismatch";
    /// A tool that the build needs is missing or has another version.
    pub const TOOLCHAIN_MISSING: &str = "custom_toolchain_missing";
    /// The tools ran and the build failed.
    pub const BUILD_FAILED: &str = "custom_build_failed";
    /// The policy has no build yet.
    pub const BUILD_MISSING: &str = "custom_build_missing";
}

/// A typed refusal of this module: a usage error that carries one of the [`code`] values.
pub fn refusal(code: &str, message: impl Into<String>) -> Error {
    let mut error = Error::usage(message);
    error.api_code = Some(code.into());
    error
}

/// The wallet code that a rail built from one policy source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalletCode {
    /// The code as the network stores it.
    pub code: Vec<u8>,
    /// The identity of `code` on the network.
    pub code_hash: [u8; 32],
    /// The tools that built it, with their versions, for the owner's record.
    pub toolchain: String,
}

/// The "build custom wallet" hook of a rail. A rail without it declares
/// [`super::plugin::Feature::CustomBuild`] skipped.
pub trait CustomWalletBuild: Sync {
    /// Build the wallet code from the exact bytes of `source`. `work` is an empty directory that
    /// the caller owns; the build may write there. A missing tool is a
    /// [`code::TOOLCHAIN_MISSING`] refusal whose message says how to install it.
    fn build(&self, network: &str, source: &str, work: &Path) -> Result<WalletCode>;
    /// The identity of wallet code as the network records it. Code that the network cannot
    /// install is an error.
    fn code_hash(&self, network: &str, code: &[u8]) -> Result<[u8; 32]>;
}

/// Refuse code whose hash differs from the hash of the client's own build of the source.
/// `code_hash` is the hash that the client is about to sign for: of the code that the server
/// uploaded, or of the wallet on the network.
pub fn check_code_hash(built: &[u8; 32], code_hash: &[u8; 32]) -> Result<()> {
    if built == code_hash {
        return Ok(());
    }
    Err(refusal(
        code::CODE_HASH_MISMATCH,
        format!(
            "The wallet code {} differs from the code {} that this client built from the policy source. Do not sign; build the policy again and compare.",
            hex(code_hash),
            hex(built)
        ),
    ))
}

/// Lowercase hexadecimal text of `bytes`, the encoding of every code hash here.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const NETWORK: &str = "test:net";
    const SOURCE: &str = "pub fn execute() {}\n";

    /// A toy build: the code is the source with a prefix, and its hash is SHA-256. A rail
    /// supplies a real build.
    struct Hook;
    impl CustomWalletBuild for Hook {
        fn build(&self, network: &str, source: &str, _: &Path) -> Result<WalletCode> {
            let code = format!("code of {source}").into_bytes();
            Ok(WalletCode {
                code_hash: self.code_hash(network, &code)?,
                code,
                toolchain: "toy 1".into(),
            })
        }
        fn code_hash(&self, _: &str, code: &[u8]) -> Result<[u8; 32]> {
            Ok(Sha256::digest(code).into())
        }
    }

    #[test]
    fn the_same_source_builds_the_same_code() {
        let a = Hook.build(NETWORK, SOURCE, Path::new(".")).unwrap();
        let b = Hook.build(NETWORK, SOURCE, Path::new(".")).unwrap();
        check_code_hash(&a.code_hash, &b.code_hash).unwrap();
    }

    #[test]
    fn code_that_the_client_did_not_build_is_refused() {
        let built = Hook.build(NETWORK, SOURCE, Path::new(".")).unwrap();
        let other = Hook.code_hash(NETWORK, b"other code").unwrap();
        let error = check_code_hash(&built.code_hash, &other).unwrap_err();
        assert_eq!(error.api_code.as_deref(), Some(code::CODE_HASH_MISMATCH));
        assert!(error.to_string().contains(&hex(&other)));
    }

    #[test]
    fn hashes_are_lowercase_hexadecimal() {
        assert_eq!(hex(&[0, 0xab, 0xff]), "00abff");
    }
}
