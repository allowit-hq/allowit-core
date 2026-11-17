//! Wallet kinds: the four selections that every host offers an owner. This module names no rail.
//! A rail advertises the kinds it supports through `RailPlugin::wallet_kinds`.
use super::runtime::{Tier, owner::Custody};
use serde::{Deserialize, Serialize};

/// The public list of the params that the `standard` kind enforces onchain.
pub const STANDARD_PARAMS_URL: &str =
    "https://github.com/allowit-hq/allowit-core/blob/main/README.md#standard-wallet-parameters";

/// The params that the `standard` kind enforces onchain: the keys of [`crate::RULES`], in order.
/// `policy::canonical_function_registry` describes the type and range of each one.
pub const STANDARD_PARAMS: [&str; 9] = [
    crate::PAYMENT_LIMIT,
    crate::LIFETIME_SPENDING_LIMIT,
    crate::DAILY_SPENDING_LIMIT,
    crate::DAILY_VOLUME_LIMIT,
    crate::CALL_LIMIT,
    crate::DESTINATIONS,
    crate::TOKENS,
    crate::EXPIRES_AT,
    crate::OPERATIONS,
];

/// The stable selection id of a wallet kind: `native`, `simple`, `standard` or `custom`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WalletKind {
    Native,
    Simple,
    Standard,
    Custom,
}

/// Where the policy is enforced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Enforcement {
    Offchain,
    Onchain,
}

/// What holds the funds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WalletCustody {
    Keypair,
    Contract,
}

/// The owner-facing description of one wallet kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletKindInfo {
    pub id: WalletKind,
    pub title: &'static str,
    /// What the wallet is and who enforces the policy, in the owner's words.
    pub explanation: &'static str,
    /// Hosts recommend this kind to an owner who deploys. Exactly one kind is recommended.
    pub recommended: bool,
    /// The sellers that a wallet of this kind pays, in the owner's words.
    pub pays: &'static str,
    /// What a payment costs the wallet in network fee, in the owner's words. A rail states the
    /// amounts.
    pub fee: &'static str,
    pub enforcement: Enforcement,
    pub custody: WalletCustody,
    /// The params this kind enforces onchain. Empty unless the kind has a fixed list.
    pub supported_params: &'static [&'static str],
    /// The public list of `supported_params`.
    pub params_url: Option<&'static str>,
}

/// The sellers that a contract wallet pays: its payment settles first, and the seller takes the
/// settled transaction as proof. A seller whose own facilitator settles the payment needs the
/// agent's own wallet.
const PROOF_SELLERS: &str = "Fewer sellers: only those that accept a settled payment as proof.";
/// The network fee of a payment from a contract wallet.
const CONTRACT_FEE: &str = "Higher: each payment runs the wallet contract on chain.";

/// Every wallet kind, in selection order. The recommended kind is first.
pub const WALLET_KINDS: [WalletKindInfo; 4] = [
    WalletKindInfo {
        id: WalletKind::Native,
        title: "Native",
        explanation: "Your agent's own wallet. Allowit checks each payment against your policy before it co-signs.",
        recommended: true,
        pays: "Every x402 seller.",
        fee: "The lowest: none when the seller's facilitator settles the payment.",
        enforcement: Enforcement::Offchain,
        custody: WalletCustody::Keypair,
        supported_params: &[],
        params_url: None,
    },
    WalletKindInfo {
        id: WalletKind::Simple,
        title: "Simple",
        explanation: "A contract wallet holds the funds. Allowit checks each payment against your policy, as for Native.",
        recommended: false,
        pays: PROOF_SELLERS,
        fee: CONTRACT_FEE,
        enforcement: Enforcement::Offchain,
        custody: WalletCustody::Contract,
        supported_params: &[],
        params_url: None,
    },
    WalletKindInfo {
        id: WalletKind::Standard,
        title: "Standard",
        explanation: "A contract wallet enforces the standard rules of your policy on chain.",
        recommended: false,
        pays: PROOF_SELLERS,
        fee: CONTRACT_FEE,
        enforcement: Enforcement::Onchain,
        custody: WalletCustody::Contract,
        supported_params: &STANDARD_PARAMS,
        params_url: Some(STANDARD_PARAMS_URL),
    },
    WalletKindInfo {
        id: WalletKind::Custom,
        title: "Custom",
        explanation: "A contract built from your policy enforces all of it on chain.",
        recommended: false,
        pays: PROOF_SELLERS,
        fee: CONTRACT_FEE,
        enforcement: Enforcement::Onchain,
        custody: WalletCustody::Contract,
        supported_params: &[],
        params_url: None,
    },
];

impl WalletKind {
    /// Every kind, in selection order.
    pub const ALL: [WalletKind; 4] = [Self::Native, Self::Simple, Self::Standard, Self::Custom];

    /// The stable selection id.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Simple => "simple",
            Self::Standard => "standard",
            Self::Custom => "custom",
        }
    }
    /// The kind with this selection id. Ids are exact; there are no aliases.
    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }
    /// The owner-facing description.
    pub const fn info(self) -> &'static WalletKindInfo {
        &WALLET_KINDS[self as usize]
    }
    /// The custody and tier that an installation of this kind binds.
    pub const fn binding(self) -> (Custody, Option<Tier>) {
        match self {
            Self::Native => (Custody::Owner, None),
            Self::Simple => (Custody::Policy, Some(Tier::Simple)),
            Self::Standard => (Custody::Policy, Some(Tier::Standard)),
            Self::Custom => (Custody::Policy, Some(Tier::Custom)),
        }
    }
    /// The kind that a custody and tier bind. A retired wallet and an inconsistent pair have none.
    pub const fn from_binding(custody: Custody, tier: Option<Tier>) -> Option<Self> {
        match (custody, tier) {
            (Custody::Owner, None) => Some(Self::Native),
            (Custody::Policy, Some(Tier::Simple)) => Some(Self::Simple),
            (Custody::Policy, Some(Tier::Standard)) => Some(Self::Standard),
            (Custody::Policy, Some(Tier::Custom)) => Some(Self::Custom),
            _ => None,
        }
    }
}
impl core::fmt::Display for WalletKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kinds_have_stable_ids_and_round_trip_their_binding() {
        let ids: Vec<_> = WALLET_KINDS.iter().map(|k| k.id.id()).collect();
        assert_eq!(ids, ["native", "simple", "standard", "custom"]);
        for kind in WalletKind::ALL {
            assert_eq!(kind.info().id, kind);
            assert_eq!(WalletKind::parse(kind.id()), Some(kind));
            let (custody, tier) = kind.binding();
            assert_eq!(WalletKind::from_binding(custody, tier), Some(kind));
            assert_eq!(serde_json::to_value(kind).unwrap(), kind.id());
        }
        assert_eq!(WalletKind::parse("common"), None);
        assert_eq!(WalletKind::from_binding(Custody::Retired, None), None);
        assert_eq!(
            WalletKind::from_binding(Custody::Owner, Some(Tier::Simple)),
            None
        );
        assert_eq!(WalletKind::from_binding(Custody::Policy, None), None);
    }
    /// Hosts recommend one kind, the agent's own wallet, and it is the first in the selection.
    /// Every kind says in plain words what it is, which sellers it pays and what a payment
    /// costs.
    #[test]
    fn one_kind_is_recommended_and_every_kind_states_its_sellers_and_its_fee() {
        let recommended: Vec<_> = WALLET_KINDS.iter().filter(|k| k.recommended).collect();
        assert_eq!(recommended.len(), 1);
        assert_eq!(recommended[0].id, WalletKind::Native);
        assert!(WALLET_KINDS[0].recommended);
        assert_eq!(WALLET_KINDS[0].pays, "Every x402 seller.");
        for kind in &WALLET_KINDS {
            for text in [kind.explanation, kind.pays, kind.fee] {
                assert!(!text.is_empty() && text.ends_with('.'), "{text}");
                assert!(text.len() <= 120, "{text}");
                // Owner words: no enforcement jargon and no amount, which a rail states.
                for jargon in ["offchain", "onchain", "keypair", "smart-contract", "stroop"] {
                    assert!(!text.contains(jargon), "{text}");
                }
                // `x402` is the name of the payment protocol.
                let words = text.replace("x402", "");
                assert!(!words.bytes().any(|b| b.is_ascii_digit()), "{text}");
            }
            // Only the agent's own wallet pays a seller whose facilitator settles.
            assert_eq!(
                kind.pays == PROOF_SELLERS,
                kind.custody == WalletCustody::Contract
            );
            assert_eq!(
                kind.fee == CONTRACT_FEE,
                kind.custody == WalletCustody::Contract
            );
        }
        let json = serde_json::to_value(WALLET_KINDS[0]).unwrap();
        assert_eq!(json["recommended"], true);
        assert_eq!(json["pays"], "Every x402 seller.");
        assert!(json["fee"].is_string());
    }
    #[test]
    fn enforcement_and_custody_match_the_owner_selection() {
        use {Enforcement::*, WalletCustody::*};
        let table: Vec<_> = WALLET_KINDS
            .iter()
            .map(|k| (k.enforcement, k.custody))
            .collect();
        assert_eq!(
            table,
            [
                (Offchain, Keypair),
                (Offchain, Contract),
                (Onchain, Contract),
                (Onchain, Contract)
            ]
        );
    }
    #[test]
    fn only_standard_lists_params_and_they_are_the_canonical_rules() {
        let keys: Vec<_> = crate::RULES.iter().map(|rule| rule.key()).collect();
        assert_eq!(STANDARD_PARAMS.to_vec(), keys);
        let registry = crate::policy::canonical_function_registry().unwrap();
        let described: Vec<_> = registry.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(described, keys);
        for kind in &WALLET_KINDS {
            let standard = kind.id == WalletKind::Standard;
            assert_eq!(kind.supported_params.is_empty(), !standard);
            assert_eq!(kind.params_url.is_some(), standard);
        }
        let json = serde_json::to_value(WalletKind::Standard.info()).unwrap();
        assert_eq!(json["id"], "standard");
        assert_eq!(json["supportedParams"][0], "payment_limit");
        assert_eq!(json["paramsUrl"], STANDARD_PARAMS_URL);
    }
}
