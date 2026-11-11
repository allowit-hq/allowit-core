//! Public installation metadata.
use crate::policy::PolicyValue;
use crate::vendor::runtime::Tier;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Custody {
    Owner,
    Policy,
    Retired,
}
#[derive(Clone, Debug)]
pub struct ConfigurationInfo {
    pub network: String,
    pub wallet: String,
    pub owner: String,
    pub agent: String,
    pub server: String,
    pub source: String,
    pub source_hash: [u8; 32],
    pub binding_hash: [u8; 32],
    pub rules: Vec<String>,
    pub is_retired: bool,
    pub module_digest: [u8; 32],
    pub policy_instance: [u8; 32],
    pub asset: String,
    pub asset_symbol: String,
    pub asset_issuer: Option<String>,
    pub max_fee_units: u64,
    pub is_native_asset: bool,
    pub fee_decimals: u8,
    pub payment_asset_decimals: u32,
    pub params: BTreeMap<String, PolicyValue>,
    pub custody: Custody,
    pub custody_encoding: serde_json::Value,
    pub tier_encoding: serde_json::Value,
    pub tier: Option<Tier>,
}
impl ConfigurationInfo {
    /// The wallet kind that this installation binds, when its custody and tier name one.
    pub fn wallet_kind(&self) -> Option<crate::vendor::kinds::WalletKind> {
        crate::vendor::kinds::WalletKind::from_binding(self.custody, self.tier)
    }
}
/// Literal retained configuration with read-only common metadata. Serialization keeps original native bytes.
#[derive(Clone, Debug)]
pub struct Configuration {
    info: ConfigurationInfo,
    document: Box<serde_json::value::RawValue>,
}
impl std::ops::Deref for Configuration {
    type Target = ConfigurationInfo;
    fn deref(&self) -> &Self::Target {
        &self.info
    }
}
impl PartialEq for Configuration {
    fn eq(&self, other: &Self) -> bool {
        self.document.get() == other.document.get()
    }
}
impl Eq for Configuration {}
impl Configuration {
    /// Retain the adapter's validated literal configuration and its common projection.
    pub fn new(info: ConfigurationInfo, document: Box<serde_json::value::RawValue>) -> Self {
        Self { info, document }
    }
    pub fn info(&self) -> &ConfigurationInfo {
        &self.info
    }
    pub fn document(&self) -> &serde_json::value::RawValue {
        &self.document
    }
    pub fn bytes(&self) -> &[u8] {
        self.document.get().as_bytes()
    }
}
impl serde::Serialize for Configuration {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        self.document.serialize(s)
    }
}
/// Caller-admitted policy identity for a new owner-custody projection. This is not source consent.
pub struct PolicyIdentity {
    pub policy_instance: [u8; 32],
    pub module_digest: [u8; 32],
    pub source: String,
    pub params: BTreeMap<String, PolicyValue>,
    pub asset: String,
    pub asset_symbol: String,
    pub asset_issuer: Option<String>,
    pub agent: String,
    pub server: String,
    pub max_fee_units: u64,
}
#[derive(Clone, Debug)]
pub struct FeeBounds {
    pub minimum: u64,
    pub maximum_execution: u64,
    pub default_execution: u64,
    pub default_deployment: u64,
    pub decimals: u8,
    pub native_symbol: String,
}
/// Literal source-consent inputs. The native codec owns field names and the signing domain.
pub struct SourceConsent<'a, P: serde::Serialize> {
    pub configuration: &'a Configuration,
    pub policy_id: &'a str,
    pub revision: u32,
    pub source: &'a str,
    pub source_hash: &'a str,
    pub ir_hash: &'a str,
    pub module_digest: &'a str,
    pub language: &'a str,
    pub tier: &'a str,
    pub rules: &'a [String],
    pub params: &'a P,
    pub params_hash: &'a str,
}
