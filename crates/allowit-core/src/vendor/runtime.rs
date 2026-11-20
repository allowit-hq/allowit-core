//! Local rail communication for server callers.
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
/// Immutable common identity with private native encoding.
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
    /// Native units that the wallet repaid above its own shares of the network fees of its
    /// earlier confirmed operations ([`Fee::repaid_native_units`]). The server keeps one carry for
    /// each wallet: it gives the carry to `RuntimeProvider::settle_receipt` and stores the
    /// [`Settlement::gas_carry`] that the rail returns. A rail whose wallet repays the network fee
    /// subtracts the carry from the gas that this operation repays, down to zero; the wallet
    /// counts what it repays. Zero for a wallet that repays no gas.
    pub gas_carry_units: u128,
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
/// The server's account of a wallet's standard figures, from the operations that settled. A
/// wallet that keeps a figure itself reads it from the network; the server's account gives the
/// others. The figures are those of `allowit::AllowitStandardState`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Accounting {
    pub nonce: u64,
    #[serde(alias = "revision")]
    pub deployment_version: u64,
    /// Irreversible expenses on `spent_day` (`day_spent`).
    pub spent_daily: i128,
    /// Lifetime irreversible expenses (`spent`).
    pub spent_total: i128,
    /// The UTC day of the daily figures (`day`).
    pub spent_day: u64,
    /// Gross amounts plus separate attributable gas on `spent_day` (`day_total`).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub volume_daily: i128,
    /// An operation lacked price facts in the wallet's lifetime (`unpriced`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub unpriced_total: bool,
    /// An operation on `spent_day` lacked price facts (`day_unpriced`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub unpriced_daily: bool,
    /// Successful operations (`calls`).
    pub calls: u64,
}
fn is_zero(value: &i128) -> bool {
    *value == 0
}
fn is_false(value: &bool) -> bool {
    !*value
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
    /// The signature of the exact `bytes`, for a rail whose transaction signature covers the
    /// whole serialized message, not a 32-byte hash of it. The signer signs the bytes as they
    /// are: it adds no prefix and hashes nothing. A signer that signs only hashes keeps the
    /// default, which refuses.
    fn sign_bytes(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let _ = bytes;
        Err(crate::client::error::Error::unsupported(
            "This signer signs only 32-byte hashes",
        ))
    }
}
/// The server's own accounts that send contract-wallet operations and code uploads and pay their
/// network fees. The server holds their keys; a rail signs each transaction that one of them
/// sends through this capability. A sending account sends one transaction of a ledger at a time,
/// so a server keeps several and hands out the next free one.
pub trait SendingAccounts: Send + Sync {
    /// The native identity of the account that sends the next transaction.
    fn account(&self) -> Result<String>;
    /// The signature of `hash` by the key of `account`, as the rail's transaction signature.
    fn sign(&self, account: &str, hash: &[u8; 32]) -> Result<Vec<u8>>;
    /// The signature of the exact `bytes` by the key of `account`, for a rail whose transaction
    /// signature covers the whole serialized message. Same rules as [`Signer::sign_bytes`]; the
    /// default refuses.
    fn sign_bytes(&self, account: &str, bytes: &[u8]) -> Result<Vec<u8>> {
        let _ = (account, bytes);
        Err(crate::client::error::Error::unsupported(
            "These sending accounts sign only 32-byte hashes",
        ))
    }
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

/// An unsigned owner step: the rail's signing input, its send cutoff, and `transaction_hash`, a
/// 32-byte digest by which the rail names the unsigned step. A caller keeps a signed step under
/// [`OwnerSignedInfo::identity`], in the rail's own bytes, which a rail whose identity is a
/// signature knows only once the owner signed.
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
/// The identity of an owner-signed step ([`OwnerSigned::info`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerSignedInfo {
    /// The rail's own action name of the step, as [`OwnerInfo::action`] names it.
    pub action: String,
    /// The identity of the step's transaction, in the rail's own bytes and length (a hash, a
    /// signature): the key under which a caller keeps the sent step. A rail whose transaction
    /// identity is its signature knows it only once the owner signed.
    pub identity: Vec<u8>,
    /// The step's send cutoff in Unix seconds: a caller sends the step only while its clock is
    /// before it (`RuntimeProvider::validate_owner_signed`). It is no proof of a lapse. A
    /// network decides by its own clock or height, so it can still apply a sent step at or just
    /// after this second, and a rail whose validity ends at a height states here the second up
    /// to which it lets a caller send. Only `RuntimeProvider::owner_step_lapsed` proves that a
    /// sent step can no longer apply.
    pub expires_at: u64,
}
impl OwnerSigned {
    pub fn info(&self) -> Result<OwnerSignedInfo> {
        self.0.provider.owner_signed_info(&self.0)
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
    /// The operation signs a payment authorization that the seller submits: the server sends
    /// nothing, and another party's transaction settles it. `transaction_hash` then
    /// identifies the authorization, not a transaction.
    pub seller_settles: bool,
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
impl Resources {
    /// Refuse a contract wallet that is not live: a wallet is live while the network holds both
    /// its instance and its code. Without either, it takes no operation or owner step, and the
    /// refusal is the typed condition [`CONTRACT_EXPIRED`], with no HTTP status
    /// (`Error::coded`): a server maps it to its own status, and the caller's workflow says what
    /// it signed or sent. A keypair wallet has no code; its host does not ask.
    pub fn require_live(&self) -> Result<()> {
        if self.wallet_present && self.code_present {
            return Ok(());
        }
        Err(crate::client::error::Error::coded(
            CONTRACT_EXPIRED,
            "The wallet's instance or its code lapsed on its network.",
        ))
    }
}
#[derive(Clone, Debug)]
pub struct ReceiptInfo {
    pub transaction_hash: [u8; 32],
    pub status: Status,
    pub observed_height: u64,
    pub fee: Fee,
    pub debits: Vec<Debit>,
    /// The transaction that settled the operation, when another party submitted it: the
    /// settlement of a payment authorization. `transaction_hash` stays the identity of the
    /// retained packet. `None` when the retained packet is the settled transaction.
    pub settlement_hash: Option<[u8; 32]>,
}
/// The fee a receipt paid. `charged_units` is in installed-asset units and counts against
/// spending and volume limits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fee {
    /// The network fee that the ledger charged the transaction's source.
    pub native_units: u128,
    pub charged_units: i128,
    /// The native units that the wallet repaid to the transaction's source for the network fee
    /// of the operation in the same transaction: the repayment of this operation, not of the
    /// other operations that the transaction carries. A receipt packet is the receipt of one
    /// Allowit operation ([`Receipt::verify_operation`]): a rail whose transaction carries
    /// several Allowit operations makes one receipt packet for each of them, so that
    /// [`Receipt::info`] reads that operation's own repayment. The network's own parts of one
    /// Allowit operation or owner step share its one receipt. Zero for a failed transaction and
    /// for a
    /// wallet that repays no gas. A confirmed operation moves its wallet's gas carry by this
    /// amount less the operation's own share of `native_units`, which the rail allocates (all of
    /// it when the transaction carries one operation), down to zero; [`Settlement::gas_carry`]
    /// gives the result.
    pub repaid_native_units: u128,
}
/// What one executed operation adds to its wallet's figures, as the wallet counts it (Core's
/// standard post-flight): `spent_units` to the spent totals of the day and of the lifetime,
/// `amount_units` and `gas_units` to the day's gross volume, one call, and the unpriced mark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub amount_units: i128,
    pub gas_units: i128,
    pub spent_units: i128,
    pub unpriced: bool,
}
/// What a receipt means for the one operation that the settlement is about, whatever else the
/// receipt's transaction carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationOutcome {
    /// The operation executed in the receipt's transaction: its effects and its figures count.
    Executed,
    /// The operation can no longer execute: it failed in the receipt's transaction, or that
    /// transaction consumed it without executing it. It is over and counts no effect.
    Failed,
    /// The operation did not execute in the receipt's transaction, and another transaction can
    /// still carry it, as another envelope of a contract wallet's authorization. The caller keeps
    /// the operation and its reservation, looks for its other envelopes
    /// (`RuntimeProvider::observe_authorizations`) and for its lapse (`RuntimeProvider::lapsed`),
    /// and releases it only on a settlement whose outcome is `Executed` or `Failed`, or on its
    /// lapse. The fee of this settlement (`Settlement::fee_units`) is charged once for each
    /// operation and `transaction`, and the reservation stays whole until the final settlement.
    Open,
}
/// What one receipt changes for one operation: its outcome, and the figures that a server keeps
/// for the wallet beside those that the wallet keeps itself. The caller stores each value as the
/// rail gives it and applies one settlement once for each operation and `transaction`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settlement {
    /// The identity of the transaction that the receipt shows, in the rail's own bytes and length
    /// (a hash, a signature): for a payment authorization that another party settles, that
    /// party's transaction.
    pub transaction: Vec<u8>,
    pub outcome: OperationOutcome,
    /// What the operation adds to its wallet's figures, as the wallet counts it ([`Usage`], its
    /// gas included), for an executed operation of a rail that counts it; `None` otherwise. The
    /// caller counts what [`Settlement::usage_for`] gives, never this field alone.
    pub usage: Option<Usage>,
    /// The network fee in native units that this operation's wallet is charged for the receipt's
    /// transaction, as the rail defines it: zero when a settlement service or another party bears
    /// the transaction's fee and the wallet pays none of it; what the wallet repaid for this
    /// operation when it repays the party that paid the fee; for a transaction that carries
    /// several operations, this operation's own share or repayment. It is the only fee figure
    /// that a caller prices.
    pub fee_units: u128,
    /// The gas that the wallet counted itself for this operation in the receipt's transaction, in
    /// installed-asset units, for a wallet that counts its own gas: zero when it counted none, as
    /// when its gas carry covered the repayment or the operation did not execute. It is a report:
    /// the caller charges no fee of its own for this operation.
    ///
    /// `None` for a wallet that counts no gas itself, as a keypair wallet: the caller prices
    /// `fee_units` with `ServerHooks::price_fee` (the wallet's asset) and charges the price to the
    /// figures that it keeps with `accounting::charge_fee`, once for each operation and
    /// `transaction`, in the UTC day in which the operation was evaluated. A settlement whose
    /// outcome is `Executed` or `Failed` is final: figures that hold a reserved fee bound for this
    /// operation, as a keypair wallet's policy state does when its policy counts the fee, take the
    /// price less that reservation (a negative difference releases the unused part), and figures
    /// without a reservation take the price. An `Open` settlement charges the price to every
    /// figure that it reaches and leaves the reservation whole for the final settlement.
    pub counted_gas: Option<i128>,
    /// The nonce of the wallet's next operation, for a wallet whose server keeps its replay state
    /// ([`Accounting::nonce`]): the nonce of `accounting` plus one after an executed operation,
    /// and the nonce of `accounting` otherwise. The rail refuses the receipt of such a wallet when
    /// the call gives no `accounting`. `None` for a wallet that refuses a replay by itself, as a
    /// contract wallet, whatever `accounting` is.
    pub next_nonce: Option<u64>,
    /// The wallet's gas carry after this operation ([`Terms::gas_carry_units`]), in native units:
    /// after an executed operation, the `carry` that the call gave plus what the operation repaid
    /// less its own share of the transaction's network fee, down to zero, as the rail allocates
    /// that fee; otherwise the `carry` that the call gave. The caller stores it as the wallet's
    /// carry and computes no carry of its own. A rail whose wallets repay no gas returns the carry
    /// that it took.
    pub gas_carry: u128,
}
impl Settlement {
    /// What the operation adds to the figures that a server keeps for its wallet, with
    /// `debit_units`, the operation's debit (`PreparedMetadata::debit_units`): `None` for an
    /// operation that did not execute; for an executed one, the rail's `usage`, or, when the
    /// receipt reports none, the debit as its spent units, with no gross amount, no gas and no
    /// unpriced mark. The caller adds it with Core's post-flight in the UTC day in which the
    /// operation was evaluated: the day whose limits admitted it.
    pub fn usage_for(&self, debit_units: i128) -> Option<Usage> {
        if self.outcome != OperationOutcome::Executed {
            return None;
        }
        Some(self.usage.unwrap_or(Usage {
            amount_units: 0,
            gas_units: 0,
            spent_units: debit_units,
            unpriced: false,
        }))
    }
}
/// The fee ceiling of an operation and the facts of its admission, as the server gives them to
/// the rail's expense bound (`RuntimeProvider::expense`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeeCeiling {
    /// The fee ceiling in the network's native units.
    pub max_fee_units: u128,
    /// The server's price of that ceiling in installed-asset units (`ServerHooks::price_fee` of
    /// the wallet's asset).
    pub gas_units: i128,
    /// The ledger height that the server's admission read observed.
    pub observed_height: u64,
    /// The UTC day of the evaluation (`accounting::day`).
    pub day: u64,
}
/// The expense bound of one operation that its wallet counts against the spending limits, as a
/// rail gives it (`RuntimeProvider::expense`). Amounts are in installed-asset units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpenseBound {
    /// The priced fee ceiling (`FeeCeiling::gas_units`).
    pub gas_units: i128,
    /// The worst-case conversion loss of a vault operation; zero for a transfer.
    pub conversion_units: i128,
    /// The principal plus `gas_units`: a transfer's principal is its amount, a vault operation's
    /// is its `conversion_units`.
    pub spent_units: i128,
    /// The fee ceiling in native units, as `FeeCeiling::max_fee_units` gives it.
    pub max_fee_units: u128,
    /// `FeeCeiling::observed_height`.
    pub observed_height: u64,
    /// `FeeCeiling::day`.
    pub day: u64,
}
impl Receipt {
    pub fn verify_operation(&self, signed: &Signed) -> Result<()> {
        self.0.provider.verify_receipt(&self.0, &signed.0)
    }
    pub fn info(&self) -> Result<ReceiptInfo> {
        self.0.provider.receipt_info(&self.0)
    }
    /// When the receipt's operation settled, in Unix seconds, or `None` when the rail does not
    /// state it ([`PacketProvider::receipt_settled_at`]).
    pub fn settled_at(&self) -> Result<Option<u64>> {
        self.0.provider.receipt_settled_at(&self.0)
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
/// An owner action on an installation. A rail prepares each as one owner transaction with
/// `RuntimeProvider::prepare_owner`.
#[derive(Clone, Debug)]
pub enum OwnerAction {
    /// Create the wallet with the code `wasm` and fund it in the same transaction: `units` of
    /// the installed asset, and `fee_asset_units` of the network's fee asset, from which the
    /// wallet repays the network fee of its operations, when the installed asset is another
    /// asset. The network holds the code already: the server uploads it with the transaction
    /// of `RuntimeProvider::prepare_code_within`, and an upload gives no authority.
    Create {
        wasm: Vec<u8>,
        units: i128,
        fee_asset_units: i128,
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
/// The owner action that changes installed params on an active wallet. A rail prepares its
/// owner step with `RuntimeProvider::prepare_params_update`: it writes the next configuration
/// revision and keeps the policy source, the recorded usage and the request nonce.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParamsUpdate {
    /// The policy that the installation binds.
    pub policy_id: String,
    /// The configuration revision that the owner reviewed. The rail refuses the update when
    /// the wallet is at another revision. `None` accepts the revision the rail reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u64>,
    /// Each changed param with its complete new value. Other params keep their values.
    pub changes: BTreeMap<String, PolicyValue>,
}
/// The storage type name of a value, as the canonical function metadata names it.
fn type_name(value: &allowit::AllowitValue) -> &'static str {
    use allowit::AllowitValue as V;
    match value {
        V::Int(_) => "Int",
        V::Bool(_) => "Bool",
        V::Str(_) => "Str",
        V::Addr(_) => "Addr",
        V::List(_) => "List",
    }
}
/// The storage types that the canonical function of this param declares, if it is canonical.
fn declared_types(key: &str) -> Option<Vec<String>> {
    crate::policy::canonical_function_registry()
        .ok()?
        .functions
        .into_iter()
        .find(|function| function.name == key)
        .map(|function| function.parameter.storage_types)
}
impl ParamsUpdate {
    /// Most params that one update may change.
    pub const MAX_CHANGES: usize = 32;
    /// The installed params with this update applied. An update changes installed params
    /// only: it adds no param, removes none and keeps the type of each value. The rail and
    /// the backend check each value against the param's range.
    pub fn apply(
        &self,
        installed: &BTreeMap<String, PolicyValue>,
    ) -> Result<BTreeMap<String, PolicyValue>> {
        use crate::client::error::Error;
        if self.changes.is_empty() || self.changes.len() > Self::MAX_CHANGES {
            return Err(Error::usage(
                "A params update needs one to 32 changed params",
            ));
        }
        let mut next = installed.clone();
        let mut changed = false;
        for (key, value) in &self.changes {
            let current = next.get_mut(key).ok_or_else(|| {
                Error::usage(format!("The installed policy has no param `{key}`"))
            })?;
            // A param keeps its type. A canonical param that is declared with more than one
            // storage type, such as a wildcard or a list, keeps any of its declared types.
            if core::mem::discriminant(&current.0) != core::mem::discriminant(&value.0)
                && !declared_types(key).is_some_and(|types| {
                    [&current.0, &value.0]
                        .iter()
                        .all(|v| types.iter().any(|t| t == type_name(v)))
                })
            {
                return Err(Error::usage(format!(
                    "The new value of `{key}` has another type than the installed value"
                )));
            }
            changed |= current != value;
            *current = value.clone();
        }
        if !changed {
            return Err(Error::usage("The update changes no installed param"));
        }
        Ok(next)
    }
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

/// The problem code of an operation or owner step on a wallet whose instance or code lapsed on
/// its network. A rail refuses to prepare such an operation or step, before anything is signed
/// ([`Resources::require_live`]). At the send of a signed owner step, a rail refuses with this
/// code the steps that it checks for liveness (`RuntimeProvider::validate_owner_signed`): the
/// refusal is of that attempt, and a later check passes once the wallet is live again. A signed
/// step that the rail does not check, because it calls the wallet, can be sent after the wallet
/// lapsed; the network then fails it, which is the step's receipt and not this code. Nothing in
/// Allowit restores a wallet: its owner keeps it live, or renews or restores it outside Allowit.
pub const CONTRACT_EXPIRED: &str = "contract_expired";

/// The problem code of an owner step at or past its send cutoff
/// ([`OwnerSignedInfo::expires_at`]): the rail refuses to send it, for good. The code says
/// nothing about the network. A step that was possibly sent before can still apply, and it ends
/// only on its receipt or on the proof of its lapse (`RuntimeProvider::owner_step_lapsed`); a
/// step that its caller knows it never sent is dropped.
pub const OWNER_TRANSACTION_EXPIRED: &str = "owner_transaction_expired";

/// The problem code of an owner step's preparation whose reads of the ledger crossed more than
/// one ledger: nothing was signed or retained, and the caller prepares the step once more with
/// the same inputs.
pub const STALE_OWNER_OBSERVATION: &str = "stale_owner_observation";

/// Public failure information.
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
    /// When the operation of the verified receipt `p` settled, in Unix seconds: the close time of
    /// the ledger or block that holds its transaction, as the network states it in the receipt.
    /// `None` when the rail does not state it; a caller then takes the time at which it first
    /// retained the receipt, never a later time at which it processes it. The default states none.
    fn receipt_settled_at(&self, p: &Packet) -> Result<Option<u64>> {
        let _ = p;
        Ok(None)
    }
    /// The identity of an owner-signed packet ([`OwnerSigned::info`]). The default refuses.
    fn owner_signed_info(&self, p: &Packet) -> Result<OwnerSignedInfo> {
        let _ = p;
        Err(crate::client::error::Error::unsupported(
            "This rail does not give the identity of an owner-signed step",
        ))
    }
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
    /// A rail that states when a receipt's operation settled, from the receipt's own document.
    struct Dated;
    static DATED: Dated = Dated;
    impl PacketProvider for Dated {
        fn request_id(&self, p: &Packet) -> Result<[u8; 32]> {
            PROVIDER.request_id(p)
        }
        fn owner_info(&self, p: &Packet) -> Result<OwnerInfo> {
            PROVIDER.owner_info(p)
        }
        fn prepared_metadata(&self, p: &Packet) -> Result<PreparedMetadata> {
            PROVIDER.prepared_metadata(p)
        }
        fn info(&self, p: &Packet, k: Kind) -> Result<PacketInfo> {
            PROVIDER.info(p, k)
        }
        fn validate_signed(&self, p: &Packet) -> Result<()> {
            PROVIDER.validate_signed(p)
        }
        fn signed_preparation(&self, p: &Packet) -> Result<Prepared> {
            PROVIDER.signed_preparation(p)
        }
        fn verify_consent(&self, s: &Snapshot, m: &str, g: &[Signature]) -> Result<()> {
            PROVIDER.verify_consent(s, m, g)
        }
        fn signer_weight(&self, s: &Snapshot, i: &str) -> Result<u32> {
            PROVIDER.signer_weight(s, i)
        }
        fn authority_document(&self, s: &Snapshot) -> Result<serde_json::Value> {
            PROVIDER.authority_document(s)
        }
        fn same_snapshot(&self, a: &Snapshot, b: &Snapshot) -> Result<bool> {
            PROVIDER.same_snapshot(a, b)
        }
        fn verify_receipt(&self, r: &Packet, p: &Packet) -> Result<()> {
            PROVIDER.verify_receipt(r, p)
        }
        fn receipt_info(&self, p: &Packet) -> Result<ReceiptInfo> {
            PROVIDER.receipt_info(p)
        }
        fn receipt_settled_at(&self, p: &Packet) -> Result<Option<u64>> {
            let document: serde_json::Value = serde_json::from_str(p.document().get())
                .map_err(|_| crate::client::error::Error::usage("unreadable receipt"))?;
            Ok(document["closedAt"].as_u64())
        }
    }
    /// A receipt states when its operation settled only when its rail states it: a rail without
    /// the hook states none, and the caller takes the time at which it first retained the receipt.
    #[test]
    fn a_receipt_states_its_settlement_time_when_its_rail_does() {
        let document = serde_json::json!({"closedAt": 1_795_000_000u64});
        let plain = Receipt(Packet::new("test:network", &document, &PROVIDER).unwrap());
        assert_eq!(plain.settled_at().unwrap(), None);
        let dated = Receipt(Packet::new("test:network", &document, &DATED).unwrap());
        assert_eq!(dated.settled_at().unwrap(), Some(1_795_000_000));
    }
    #[test]
    fn params_update_changes_only_installed_params_of_the_same_type() {
        use allowit::AllowitValue as V;
        let installed: BTreeMap<String, PolicyValue> = [
            ("daily_spending_limit".to_owned(), PolicyValue(V::Int(100))),
            ("label".to_owned(), PolicyValue(V::Bool(false))),
            ("operations".to_owned(), PolicyValue(V::Str("*".into()))),
            ("memo".to_owned(), PolicyValue(V::Str("note".into()))),
        ]
        .into();
        let update = |changes: &[(&str, V)]| ParamsUpdate {
            policy_id: "policy".into(),
            expected_revision: Some(1),
            changes: changes
                .iter()
                .map(|(k, v)| ((*k).to_owned(), PolicyValue(v.clone())))
                .collect(),
        };
        let next = update(&[("daily_spending_limit", V::Int(250))])
            .apply(&installed)
            .unwrap();
        assert_eq!(next["daily_spending_limit"], PolicyValue(V::Int(250)));
        assert_eq!(next["label"], installed["label"]);
        assert_eq!(next.len(), installed.len());
        // The wildcard form of a param can become its list form.
        let list = V::List(vec![V::Str("transfer".into())].into());
        let next = update(&[("operations", list.clone())])
            .apply(&installed)
            .unwrap();
        assert_eq!(next["operations"], PolicyValue(list.clone()));
        for refused in [
            update(&[]),
            update(&[("call_limit", V::Int(1))]),
            update(&[("daily_spending_limit", V::Bool(true))]),
            // Only a param that declares both forms can change its form.
            update(&[("memo", list.clone())]),
            update(&[("daily_spending_limit", V::Int(100))]),
        ] {
            assert!(refused.apply(&installed).is_err());
        }
        // The signed-in wire form is stable: camelCase keys, typed values, no unknown field.
        let wire = serde_json::json!({"policyId":"policy","expectedRevision":1,
            "changes":{"daily_spending_limit":{"Int":"250"}}});
        let update = update(&[("daily_spending_limit", V::Int(250))]);
        assert_eq!(serde_json::to_value(&update).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<ParamsUpdate>(wire).unwrap(),
            update
        );
        assert!(
            serde_json::from_value::<ParamsUpdate>(
                serde_json::json!({"policyId":"p","changes":{},"reset":true})
            )
            .is_err()
        );
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
    fn an_account_without_volume_or_unpriced_figures_keeps_its_wire_form() {
        let earlier = r#"{"nonce":1,"deploymentVersion":2,"spentDaily":3,"spentTotal":4,"spentDay":5,"calls":6}"#;
        let account: Accounting = serde_json::from_str(earlier).unwrap();
        assert_eq!(
            account,
            Accounting {
                nonce: 1,
                deployment_version: 2,
                spent_daily: 3,
                spent_total: 4,
                spent_day: 5,
                calls: 6,
                ..Accounting::default()
            }
        );
        assert_eq!(serde_json::to_string(&account).unwrap(), earlier);
        let full = Accounting {
            volume_daily: 7,
            unpriced_total: true,
            unpriced_daily: true,
            ..account
        };
        let text = serde_json::to_string(&full).unwrap();
        assert!(text.contains(r#""volumeDaily":7,"unpricedTotal":true,"unpricedDaily":true"#));
        assert_eq!(serde_json::from_str::<Accounting>(&text).unwrap(), full);
    }
    struct HashOnly;
    impl Signer for HashOnly {
        fn identity(&self) -> &str {
            "hash-only"
        }
        fn sign_hash(&self, digest: &[u8; 32]) -> Result<Vec<u8>> {
            Ok(digest.to_vec())
        }
    }
    impl SendingAccounts for HashOnly {
        fn account(&self) -> Result<String> {
            Ok("hash-only".into())
        }
        fn sign(&self, _: &str, hash: &[u8; 32]) -> Result<Vec<u8>> {
            Ok(hash.to_vec())
        }
    }
    /// Signs the exact bytes it receives: the signature is the bytes reversed, so a test sees
    /// that nothing hashed or prefixed them on the way.
    struct Whole;
    impl Signer for Whole {
        fn identity(&self) -> &str {
            "whole"
        }
        fn sign_hash(&self, digest: &[u8; 32]) -> Result<Vec<u8>> {
            Signer::sign_bytes(self, digest)
        }
        fn sign_bytes(&self, bytes: &[u8]) -> Result<Vec<u8>> {
            Ok(bytes.iter().rev().copied().collect())
        }
    }
    impl SendingAccounts for Whole {
        fn account(&self) -> Result<String> {
            Ok("whole".into())
        }
        fn sign(&self, account: &str, hash: &[u8; 32]) -> Result<Vec<u8>> {
            SendingAccounts::sign_bytes(self, account, hash)
        }
        fn sign_bytes(&self, _: &str, bytes: &[u8]) -> Result<Vec<u8>> {
            Ok(bytes.iter().rev().copied().collect())
        }
    }
    #[test]
    fn a_hash_only_signer_refuses_whole_messages() {
        let message = [7u8; 200];
        let signer: &dyn Signer = &HashOnly;
        let error = signer.sign_bytes(&message).unwrap_err();
        assert_eq!((error.code, error.config), (3, false));
        assert_eq!(error.message, "This signer signs only 32-byte hashes");
        let accounts: &dyn SendingAccounts = &HashOnly;
        let error = accounts.sign_bytes("hash-only", &message).unwrap_err();
        assert_eq!((error.code, error.config), (3, false));
        assert_eq!(
            error.message,
            "These sending accounts sign only 32-byte hashes"
        );
        assert_eq!(signer.sign_hash(&[1; 32]).unwrap(), vec![1; 32]);
    }
    #[test]
    fn a_whole_message_signer_receives_the_exact_bytes() {
        let message: Vec<u8> = (0..=255u8).chain(0..100).collect();
        let expected: Vec<u8> = message.iter().rev().copied().collect();
        let signer: &dyn Signer = &Whole;
        assert_eq!(signer.sign_bytes(&message).unwrap(), expected);
        let accounts: std::sync::Arc<dyn SendingAccounts> = std::sync::Arc::new(Whole);
        assert_eq!(accounts.sign_bytes("whole", &message).unwrap(), expected);
    }
    /// A contract wallet is live only with its instance and its code; without either the
    /// refusal is `contract_expired`.
    #[test]
    fn a_wallet_without_its_instance_or_its_code_is_not_live() {
        let resources = |wallet_present, code_present| Resources {
            code_present,
            wallet_present,
            lifetime: None,
        };
        assert!(resources(true, true).require_live().is_ok());
        for (wallet, code) in [(true, false), (false, true), (false, false)] {
            let refused = resources(wallet, code).require_live().unwrap_err();
            assert_eq!(refused.api_code.as_deref(), Some(CONTRACT_EXPIRED));
            // A local check: no HTTP status and no HTTP wording.
            assert_eq!(refused.status, None);
            assert!(!refused.message.contains("HTTP"));
        }
    }
    /// An executed operation counts the rail's usage, or its debit when the receipt reports none;
    /// an operation that did not execute counts nothing.
    #[test]
    fn an_executed_operation_counts_its_usage_or_its_debit() {
        let usage = Usage {
            amount_units: 2_000,
            gas_units: 136,
            spent_units: 2_136,
            unpriced: true,
        };
        let settlement = |outcome, usage| Settlement {
            transaction: vec![1; 64],
            outcome,
            usage,
            fee_units: 0,
            counted_gas: None,
            next_nonce: None,
            gas_carry: 0,
        };
        assert_eq!(
            settlement(OperationOutcome::Executed, Some(usage)).usage_for(500),
            Some(usage)
        );
        assert_eq!(
            settlement(OperationOutcome::Executed, None).usage_for(500),
            Some(Usage {
                amount_units: 0,
                gas_units: 0,
                spent_units: 500,
                unpriced: false
            })
        );
        for outcome in [OperationOutcome::Failed, OperationOutcome::Open] {
            assert_eq!(settlement(outcome, Some(usage)).usage_for(500), None);
        }
    }
    #[test]
    fn request_equality_never_equates_parse_failures() {
        let nested = format!("{}0{}", "[".repeat(150), "]".repeat(150));
        let other = format!("{}1{}", "[".repeat(150), "]".repeat(150));
        assert_ne!(request_document(&nested), request_document(&other));
    }
}
