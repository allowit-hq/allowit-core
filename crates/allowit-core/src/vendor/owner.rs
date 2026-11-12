//! Owner capability inputs. Keys remain caller-owned.
use super::wallet::Signer;
use crate::client::error::Result;
use crate::client::store::Store;
pub struct Context<'a> {
    pub network: &'a str,
    pub backend: &'a str,
    pub authority: &'a crate::vendor::config::ServerAuthority,
    pub origin: &'a str,
    pub rpc_url: &'a str,
    /// Open caller-owned storage after the native preflight checks succeed.
    pub store: &'a dyn Fn() -> Result<Store>,
    pub signer: &'a dyn Signer,
    pub clock: &'a dyn Fn() -> Result<u64>,
}
pub struct Deploy<'a> {
    pub draft: &'a [u8],
    pub agent: &'a str,
    pub server: &'a str,
    pub executable: &'a [u8],
    pub request_id: &'a str,
    pub fee_limit: &'a str,
}
pub struct Activate<'a> {
    pub document: &'a [u8],
    pub agent: Option<&'a str>,
    pub server: Option<&'a str>,
    pub request_id: &'a str,
}
pub struct Setup<'a> {
    pub agent: &'a str,
    pub server: &'a str,
    pub request_id: &'a str,
    pub fee_per_operation: &'a str,
}
/// Typed edits of a draft's params through the backend's owner route. The backend checks each
/// value against the param's type and range and refuses an invalid value.
pub struct EditParams<'a> {
    pub draft: &'a [u8],
    pub changes: &'a std::collections::BTreeMap<String, crate::policy::PolicyValue>,
}
/// One revision of a draft in the owner's words. It can change the source.
pub struct Revise<'a> {
    pub draft: &'a [u8],
    pub instruction: &'a str,
}
/// An owner-signed change of installed params on an active wallet.
pub struct UpdateParams<'a> {
    /// The owner's deployment or activation document of the installed policy.
    pub document: &'a [u8],
    /// Each changed param with its complete new typed value.
    pub changes: &'a std::collections::BTreeMap<String, crate::policy::PolicyValue>,
    pub request_id: &'a str,
    /// The configuration revision that the owner reviewed, when the owner names one.
    pub expected_revision: Option<u64>,
    pub fee_limit: &'a str,
}
/// One compared fact of a verification: what the owner's document approves and what the
/// ledger holds.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Comparison {
    /// The compared fact, such as `executable`, `source`, `params` or `walletKind`.
    pub field: String,
    pub expected: serde_json::Value,
    pub observed: serde_json::Value,
    pub matches: bool,
}
impl Comparison {
    pub fn new(field: &str, expected: serde_json::Value, observed: serde_json::Value) -> Self {
        Self {
            field: field.into(),
            matches: expected == observed,
            expected,
            observed,
        }
    }
}
/// The usage that a wallet has recorded, in installed-asset units.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Counters {
    #[serde(with = "decimal")]
    pub spent_total: i128,
    #[serde(with = "decimal")]
    pub spent_daily: i128,
    /// The UTC day number that `spent_daily` counts.
    pub spent_day: u64,
    pub calls: u64,
    /// The next request nonce.
    pub nonce: u64,
}
/// The state that verification read from the ledger.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StateReadback {
    pub wallet: String,
    pub wallet_kind: Option<crate::vendor::kinds::WalletKind>,
    /// The installed configuration revision.
    pub revision: u64,
    pub paused: bool,
    /// Where the counters are recorded: `ledger` for a wallet that enforces its policy
    /// on-chain, `server` for a wallet whose policy the server enforces.
    pub counters_from: String,
    pub counters: Option<Counters>,
    /// Param key to its installed typed value.
    pub params: std::collections::BTreeMap<String, crate::policy::PolicyValue>,
    /// The ledger height of the read.
    pub observed_height: u64,
}
/// The result of comparing the owner's local document with the ledger. `matches` is true only
/// when every comparison matches.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Verification {
    pub policy_id: String,
    pub network: String,
    pub matches: bool,
    pub comparisons: Vec<Comparison>,
    pub state: StateReadback,
}
impl Verification {
    /// Build a verification whose `matches` follows its comparisons. No comparison is not a match.
    pub fn new(
        policy_id: String,
        network: String,
        comparisons: Vec<Comparison>,
        state: StateReadback,
    ) -> Self {
        Self {
            matches: !comparisons.is_empty() && comparisons.iter().all(|c| c.matches),
            policy_id,
            network,
            comparisons,
            state,
        }
    }
}
/// What the server evaluates before it signs: the complete source, the evidence functions that
/// the source calls on the host, and the params.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostPlacement {
    pub source: String,
    pub source_hash: String,
    /// Every rule that the source calls, in source order. The server evaluates all of them.
    pub rules: Vec<String>,
    /// Host evidence functions that the source calls. They never run on-chain.
    pub evidence_functions: Vec<String>,
    pub params: std::collections::BTreeMap<String, crate::policy::PolicyValue>,
}
/// What the wallet enforces on each invocation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePlacement {
    pub wallet_kind: crate::vendor::kinds::WalletKind,
    pub enforcement: crate::vendor::kinds::Enforcement,
    /// The ordered rules that the wallet checks on-chain. Empty when the server alone enforces
    /// the policy; a compiled source enforces every rule it calls.
    pub rules: Vec<String>,
    /// The source that the wallet compiled, when the wallet kind compiles source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The params that the wallet stores. Empty when the wallet stores none.
    pub params: std::collections::BTreeMap<String, crate::policy::PolicyValue>,
}
/// A draft as its two placements: the host part and the native part of one policy.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairedExport {
    pub policy_id: String,
    pub network: String,
    pub host: HostPlacement,
    pub native: NativePlacement,
}
impl PairedExport {
    /// Pair the placements of one checked source. `rules` are the rules that the source calls in
    /// order, `prefix` the ordered canonical prefix, and `evidence_functions` its host calls.
    #[allow(clippy::too_many_arguments)]
    pub fn of(
        policy_id: String,
        network: String,
        kind: crate::vendor::kinds::WalletKind,
        source: String,
        source_hash: String,
        rules: Vec<String>,
        prefix: Vec<String>,
        evidence_functions: Vec<String>,
        params: std::collections::BTreeMap<String, crate::policy::PolicyValue>,
    ) -> Self {
        use crate::vendor::kinds::{Enforcement, WalletKind};
        let (enforcement, native_rules, native_source, native_params) = match kind {
            WalletKind::Native | WalletKind::Simple => {
                (Enforcement::Offchain, Vec::new(), None, Default::default())
            }
            WalletKind::Standard => (Enforcement::Onchain, prefix, None, params.clone()),
            WalletKind::Custom => (
                Enforcement::Onchain,
                rules.clone(),
                Some(source.clone()),
                params.clone(),
            ),
        };
        Self {
            policy_id,
            network,
            host: HostPlacement {
                source,
                source_hash,
                rules,
                evidence_functions,
                params,
            },
            native: NativePlacement {
                wallet_kind: kind,
                enforcement,
                rules: native_rules,
                source: native_source,
                params: native_params,
            },
        }
    }
}
/// Canonical decimal text for integers beyond the JSON number range.
mod decimal {
    pub fn serialize<S: serde::Serializer>(v: &i128, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(v)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i128, D::Error> {
        use serde::de::Error;
        let text = <String as serde::Deserialize>::deserialize(d)?;
        let value = text.parse::<i128>().map_err(D::Error::custom)?;
        if value.to_string() != text {
            return Err(D::Error::custom("Expected canonical decimal text"));
        }
        Ok(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::PolicyValue;
    use crate::vendor::kinds::{Enforcement, WalletKind};
    use allowit::AllowitValue as V;
    fn params() -> std::collections::BTreeMap<String, PolicyValue> {
        [("daily_spending_limit".to_owned(), PolicyValue(V::Int(5)))].into()
    }
    fn state() -> StateReadback {
        StateReadback {
            wallet: "wallet".into(),
            wallet_kind: Some(WalletKind::Standard),
            revision: 2,
            paused: false,
            counters_from: "ledger".into(),
            counters: Some(Counters {
                spent_total: i128::MAX,
                spent_daily: 7,
                spent_day: 20_769,
                calls: 3,
                nonce: 4,
            }),
            params: params(),
            observed_height: 9,
        }
    }
    #[test]
    fn verification_matches_only_when_every_comparison_matches() {
        let same = Comparison::new("source", "a".into(), "a".into());
        let other = Comparison::new("params", "a".into(), "b".into());
        assert!(same.matches && !other.matches);
        let verify = |c: Vec<Comparison>| Verification::new("p".into(), "n".into(), c, state());
        assert!(verify(vec![same.clone()]).matches);
        assert!(!verify(vec![same, other]).matches);
        assert!(!verify(vec![]).matches);
    }
    #[test]
    fn verification_wire_form_is_stable_and_keeps_wide_counters() {
        let v = Verification::new(
            "p".into(),
            "n".into(),
            vec![Comparison::new(
                "walletKind",
                "standard".into(),
                "standard".into(),
            )],
            state(),
        );
        let wire = serde_json::to_value(&v).unwrap();
        assert_eq!(
            wire["state"]["counters"]["spentTotal"],
            i128::MAX.to_string()
        );
        assert_eq!(wire["state"]["walletKind"], "standard");
        assert_eq!(wire["state"]["revision"], 2);
        assert_eq!(wire["state"]["params"]["daily_spending_limit"]["Int"], "5");
        assert_eq!(wire["comparisons"][0]["field"], "walletKind");
        assert_eq!(serde_json::from_value::<Verification>(wire).unwrap(), v);
        assert!(
            serde_json::from_value::<Counters>(serde_json::json!({"spentTotal":"01",
                "spentDaily":"0","spentDay":0,"calls":0,"nonce":0}))
            .is_err()
        );
    }
    #[test]
    fn paired_export_separates_what_the_server_evaluates_from_what_the_wallet_enforces() {
        let pair = |kind| {
            PairedExport::of(
                "p".into(),
                "n".into(),
                kind,
                "source".into(),
                "hash".into(),
                vec![
                    "operations".into(),
                    "payment_limit".into(),
                    "destinations".into(),
                ],
                vec!["operations".into(), "payment_limit".into()],
                vec!["catalog::report".into()],
                params(),
            )
        };
        for kind in [WalletKind::Native, WalletKind::Simple] {
            let p = pair(kind);
            assert_eq!(p.native.enforcement, Enforcement::Offchain);
            assert!(p.native.rules.is_empty() && p.native.params.is_empty());
            assert_eq!(p.host.rules.len(), 3);
        }
        let standard = pair(WalletKind::Standard);
        assert_eq!(standard.native.rules, ["operations", "payment_limit"]);
        assert_eq!(standard.native.source, None);
        let custom = pair(WalletKind::Custom);
        assert_eq!(custom.native.rules, custom.host.rules);
        assert_eq!(custom.native.source.as_deref(), Some("source"));
        // Evidence functions stay with the host in every placement.
        for p in [standard, custom] {
            assert_eq!(p.host.evidence_functions, ["catalog::report"]);
            assert_eq!(p.native.params, p.host.params);
            let wire = serde_json::to_value(&p).unwrap();
            assert_eq!(serde_json::from_value::<PairedExport>(wire).unwrap(), p);
        }
    }
}
