//! The custom wallet build.
//!
//! A Custom wallet runs code that is built from the exact policy source. The server does not
//! build that code and does not vouch for a build. Two signed statements bind a build instead:
//!
//! 1. The server presignature. The server signs the network, the policy id and the hash of the
//!    policy source that it admitted.
//! 2. The owner attestation. The owner signs the presignature together with the hash of the
//!    wallet code that the owner built from that source.
//!
//! Both are plain text that a wallet shows before it signs. The rail of the network supplies the
//! signature scheme, the code hash and the build itself through [`CustomWalletBuild`].
//!
//! This module names no rail.
use super::{
    error::{Error, Result},
    wallet::Signer,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The first line of a server presignature message. The line is its domain and its version.
pub const PRESIGNATURE_TITLE: &str = "Allowit custom policy presignature v1";
/// The first line of an owner attestation message. The line is its domain and its version.
pub const ATTESTATION_TITLE: &str = "Allowit custom build attestation v1";
/// The longest network id, policy id or signer identity of a statement, in bytes.
pub const MAX_FIELD_BYTES: usize = 128;
/// The longest signature of a statement, in bytes.
pub const MAX_SIGNATURE_BYTES: usize = 512;

/// The stable codes of the refusals of a custom wallet build. Hosts match the code, never the
/// message.
pub mod code {
    /// No Custom policy with this id awaits a build.
    pub const UNKNOWN_POLICY: &str = "custom_policy_unknown";
    /// The presignature is not this server's statement for this network, policy and source.
    pub const PRESIGNATURE_MISMATCH: &str = "custom_presignature_mismatch";
    /// The attestation is not the owner's statement for this presignature and code hash.
    pub const ATTESTATION_INVALID: &str = "custom_attestation_invalid";
    /// The code differs from the code that the owner attested.
    pub const CODE_HASH_MISMATCH: &str = "custom_code_hash_mismatch";
    /// A tool that the build needs is missing or has another version.
    pub const TOOLCHAIN_MISSING: &str = "custom_toolchain_missing";
    /// The tools ran and the build failed.
    pub const BUILD_FAILED: &str = "custom_build_failed";
    /// The policy has no attested build yet.
    pub const BUILD_MISSING: &str = "custom_build_missing";
    /// The policy already has an attested build with another code hash.
    pub const BUILD_ATTESTED: &str = "custom_build_attested";
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
    /// Sign `message` as a wallet of this rail signs a text message.
    fn sign_message(&self, signer: &dyn Signer, message: &str) -> Result<Vec<u8>>;
    /// Verify the signature of `address` over `message`, as [`Self::sign_message`] made it.
    fn verify_message(
        &self,
        network: &str,
        address: &str,
        message: &str,
        signature: &[u8],
    ) -> Result<()>;
}

/// The server's signed statement that it admitted one policy source on one network.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Presignature {
    /// The CAIP-2 network of the policy.
    pub network: String,
    pub policy_id: String,
    /// SHA-256 of the exact policy source, as lowercase hexadecimal text.
    pub source_hash: String,
    /// The identity of the server key that signed.
    pub server: String,
    /// The server's signature over [`Presignature::message`], as lowercase hexadecimal text.
    pub signature: String,
}

/// The owner's signed statement that `code_hash` is the wallet code built from the source of
/// `presignature`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Attestation {
    pub presignature: Presignature,
    /// The identity of the wallet code, as lowercase hexadecimal text.
    pub code_hash: String,
    /// The identity of the owner key that signed.
    pub owner: String,
    /// The owner's signature over [`Attestation::message`], as lowercase hexadecimal text.
    pub signature: String,
}

/// What a verifier independently knows: the network, the policy, its exact source and the
/// server key that it trusts. Nothing here comes from the statement under verification.
#[derive(Clone, Copy, Debug)]
pub struct Admitted<'a> {
    pub network: &'a str,
    pub policy_id: &'a str,
    pub source: &'a str,
    /// The identity of the trusted server key.
    pub server: &'a str,
}

impl Presignature {
    /// The exact text that the server signs:
    ///
    /// ```text
    /// Allowit custom policy presignature v1
    /// network: <CAIP-2 network>
    /// policy: <policy id>
    /// source-sha256: <64 hexadecimal digits>
    /// server: <server identity>
    /// ```
    ///
    /// Lines end with one line feed and the last line has none.
    pub fn message(&self) -> Result<String> {
        presignature_message(
            &self.network,
            &self.policy_id,
            &self.source_hash,
            &self.server,
        )
    }
}

impl Attestation {
    /// The exact text that the owner signs. It repeats every field of the presignature and
    /// carries the server's signature, so the attestation verifies without the server:
    ///
    /// ```text
    /// Allowit custom build attestation v1
    /// network: <CAIP-2 network>
    /// policy: <policy id>
    /// source-sha256: <64 hexadecimal digits>
    /// code-sha256: <64 hexadecimal digits>
    /// server: <server identity>
    /// presignature: <the server's signature, hexadecimal>
    /// owner: <owner identity>
    /// ```
    pub fn message(&self) -> Result<String> {
        attestation_message(&self.presignature, &self.code_hash, &self.owner)
    }
}

/// The presignature message of these fields. See [`Presignature::message`].
pub fn presignature_message(
    network: &str,
    policy_id: &str,
    source_hash: &str,
    server: &str,
) -> Result<String> {
    field("network", network)?;
    field("policy id", policy_id)?;
    hash_text("source hash", source_hash)?;
    field("server identity", server)?;
    Ok(format!(
        "{PRESIGNATURE_TITLE}\nnetwork: {network}\npolicy: {policy_id}\nsource-sha256: {source_hash}\nserver: {server}"
    ))
}

/// The attestation message of a presignature, a code hash and an owner. See
/// [`Attestation::message`].
pub fn attestation_message(
    presignature: &Presignature,
    code_hash: &str,
    owner: &str,
) -> Result<String> {
    let Presignature {
        network,
        policy_id,
        source_hash,
        server,
        signature,
    } = presignature;
    field("network", network)?;
    field("policy id", policy_id)?;
    hash_text("source hash", source_hash)?;
    hash_text("code hash", code_hash)?;
    field("server identity", server)?;
    signature_bytes(signature)?;
    field("owner identity", owner)?;
    Ok(format!(
        "{ATTESTATION_TITLE}\nnetwork: {network}\npolicy: {policy_id}\nsource-sha256: {source_hash}\ncode-sha256: {code_hash}\nserver: {server}\npresignature: {signature}\nowner: {owner}"
    ))
}

/// Sign the server presignature of an admitted source. `signer` is the server key.
pub fn presign(
    hook: &dyn CustomWalletBuild,
    signer: &dyn Signer,
    network: &str,
    policy_id: &str,
    source: &str,
) -> Result<Presignature> {
    let mut presignature = Presignature {
        network: network.into(),
        policy_id: policy_id.into(),
        source_hash: crate::digest(source.as_bytes()),
        server: signer.address().into(),
        signature: String::new(),
    };
    let message = presignature.message()?;
    presignature.signature = signed(hook, signer, network, &message)?;
    Ok(presignature)
}

/// Verify a presignature against what the verifier knows. It passes only when the trusted
/// server key signed exactly this network, this policy and the hash of this source.
pub fn verify_presignature(
    hook: &dyn CustomWalletBuild,
    presignature: &Presignature,
    admitted: &Admitted<'_>,
) -> Result<()> {
    let mismatch = |what: &str| {
        refusal(
            code::PRESIGNATURE_MISMATCH,
            format!(
                "The server presignature is for another {what}. Fetch the presignature of this policy again from the server that you trust."
            ),
        )
    };
    if presignature.network != admitted.network {
        return Err(mismatch("network"));
    }
    if presignature.policy_id != admitted.policy_id {
        return Err(mismatch("policy"));
    }
    if presignature.source_hash != crate::digest(admitted.source.as_bytes()) {
        return Err(mismatch("policy source"));
    }
    if presignature.server != admitted.server {
        return Err(mismatch("server key"));
    }
    let message = presignature.message().map_err(|_| mismatch("format"))?;
    let signature = signature_bytes(&presignature.signature).map_err(|_| mismatch("format"))?;
    hook.verify_message(admitted.network, admitted.server, &message, &signature)
        .map_err(|_| {
            refusal(
                code::PRESIGNATURE_MISMATCH,
                "The trusted server key did not sign this presignature. Check the server address and its public key, then fetch the presignature again.",
            )
        })
}

/// Sign the owner attestation of a build. `signer` is the owner key. The caller verifies the
/// presignature first; this function signs what it is given.
pub fn attest(
    hook: &dyn CustomWalletBuild,
    signer: &dyn Signer,
    presignature: &Presignature,
    code_hash: &[u8; 32],
) -> Result<Attestation> {
    let mut attestation = Attestation {
        presignature: presignature.clone(),
        code_hash: hex(code_hash),
        owner: signer.address().into(),
        signature: String::new(),
    };
    let message = attestation.message()?;
    attestation.signature = signed(hook, signer, &presignature.network, &message)?;
    Ok(attestation)
}

/// Verify an attestation against what the verifier knows. It passes only when the presignature
/// passes [`verify_presignature`] and `owner` signed that presignature with the code hash.
pub fn verify_attestation(
    hook: &dyn CustomWalletBuild,
    attestation: &Attestation,
    admitted: &Admitted<'_>,
    owner: &str,
) -> Result<()> {
    verify_presignature(hook, &attestation.presignature, admitted)?;
    let invalid = |reason: &str| {
        refusal(
            code::ATTESTATION_INVALID,
            format!(
                "The build attestation is invalid: {reason}. Sign the attestation of this build again with the owner key of the policy."
            ),
        )
    };
    if attestation.owner != owner {
        return Err(invalid("another account signed it"));
    }
    let message = attestation
        .message()
        .map_err(|_| invalid("a field has an invalid format"))?;
    let signature = signature_bytes(&attestation.signature)
        .map_err(|_| invalid("the signature has an invalid format"))?;
    hook.verify_message(admitted.network, owner, &message, &signature)
        .map_err(|_| invalid("the owner key did not sign it"))
}

/// Refuse code whose hash differs from the attested hash. `code_hash` is the hash that the
/// verifier read itself: of the wallet on the network, or of the code that it is about to
/// install.
pub fn check_code_hash(attestation: &Attestation, code_hash: &[u8; 32]) -> Result<()> {
    if attestation.code_hash == hex(code_hash) {
        return Ok(());
    }
    Err(refusal(
        code::CODE_HASH_MISMATCH,
        format!(
            "The wallet code {} differs from the attested code {}. Build the policy again and deploy the code of that build.",
            hex(code_hash),
            attestation.code_hash
        ),
    ))
}

fn signed(
    hook: &dyn CustomWalletBuild,
    signer: &dyn Signer,
    network: &str,
    message: &str,
) -> Result<String> {
    let signature = hook.sign_message(signer, message)?;
    // A signer that returns a signature of another key or message is refused here, before the
    // statement leaves the signing host.
    hook.verify_message(network, signer.address(), message, &signature)
        .map_err(|_| Error::config("The signing key returned an invalid signature"))?;
    if signature.is_empty() || signature.len() > MAX_SIGNATURE_BYTES {
        return Err(Error::config(
            "The signing key returned a signature of an invalid size",
        ));
    }
    Ok(hex(&signature))
}

/// One line value: printable ASCII without a space, so no value starts another line or field.
fn field(name: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_FIELD_BYTES
        || !value.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(Error::usage(format!(
            "The {name} of a custom build statement must be 1 to {MAX_FIELD_BYTES} printable characters without a space"
        )));
    }
    Ok(())
}

fn hash_text(name: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(Error::usage(format!(
            "The {name} of a custom build statement must be 64 lowercase hexadecimal digits"
        )));
    }
    Ok(())
}

/// The bytes of a signature in lowercase hexadecimal text.
pub fn signature_bytes(text: &str) -> Result<Vec<u8>> {
    let invalid = || Error::usage("A signature must be lowercase hexadecimal text");
    if text.is_empty() || text.len() > MAX_SIGNATURE_BYTES * 2 || !text.len().is_multiple_of(2) {
        return Err(invalid());
    }
    let nibble = |b: u8| match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(invalid()),
    };
    text.as_bytes()
        .chunks(2)
        .map(|pair| Ok(nibble(pair[0])? << 4 | nibble(pair[1])?))
        .collect()
}

/// Lowercase hexadecimal text of `bytes`, the encoding of every hash and signature here.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const NETWORK: &str = "test:net";
    const SOURCE: &str = "pub fn execute() {}\n";

    /// A toy scheme for the binding tests: the signature is a hash of the address and the
    /// message. A rail supplies a real scheme.
    struct Hook;
    fn toy(address: &str, message: &[u8]) -> Vec<u8> {
        let mut hash = Sha256::new();
        hash.update(address.as_bytes());
        hash.update([0]);
        hash.update(message);
        hash.finalize().to_vec()
    }
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
        fn sign_message(&self, signer: &dyn Signer, message: &str) -> Result<Vec<u8>> {
            signer.sign_hash(message.as_bytes())
        }
        fn verify_message(&self, _: &str, address: &str, message: &str, sig: &[u8]) -> Result<()> {
            if sig == toy(address, message.as_bytes()) {
                Ok(())
            } else {
                Err(Error::usage("invalid signature"))
            }
        }
    }
    struct Key(&'static str);
    impl Signer for Key {
        fn network(&self) -> &str {
            NETWORK
        }
        fn address(&self) -> &str {
            self.0
        }
        fn sign_hash(&self, digest: &[u8]) -> Result<Vec<u8>> {
            Ok(toy(self.0, digest))
        }
    }
    const SERVER: Key = Key("SERVER");
    const OWNER: Key = Key("OWNER");
    fn admitted() -> Admitted<'static> {
        Admitted {
            network: NETWORK,
            policy_id: "policy-1",
            source: SOURCE,
            server: "SERVER",
        }
    }
    fn presignature() -> Presignature {
        presign(&Hook, &SERVER, NETWORK, "policy-1", SOURCE).unwrap()
    }
    fn code_of(error: Error) -> String {
        error.api_code.unwrap_or_default()
    }

    #[test]
    fn the_statements_have_fixed_versioned_text() {
        let presignature = presignature();
        assert_eq!(
            presignature.message().unwrap(),
            format!(
                "Allowit custom policy presignature v1\nnetwork: test:net\npolicy: policy-1\nsource-sha256: {}\nserver: SERVER",
                crate::digest(SOURCE.as_bytes())
            )
        );
        let built = Hook.build(NETWORK, SOURCE, Path::new(".")).unwrap();
        let attestation = attest(&Hook, &OWNER, &presignature, &built.code_hash).unwrap();
        assert_eq!(
            attestation.message().unwrap(),
            format!(
                "Allowit custom build attestation v1\nnetwork: test:net\npolicy: policy-1\nsource-sha256: {}\ncode-sha256: {}\nserver: SERVER\npresignature: {}\nowner: OWNER",
                crate::digest(SOURCE.as_bytes()),
                hex(&built.code_hash),
                presignature.signature
            )
        );
        verify_presignature(&Hook, &presignature, &admitted()).unwrap();
        verify_attestation(&Hook, &attestation, &admitted(), "OWNER").unwrap();
        check_code_hash(&attestation, &built.code_hash).unwrap();
        // The JSON form keeps every field and admits no other.
        let json = serde_json::to_value(&attestation).unwrap();
        assert_eq!(json["presignature"]["policyId"], "policy-1");
        assert_eq!(
            serde_json::from_value::<Attestation>(json.clone()).unwrap(),
            attestation
        );
        let mut extra = json;
        extra["accepted"] = true.into();
        assert!(serde_json::from_value::<Attestation>(extra).is_err());
    }

    #[test]
    fn a_presignature_replayed_for_another_network_or_policy_is_refused() {
        let presignature = presignature();
        for other in [
            Admitted {
                network: "test:other",
                ..admitted()
            },
            Admitted {
                policy_id: "policy-2",
                ..admitted()
            },
        ] {
            let error = verify_presignature(&Hook, &presignature, &other).unwrap_err();
            assert_eq!(code_of(error), code::PRESIGNATURE_MISMATCH);
        }
        // Relabelling the statement for the other policy breaks the server's signature.
        let mut relabelled = presignature;
        relabelled.policy_id = "policy-2".into();
        let other = Admitted {
            policy_id: "policy-2",
            ..admitted()
        };
        let error = verify_presignature(&Hook, &relabelled, &other).unwrap_err();
        assert_eq!(code_of(error), code::PRESIGNATURE_MISMATCH);
    }

    #[test]
    fn a_presignature_does_not_cover_altered_source() {
        let altered = Admitted {
            source: "pub fn execute() { /* altered */ }\n",
            ..admitted()
        };
        let error = verify_presignature(&Hook, &presignature(), &altered).unwrap_err();
        assert_eq!(code_of(error), code::PRESIGNATURE_MISMATCH);
    }

    #[test]
    fn a_presignature_from_an_untrusted_server_key_is_refused() {
        // Another key signs a well-formed presignature of the same source.
        let foreign = presign(&Hook, &Key("OTHER"), NETWORK, "policy-1", SOURCE).unwrap();
        let error = verify_presignature(&Hook, &foreign, &admitted()).unwrap_err();
        assert_eq!(code_of(error), code::PRESIGNATURE_MISMATCH);
        // It claims the trusted key but keeps its own signature.
        let mut claimed = foreign;
        claimed.server = "SERVER".into();
        let error = verify_presignature(&Hook, &claimed, &admitted()).unwrap_err();
        assert_eq!(code_of(error), code::PRESIGNATURE_MISMATCH);
    }

    #[test]
    fn an_attestation_binds_its_owner_presignature_and_code_hash() {
        let presignature = presignature();
        let built = Hook.build(NETWORK, SOURCE, Path::new(".")).unwrap();
        let attestation = attest(&Hook, &OWNER, &presignature, &built.code_hash).unwrap();
        // Another account signed.
        let error = verify_attestation(&Hook, &attestation, &admitted(), "OTHER").unwrap_err();
        assert_eq!(code_of(error), code::ATTESTATION_INVALID);
        let stranger = attest(&Hook, &Key("OTHER"), &presignature, &built.code_hash).unwrap();
        let error = verify_attestation(&Hook, &stranger, &admitted(), "OWNER").unwrap_err();
        assert_eq!(code_of(error), code::ATTESTATION_INVALID);
        // The code hash changed after the owner signed.
        let mut changed = attestation.clone();
        changed.code_hash = hex(&[7; 32]);
        let error = verify_attestation(&Hook, &changed, &admitted(), "OWNER").unwrap_err();
        assert_eq!(code_of(error), code::ATTESTATION_INVALID);
        // The presignature inside it is for another policy.
        let other = Admitted {
            policy_id: "policy-2",
            ..admitted()
        };
        let error = verify_attestation(&Hook, &attestation, &other, "OWNER").unwrap_err();
        assert_eq!(code_of(error), code::PRESIGNATURE_MISMATCH);
    }

    #[test]
    fn code_that_the_owner_did_not_attest_is_refused() {
        let built = Hook.build(NETWORK, SOURCE, Path::new(".")).unwrap();
        let attestation = attest(&Hook, &OWNER, &presignature(), &built.code_hash).unwrap();
        let other = Hook.code_hash(NETWORK, b"other code").unwrap();
        let error = check_code_hash(&attestation, &other).unwrap_err();
        assert_eq!(code_of(error), code::CODE_HASH_MISMATCH);
    }

    #[test]
    fn a_field_cannot_start_another_line_or_field() {
        for policy_id in ["", "a b", "a\nserver: OTHER", "a\r", "é"] {
            assert!(presign(&Hook, &SERVER, NETWORK, policy_id, SOURCE).is_err());
        }
        assert!(presign(&Hook, &SERVER, "test net", "policy-1", SOURCE).is_err());
        let long = "a".repeat(MAX_FIELD_BYTES + 1);
        assert!(presign(&Hook, &SERVER, NETWORK, &long, SOURCE).is_err());
        assert!(presignature_message(NETWORK, "p", &"A".repeat(64), "S").is_err());
        assert!(presignature_message(NETWORK, "p", "abc", "S").is_err());
        assert!(signature_bytes("").is_err());
        assert!(signature_bytes("abc").is_err());
        assert!(signature_bytes("AB").is_err());
        assert_eq!(signature_bytes("00ff").unwrap(), [0, 255]);
    }

    #[test]
    fn a_signer_that_returns_a_foreign_signature_signs_nothing() {
        struct Lying;
        impl Signer for Lying {
            fn network(&self) -> &str {
                NETWORK
            }
            fn address(&self) -> &str {
                "SERVER"
            }
            fn sign_hash(&self, digest: &[u8]) -> Result<Vec<u8>> {
                Ok(toy("OTHER", digest))
            }
        }
        assert!(presign(&Hook, &Lying, NETWORK, "policy-1", SOURCE).is_err());
    }
}
