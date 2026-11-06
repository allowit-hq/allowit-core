//! Local rail communication for server callers. Private admission and key authority remain with the caller.
pub mod owner;
use crate::policy::PolicyValue;
use crate::vendor::{config::ServerAuthority, error::Result, profiles::Description};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{collections::BTreeMap, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    Simple,
    #[serde(rename = "common")]
    Standard,
    Custom,
}
pub struct PolicyInstallation {
    pub description: Description,
    pub policy_instance: [u8; 32],
    pub tier: Tier,
    pub asset: crate::vendor::input::asset::Asset,
    pub max_fee_units: u64,
}
/// Immutable common identity with private native encoding. Construction grants no signing authority.
pub struct Installation {
    pub description: Description,
    pub config: Box<RawValue>,
    pub export: Option<Box<RawValue>>,
    pub authority: Option<ServerAuthority>,
}
impl Installation {
    pub fn description(&self) -> &Description {
        &self.description
    }
    /// Preserve the existing native installation record when the caller persists its own state.
    pub fn native_document(&self) -> &[u8] {
        self.config.get().as_bytes()
    }
}
#[derive(Clone, Debug)]
pub enum Action {
    Transfer {
        recipient: String,
        units: i128,
    },
    Deposit {
        vault: String,
        assets: i128,
        min_shares: i128,
    },
    Redeem {
        vault: String,
        shares: i128,
        min_assets: i128,
    },
}
#[derive(Clone, Debug, Default)]
pub struct Terms {
    pub args: BTreeMap<String, PolicyValue>,
    pub expense: Option<Expense>,
}
#[derive(Clone, Debug)]
pub struct Expense {
    pub gas_units: i128,
    pub conversion_units: i128,
    pub spent_units: i128,
    pub max_fee_units: u64,
    pub observed_height: u64,
    pub day: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Accounting {
    pub nonce: u64,
    #[serde(alias = "revision")]
    pub deployment_version: u64,
    pub spent_daily: i128,
    pub spent_total: i128,
    pub spent_day: u64,
    pub calls: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct Validity {
    pub now: u64,
    pub expires_at: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Prepared,
    Submitted,
    SubmissionUnknown,
    Confirmed,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    #[serde(rename = "signer")]
    pub identity: String,
    #[serde(rename = "signature")]
    pub bytes: Vec<u8>,
}
/// The caller owns the private key and decides when this capability is available.
pub trait Signer {
    fn identity(&self) -> &str;
    fn sign_hash(&self, digest: &[u8; 32]) -> Result<Vec<u8>>;
}
/// Existing portable Core evaluator. The server supplies only its admitted compiled policy.
pub type Evaluator<'a> = &'a dyn Fn(&allowit::AllowitContext) -> allowit::Result;

#[derive(Clone, Copy)]
pub enum Kind {
    Prepared,
    Signed,
    Receipt,
    Owner,
    OwnerSigned,
    Request,
}
/// A native document with immutable identity and provider-owned state.
/// Callers cannot replace its bytes while retaining an authorization capability.
///
/// ```compile_fail
/// use allowit_core::vendor::runtime::Packet;
/// fn replace(packet: &mut Packet) { packet.network = "other:network".into(); }
/// ```
#[derive(Clone)]
pub struct Packet {
    provider: &'static dyn PacketProvider,
    network: String,
    document: Box<RawValue>,
    capability: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
}
impl Packet {
    pub fn network(&self) -> &str {
        &self.network
    }
    pub fn document(&self) -> &RawValue {
        &self.document
    }
    pub fn provider(&self) -> &'static dyn PacketProvider {
        self.provider
    }
    /// Read only the owning provider's private capability type.
    pub fn capability<T: std::any::Any>(&self) -> Option<&T> {
        self.capability.as_ref()?.downcast_ref::<T>()
    }
    /// Attach native in-memory state without changing this packet's retained bytes.
    pub fn with_capability<T: std::any::Any + Send + Sync>(mut self, value: T) -> Self {
        self.capability = Some(std::sync::Arc::new(value));
        self
    }
    /// Retain literal native bytes. The adapter validates them before it returns a public carrier.
    pub fn from_raw(
        network: &str,
        document: Box<RawValue>,
        provider: &'static dyn PacketProvider,
    ) -> Self {
        Self {
            network: network.into(),
            document,
            provider,
            capability: None,
        }
    }

    pub fn new(
        network: &str,
        value: &impl Serialize,
        provider: &'static dyn PacketProvider,
    ) -> Result<Self> {
        let document = RawValue::from_string(
            serde_json::to_string(value)
                .map_err(|_| crate::client::error::Error::config("Cannot encode native packet"))?,
        )
        .map_err(|_| crate::client::error::Error::config("Cannot retain native packet"))?;
        Ok(Self {
            network: network.into(),
            provider,
            document,
            capability: None,
        })
    }
}
macro_rules! carrier {
    ($name:ident,$kind:ident) => {
        #[derive(Clone, Debug)]
        pub struct $name(pub Packet);
        impl $name {
            pub fn bytes(&self) -> &[u8] {
                self.0.document.get().as_bytes()
            }
            pub fn network(&self) -> &str {
                &self.0.network
            }
        }
        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(
                &self,
                s: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                self.0.document.serialize(s)
            }
        }
    };
}
carrier!(Prepared, Prepared);
carrier!(Signed, Signed);
carrier!(Receipt, Receipt);
carrier!(OwnerPacket, Owner);
carrier!(OwnerSigned, OwnerSigned);
carrier!(RequestPacket, Request);
impl RequestPacket {
    pub fn operation_id(&self) -> Result<[u8; 32]> {
        self.0.provider.request_id(&self.0)
    }
}
impl PartialEq for RequestPacket {
    fn eq(&self, other: &Self) -> bool {
        self.network() == other.network()
            && match (
                serde_json::from_slice::<serde_json::Value>(self.bytes()),
                serde_json::from_slice::<serde_json::Value>(other.bytes()),
            ) {
                (Ok(left), Ok(right)) => left == right,
                _ => false,
            }
    }
}

/// Native owner-signing input and common immutable transaction identity.
#[derive(Clone, Debug)]
pub struct OwnerInfo {
    pub action: String,
    pub signing_payload: Vec<u8>,
    pub transaction_hash: [u8; 32],
    pub expires_at: u64,
}
impl OwnerPacket {
    pub fn info(&self) -> Result<OwnerInfo> {
        self.0.provider.owner_info(&self.0)
    }
}
#[derive(Clone, Debug)]
pub struct PacketInfo {
    pub operation_id: [u8; 32],
    pub transaction_hash: [u8; 32],
    pub wallet: String,
    pub expires_at: u64,
}
/// Read-only common projection of a validated retained operation.
pub struct PreparedMetadata {
    pub configuration: owner::Configuration,
    pub request: RequestPacket,
    pub operation_id: [u8; 32],
    pub created_at: u64,
    pub expires_at: u64,
    pub transaction_hash: [u8; 32],
    pub debit_units: i128,
    pub request_hash: [u8; 32],
    pub debit: Debit,
}
#[derive(Clone, Debug)]
pub struct Debit {
    pub units: i128,
    pub token: String,
    pub recipient: String,
    pub kind: String,
    pub display_amount: String,
    pub display_token: String,
}
impl Prepared {
    pub fn metadata(&self) -> Result<PreparedMetadata> {
        self.0.provider.prepared_metadata(&self.0)
    }
    pub fn info(&self) -> Result<PacketInfo> {
        self.0.provider.info(&self.0, Kind::Prepared)
    }
}
impl Signed {
    pub fn validate_signatures(&self) -> Result<()> {
        self.0.provider.validate_signed(&self.0)
    }
    pub fn preparation(&self) -> Result<Prepared> {
        self.0.provider.signed_preparation(&self.0)
    }
    pub fn info(&self) -> Result<PacketInfo> {
        self.0.provider.info(&self.0, Kind::Signed)
    }
}
/// Values observed by the selected native implementation. This is not a complete economic-state commitment.
#[derive(Clone)]
pub struct Snapshot {
    pub accounting: Accounting,
    pub observed_height: u64,
    pub paused: bool,
    pub params: BTreeMap<String, PolicyValue>,
    pub state: BTreeMap<String, PolicyValue>,
    pub evidence: Packet,
}
impl Snapshot {
    pub fn verify_consent(&self, message: &str, signatures: &[Signature]) -> Result<()> {
        self.evidence
            .provider
            .verify_consent(self, message, signatures)
    }
    pub fn signer_weight(&self, identity: &str) -> Result<u32> {
        self.evidence.provider.signer_weight(self, identity)
    }
    pub fn authority_document(&self) -> Result<serde_json::Value> {
        self.evidence.provider.authority_document(self)
    }
    /// Compare only native known-state evidence. Current balance, lifetime and validity checks stay separate.
    pub fn same_known_state(&self, other: &Self) -> Result<bool> {
        self.evidence.provider.same_snapshot(self, other)
    }
}
#[derive(Clone, Debug)]
pub struct Lifetime {
    pub observed_height: u64,
    pub instance_until: u64,
    pub code_until: u64,
    pub extension_target: u64,
    pub needs_renewal: bool,
}
#[derive(Clone, Debug)]
pub struct Resources {
    pub code_present: bool,
    pub wallet_present: bool,
    pub lifetime: Option<Lifetime>,
}
#[derive(Clone, Debug)]
pub struct ReceiptInfo {
    pub transaction_hash: [u8; 32],
    pub status: Status,
    pub observed_height: u64,
}
impl Receipt {
    pub fn verify_operation(&self, signed: &Signed) -> Result<()> {
        self.0.provider.verify_receipt(&self.0, &signed.0)
    }
    pub fn info(&self) -> Result<ReceiptInfo> {
        self.0.provider.receipt_info(&self.0)
    }
}
/// Atomically reject a different signed packet under an existing operation ID.
pub trait OperationStore {
    fn load(&self, id: [u8; 32]) -> Result<Option<(Signed, Status)>>;
    fn record(&mut self, id: [u8; 32], signed: &Signed, status: Status) -> Result<()>;
}
pub struct ObservedAuthorization {
    pub signed: Signed,
    pub receipt: Receipt,
    pub status: Status,
}
#[derive(Clone, Debug)]
pub enum OwnerAction {
    Upload {
        wasm: Vec<u8>,
    },
    Create {
        wasm: Vec<u8>,
    },
    Fund {
        units: i128,
    },
    Extend,
    SetPause {
        paused: bool,
    },
    Recover {
        units: Option<i128>,
        recipient: Option<String>,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct Poll {
    pub attempts: u32,
    pub interval: Duration,
}
impl Default for Poll {
    fn default() -> Self {
        Self {
            attempts: 30,
            interval: Duration::from_secs(2),
        }
    }
}

/// Caller-owned bounded RPC transport. Native methods and packet interpretation stay in the vendor.
pub trait Transport: Send + Sync {
    fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value>;
}

/// Public failure information. It grants no execution or settlement authority.
#[derive(Clone, Debug)]
pub struct Problem {
    pub status: u16,
    pub code: &'static str,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl std::fmt::Debug for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Packet")
            .field("network", &self.network)
            .field("document", &self.document)
            .finish_non_exhaustive()
    }
}
/// Native projections validate retained documents before exposing identities or effects.
pub trait PacketProvider: Send + Sync {
    fn request_id(&self, p: &Packet) -> Result<[u8; 32]>;
    fn owner_info(&self, p: &Packet) -> Result<OwnerInfo>;
    fn prepared_metadata(&self, p: &Packet) -> Result<PreparedMetadata>;
    fn info(&self, p: &Packet, k: Kind) -> Result<PacketInfo>;
    fn validate_signed(&self, p: &Packet) -> Result<()>;
    fn signed_preparation(&self, p: &Packet) -> Result<Prepared>;
    fn verify_consent(&self, s: &Snapshot, message: &str, signatures: &[Signature]) -> Result<()>;
    fn signer_weight(&self, s: &Snapshot, identity: &str) -> Result<u32>;
    fn authority_document(&self, s: &Snapshot) -> Result<serde_json::Value>;
    fn same_snapshot(&self, a: &Snapshot, b: &Snapshot) -> Result<bool>;
    fn verify_receipt(&self, r: &Packet, p: &Packet) -> Result<()>;
    fn receipt_info(&self, p: &Packet) -> Result<ReceiptInfo>;
}

/// A diagnostic provider and its operation-local error classifier.
pub struct DiagnosticSession {
    pub provider: Box<dyn super::RuntimeProvider>,
    pub classify: Box<dyn FnOnce(&crate::client::error::Error) -> Problem>,
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Provider;
    static PROVIDER: Provider = Provider;
    impl PacketProvider for Provider {
        fn request_id(&self, p: &Packet) -> Result<[u8; 32]> {
            use sha2::{Digest, Sha256};
            Ok(Sha256::digest(p.document.get().as_bytes()).into())
        }
        fn owner_info(&self, _: &Packet) -> Result<OwnerInfo> {
            unreachable!()
        }
        fn prepared_metadata(&self, _: &Packet) -> Result<PreparedMetadata> {
            unreachable!()
        }
        fn info(&self, _: &Packet, _: Kind) -> Result<PacketInfo> {
            unreachable!()
        }
        fn validate_signed(&self, _: &Packet) -> Result<()> {
            unreachable!()
        }
        fn signed_preparation(&self, _: &Packet) -> Result<Prepared> {
            unreachable!()
        }
        fn verify_consent(&self, _: &Snapshot, _: &str, _: &[Signature]) -> Result<()> {
            unreachable!()
        }
        fn signer_weight(&self, _: &Snapshot, _: &str) -> Result<u32> {
            unreachable!()
        }
        fn authority_document(&self, _: &Snapshot) -> Result<serde_json::Value> {
            unreachable!()
        }
        fn same_snapshot(&self, _: &Snapshot, _: &Snapshot) -> Result<bool> {
            unreachable!()
        }
        fn verify_receipt(&self, _: &Packet, _: &Packet) -> Result<()> {
            unreachable!()
        }
        fn receipt_info(&self, _: &Packet) -> Result<ReceiptInfo> {
            unreachable!()
        }
    }
    #[test]
    fn retained_document_keeps_exact_bytes_and_opaque_state_stays_in_memory() {
        let raw = r#"{ "network" : "example:test", "requestId" : "first" }"#;
        let packet = RequestPacket(
            Packet::from_raw(
                "example:test",
                RawValue::from_string(raw.into()).unwrap(),
                &PROVIDER,
            )
            .with_capability(String::from("private capability")),
        );
        let cloned = packet.clone();
        assert_eq!(packet.bytes(), raw.as_bytes());
        assert_eq!(serde_json::to_string(&packet).unwrap(), raw);
        assert_eq!(
            packet.operation_id().unwrap(),
            cloned.operation_id().unwrap()
        );
        assert!(std::ptr::eq(
            packet.0.capability::<String>().unwrap(),
            cloned.0.capability::<String>().unwrap(),
        ));
        assert!(
            !serde_json::to_string(&packet)
                .unwrap()
                .contains("capability")
        );
        assert_ne!(
            packet.operation_id().unwrap(),
            RequestPacket(
                Packet::new(
                    "example:test",
                    &serde_json::json!({"requestId":"other"}),
                    &PROVIDER
                )
                .unwrap()
            )
            .operation_id()
            .unwrap()
        );
    }
    fn request_document(raw: &str) -> RequestPacket {
        RequestPacket(Packet::from_raw(
            "example:test",
            RawValue::from_string(raw.into()).unwrap(),
            &PROVIDER,
        ))
    }
    #[test]
    fn request_equality_preserves_large_integer_identity() {
        for (left, right) in [
            ("18446744073709551616", "18446744073709551617"),
            ("-9223372036854775809", "-9223372036854775810"),
        ] {
            assert_ne!(
                request_document(&format!("{{\"amount\":{left}}}")),
                request_document(&format!("{{\"amount\":{right}}}"))
            );
        }
        assert_eq!(
            request_document(r#"{ "amount": 1, "id": "a" }"#),
            request_document(r#"{"id":"a","amount":1}"#)
        );
    }
    #[test]
    fn request_equality_never_equates_parse_failures() {
        let nested = format!("{}0{}", "[".repeat(150), "]".repeat(150));
        let other = format!("{}1{}", "[".repeat(150), "]".repeat(150));
        assert_ne!(request_document(&nested), request_document(&other));
    }
}
