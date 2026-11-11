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
    pub explanation: &'static str,
    pub enforcement: Enforcement,
    pub custody: WalletCustody,
    /// The params this kind enforces onchain. Empty unless the kind has a fixed list.
    pub supported_params: &'static [&'static str],
    /// The public list of `supported_params`.
    pub params_url: Option<&'static str>,
}

/// Every wallet kind, in selection order.
pub const WALLET_KINDS: [WalletKindInfo; 4] = [
    WalletKindInfo {
        id: WalletKind::Native,
        title: "Native",
        explanation: "Allowit enforces the policy offchain, and a keypair account holds the funds.",
        enforcement: Enforcement::Offchain,
        custody: WalletCustody::Keypair,
        supported_params: &[],
        params_url: None,
    },
    WalletKindInfo {
        id: WalletKind::Simple,
        title: "Simple",
        explanation: "Allowit enforces the policy offchain, as for Native, and a smart-contract wallet holds the funds.",
        enforcement: Enforcement::Offchain,
        custody: WalletCustody::Contract,
        supported_params: &[],
        params_url: None,
    },
    WalletKindInfo {
        id: WalletKind::Standard,
        title: "Standard",
        explanation: "A smart-contract wallet enforces the standard policy onchain for the supported params.",
        enforcement: Enforcement::Onchain,
        custody: WalletCustody::Contract,
        supported_params: &STANDARD_PARAMS,
        params_url: Some(STANDARD_PARAMS_URL),
    },
    WalletKindInfo {
        id: WalletKind::Custom,
        title: "Custom",
        explanation: "A custom contract, built from the policy source, enforces the policy onchain.",
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
