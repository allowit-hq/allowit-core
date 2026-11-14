//! Rail plugin contract. A vendor package implements these traits; the SDK registers each
//! plugin with one line and dispatches by network. This module names no rail.
use super::{
    KeyProvider, OwnerProvider, RuntimeProvider,
    error::{Error, Result, quoted},
    execution::{Capabilities, LimitReport, Outcome, Recovery, Request},
    generation::{GenerateRequest, GeneratedDraft},
    input::{
        Formats,
        asset::{Amount, AssetDisplay, Token},
    },
    kinds::WalletKind,
    paid_service::PaidServiceProvider,
    profiles::{Description, Profile, ProfileBinding},
    request_proof::Protocol,
    routing::RailInfo,
    runtime::{
        Installation, Kind, OwnerPacket, Packet, PolicyInstallation, ReceiptInfo, Tier, Transport,
        owner::{Configuration, Custody, FeeBounds, PolicyIdentity, SourceConsent},
    },
    wallet::Signer,
};
use crate::client::store::Store;
use serde_json::{Value, value::RawValue};
use std::{sync::Arc, time::Duration};

/// Opens the caller-owned store namespace that the SDK assigned to this plugin.
pub type StoreOpener<'a> = &'a dyn Fn() -> Result<Store>;
pub use crate::policy_package::PolicyFunction;

/// One rail family. Every method takes a canonical network id from [`RailPlugin::networks`].
pub trait RailPlugin: Sync {
    /// Lowercase namespace for store names, element tags and policy functions.
    fn id(&self) -> &'static str;
    /// The CAIP-2 networks this plugin serves.
    fn networks(&self) -> &'static [RailInfo];
    /// Admitted operations, owner verbs and key curve.
    fn capabilities(&self, network: &str) -> Capabilities;
    /// What the shared looped tests run on this rail, and what they skip.
    fn test_support(&self) -> &'static TestSupport;

    /// Native address and asset formats.
    fn formats(&self) -> &'static Formats;
    /// Asset metadata for a network.
    fn tokens(&self, network: &str) -> &'static [Token];
    /// Display metadata of the network's native asset: its unit symbol, decimals and name.
    fn native_asset(&self, network: &str) -> Result<AssetDisplay> {
        self.tokens(network)
            .iter()
            .find(|token| token.native)
            .map(AssetDisplay::from)
            .ok_or_else(|| Error::unsupported("This network publishes no native asset"))
    }
    /// Display metadata of an asset reference: a code, or the issuer, mint or token address.
    /// Hosts print amounts with [`AssetDisplay::format`]; they never invent a unit.
    fn asset_display(&self, network: &str, asset: &str) -> Result<AssetDisplay> {
        self.tokens(network)
            .iter()
            .find(|token| token.code == asset || token.issuer == Some(asset))
            .map(AssetDisplay::from)
            .ok_or_else(|| Error::usage(format!("unknown asset {}", quoted(asset))))
    }
    /// The wallet kinds this rail supports on a network, in selection order.
    fn wallet_kinds(&self, network: &str) -> Vec<WalletKind> {
        let _ = network;
        let profiles = self.server().profiles();
        WalletKind::ALL
            .into_iter()
            .filter(|kind| {
                profiles
                    .iter()
                    .any(|profile| profile.wallet_kind() == Some(*kind))
            })
            .collect()
    }
    /// The wallet kind that a public wallet profile document binds.
    fn wallet_kind(&self, network: &str, document: &[u8]) -> Result<WalletKind> {
        self.decode_profile(network, document)?
            .1
            .wallet_kind()
            .ok_or_else(|| Error::unsupported("This rail does not report a wallet kind"))
    }
    /// A block-explorer link, if the network has one.
    fn explorer(&self, network: &str, kind: LinkKind, id: &str) -> Option<String>;
    /// Validity windows and block time.
    fn deadlines(&self, network: &str) -> Deadlines;

    /// Local key derivation and parsing.
    fn keys(&self) -> &'static dyn KeyProvider;
    /// Fund a test key. Networks without a faucet return an unsupported error.
    fn fund_key(&self, network: &str, address: &str, endpoint: Option<&str>) -> Result<String>;
    /// Decode a public wallet profile document.
    fn decode_profile(
        &self,
        network: &str,
        bytes: &[u8],
    ) -> Result<(Description, Box<dyn ProfileBinding>)>;
    /// Ask the backend to generate a policy draft for this rail.
    fn generate(
        &self,
        cx: &GenerateContext<'_>,
        request: &GenerateRequest,
    ) -> Result<GeneratedDraft>;

    /// The policy params of a draft, policy or installation document of this rail, with the
    /// asset that their amounts count. The SDK prints them; the document shape stays with the
    /// rail. A rail without a readable document declares [`Feature::DraftDisplay`] skipped.
    fn draft_params(&self, network: &str, document: &Value) -> Result<DraftParams> {
        let _ = (network, document);
        Err(Error::unsupported(
            "This rail does not describe the params of a draft",
        ))
    }

    /// Agent execution and receipts.
    fn execution(&self) -> &'static dyn ExecutionProvider;
    /// Client owner workflow: deploy, fund, activate, export, setup and status.
    fn owner(&self) -> &'static dyn OwnerProvider;
    /// Server owner-step preparation and validation.
    fn owner_runtime(&self) -> &'static dyn OwnerRuntime;

    /// Open a bounded RPC transport to a network endpoint.
    fn connect(&self, network: &str, endpoint: &str) -> Result<Arc<dyn Transport>>;
    /// The local runtime over a transport.
    fn runtime(
        &self,
        network: &str,
        transport: Arc<dyn Transport>,
    ) -> Result<Box<dyn RuntimeProvider>>;
    /// Bind a portable policy installation to its native encoding.
    fn install(&self, installation: PolicyInstallation) -> Result<Installation>;
    /// Restore a retained native packet. The SDK stores `{rail: id(), packet}`.
    fn restore(&self, kind: Kind, raw: Box<RawValue>) -> Result<Packet>;
    /// The agent request-proof protocol.
    fn request_proof_protocol(&self) -> Protocol;
    /// The native identity of a signer public key.
    fn signer_identity(&self, key: &[u8; 32]) -> Result<String>;
    /// The operation id of a request packet.
    fn request_id(&self, packet: &Packet) -> Result<[u8; 32]>;
    /// Server hooks.
    fn server(&self) -> &'static dyn ServerHooks;

    /// The paid-service hook: the seller challenges that this rail parses and the proofs that
    /// it builds from settled operations. A rail without it declares
    /// [`Feature::PaidService`] skipped.
    fn paid_service(&self) -> Option<&'static dyn PaidServiceProvider> {
        None
    }

    /// Vendor evidence functions, by name, that policies may call.
    fn policy_functions(&self) -> &'static [(&'static str, PolicyFunction)] {
        &[]
    }
}

/// The params of a rail document and the asset that their amounts count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftParams {
    /// Param key to its `PolicyValue` JSON, in the order of the document.
    pub params: Vec<(String, Value)>,
    /// The installed asset: the unit of every amount param.
    pub asset: AssetDisplay,
}

/// Inputs for backend policy generation. Keys stay with the caller.
pub struct GenerateContext<'a> {
    pub backend: &'a str,
    pub origin: &'a str,
    pub signer: &'a dyn Signer,
    pub clock: &'a dyn Fn() -> Result<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    Address,
    Transaction,
    Contract,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deadlines {
    pub max_validity_secs: u64,
    pub owner_step_ttl_secs: u64,
    pub block_time_secs: u64,
}

/// Client execution. The SDK owns the journal and the store root; the vendor gets a store opener.
pub trait ExecutionProvider: Sync {
    fn check(
        &self,
        network: &str,
        op: &str,
        to: &str,
        minimum: Option<&Amount>,
        args: bool,
    ) -> Result<()>;
    fn build(
        &self,
        profile: &Profile,
        store: StoreOpener<'_>,
        rpc: Option<&str>,
        request: &Request,
    ) -> Result<Value>;
    fn execute(
        &self,
        profile: Profile,
        signer: Arc<dyn Signer>,
        store: StoreOpener<'_>,
        rpc: Option<String>,
        request: &Request,
    ) -> Result<Outcome>;
    fn status(
        &self,
        profile: Profile,
        signer: Arc<dyn Signer>,
        store: StoreOpener<'_>,
        rpc: Option<String>,
        id: &str,
        wait: Duration,
    ) -> Result<Outcome>;
    fn retained(
        &self,
        store: StoreOpener<'_>,
        network: &str,
        wallet: Option<&str>,
        id: &str,
    ) -> Result<Option<Recovery>>;
    /// The spending limits of the profile's wallet as the backend reports them: what each limit
    /// allows, what is spent and what remains. `None` means that the rail holds no report for
    /// this wallet yet. A rail without the read declares [`Feature::Limits`] skipped.
    fn limits(
        &self,
        profile: Profile,
        signer: Arc<dyn Signer>,
        store: StoreOpener<'_>,
        rpc: Option<String>,
    ) -> Result<Option<Vec<LimitReport>>> {
        let _ = (profile, signer, store, rpc);
        Err(Error::unsupported(
            "This rail does not report spending limits",
        ))
    }
    #[allow(clippy::too_many_arguments)]
    fn recover(
        &self,
        original: Recovery,
        signer: Arc<dyn Signer>,
        backend: &str,
        store: StoreOpener<'_>,
        id: &str,
        wait: Duration,
        request: Option<&Request>,
    ) -> Result<Outcome>;
}

/// Owner steps: the server prepares them; the owner's wallet signs them in a vendor element or
/// the CLI.
pub trait OwnerRuntime: Sync {
    fn configuration(&self, network: &str, document: &[u8]) -> Result<Configuration>;
    fn envelope_hash(
        &self,
        network: &str,
        payload: &[u8],
        require_signature: bool,
    ) -> Result<[u8; 32]>;
    fn validate_signed(&self, network: &str, original: &[u8], signed: &[u8]) -> Result<[u8; 32]>;
    fn with_policy(&self, c: &Configuration, identity: PolicyIdentity) -> Result<Configuration>;
    /// The installation after a confirmed params update: the same wallet, source and identity
    /// with `update` applied to its params. The caller stores it only after the update settled.
    fn with_params(
        &self,
        c: &Configuration,
        update: &crate::vendor::runtime::ParamsUpdate,
    ) -> Result<Configuration> {
        let _ = (c, update);
        Err(Error::unsupported(
            "This rail does not update the params of an installed wallet",
        ))
    }
    fn code_hash(&self, wasm: &[u8]) -> Result<[u8; 32]>;
    fn fee_bounds(&self) -> FeeBounds;
    fn wallet_address(&self, network: &str, owner: &str, instance: &[u8; 32]) -> Result<String>;
    fn custody_encoding(&self, c: Custody) -> Result<Value>;
    fn tier_encoding(&self, t: Option<Tier>) -> Result<Value>;
    fn source_consent(&self, input: &SourceConsent<'_, Value>) -> Result<(Value, String)>;
    /// The rail-neutral step that the owner signs.
    fn step(&self, network: &str, packet: &OwnerPacket) -> Result<OwnerStep>;
}

/// Rail-neutral owner step. `payload` is opaque text that only the vendor signs and reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerStep {
    pub action: String,
    pub payload: String,
    pub hash: [u8; 32],
    pub expires_at: u64,
    pub explorer: Option<String>,
}

/// Server hooks. The engine calls them; it never interprets vendor configuration.
pub trait ServerHooks: Sync {
    /// Validate this network's block of the server rail configuration.
    fn configure(&self, network: &str, config: &Value) -> Result<()>;
    /// The wallet profiles this rail offers.
    fn profiles(&self) -> &'static [ProfileInfo];
    /// Verify a wallet sign-in signature over `message`.
    fn verify_sign_in(
        &self,
        network: &str,
        address: &str,
        message: &[u8],
        signature: &[u8],
    ) -> Result<()>;
    /// The native fee of a receipt in installed-asset units. It counts against spending and
    /// volume limits.
    fn charged_fee(&self, network: &str, config: &Value, receipt: &ReceiptInfo) -> Result<i128>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProfileInfo {
    pub id: &'static str,
    pub custody: Custody,
    pub tier: Option<Tier>,
    pub label: &'static str,
}
impl ProfileInfo {
    /// The wallet kind that this profile installs.
    pub fn wallet_kind(&self) -> Option<WalletKind> {
        WalletKind::from_binding(self.custody, self.tier)
    }
}

/// A feature that shared tests exercise on each registered rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    Transfer,
    Swap,
    Deposit,
    Redeem,
    KeysFund,
    OwnerSetup,
    Deploy,
    Fund,
    Activate,
    Export,
    Recover,
    ExplorerLinks,
    Live,
    /// The read of spent and remaining spending limits.
    Limits,
    /// The params of a generated draft for display.
    DraftDisplay,
    /// Seller payment challenges and proofs of settled payments.
    PaidService,
    /// Typed param edits and revisions of an unsigned draft.
    EditParams,
    /// The owner-signed params update of an active wallet.
    UpdateParams,
    /// Ledger verification with the comparison and the state read-back.
    Verify,
    /// The paired host and native placements of a policy.
    PairedExport,
}

/// Test capability declaration of one rail.
pub struct TestSupport {
    /// The testnet id that looped tests use.
    pub network: &'static str,
    /// Features this rail skips, with the reason.
    pub skips: &'static [(Feature, &'static str)],
    pub fixtures: fn() -> Fixtures,
}
impl TestSupport {
    /// The reason this rail skips `feature`, if it does.
    pub fn skip(&self, feature: Feature) -> Option<&'static str> {
        self.skips
            .iter()
            .find(|(skipped, _)| *skipped == feature)
            .map(|(_, reason)| *reason)
    }
}

/// Rail values for shared tests. Shared tests never assert a vendor literal.
pub struct Fixtures {
    pub account: String,
    pub wallet: String,
    pub recipient: String,
    pub invalid_address: String,
    pub token: Token,
    pub agent_seed: [u8; 32],
    /// An importable public wallet profile document.
    pub public_profile: Vec<u8>,
    /// A backend owner-generation reply.
    pub owner_generation: Value,
    /// Deterministic in-process replay for offline execution, status and owner tests.
    pub chain: Option<Arc<dyn Transport>>,
    /// Live testnet settings for ignored tests.
    pub live: Option<LiveConfig>,
}

/// Environment variables that enable ignored live tests.
#[derive(Clone, Copy, Debug)]
pub struct LiveConfig {
    pub env: &'static [&'static str],
}
