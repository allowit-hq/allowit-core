//! Common execution inputs and results.
use super::input::asset::{Amount, AssetDisplay};
use serde::{Deserialize, Serialize};
use std::time::Duration;
pub struct Request {
    pub op: String,
    pub to: String,
    pub amount: Amount,
    pub token_out: Option<Amount>,
    pub args: Option<serde_json::Value>,
    pub request_id: String,
    pub wait: Duration,
    pub build_only: bool,
    pub wallet: Option<String>,
    /// What the payment buys, when the operation pays a seller's challenge. The rail sends it
    /// to the server with the operation; it never changes the operation itself.
    pub paid_service: Option<super::paid_service::PaidService>,
}
pub struct Outcome {
    pub document: serde_json::Value,
    pub status: crate::vendor::runtime::Status,
}
/// Base units of an asset with the rail's display metadata and the printed text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quantity {
    /// Base units as decimal text, so no JSON reader rounds them.
    pub units: String,
    pub asset: AssetDisplay,
    /// The amount with its unit, from [`AssetDisplay::format`].
    pub text: String,
}
impl Quantity {
    pub fn new(units: i128, asset: AssetDisplay) -> Self {
        Self {
            units: units.to_string(),
            text: asset.format(units),
            asset,
        }
    }
}

/// What a settled operation cost. A provider puts it in [`Outcome::document`] under
/// [`Settlement::KEY`] once it holds a verified receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settlement {
    /// The block or ledger that holds the transaction.
    pub height: u64,
    /// The network fee that the transaction paid, in the fee asset.
    pub fee: Quantity,
    /// The amount that the operation moved, in the operation's asset.
    pub amount: Option<Quantity>,
}
impl Settlement {
    /// The key of this value in an outcome document.
    pub const KEY: &'static str = "settlement";
    /// The settlement that an outcome document carries, if it has one.
    pub fn of(document: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(document.get(Self::KEY)?.clone()).ok()
    }
}
/// One spending limit of a policy: what it allows, what the policy has spent against it and
/// what remains, each in the asset that the limit counts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitReport {
    /// The policy param that holds the limit, such as `daily_spending_limit`.
    pub param: String,
    pub limit: Quantity,
    pub spent: Quantity,
    pub remaining: Quantity,
}
/// The spending limits that an outcome or wallet document reports under [`Limits::KEY`].
pub struct Limits;
impl Limits {
    /// The key of the limit reports in a document.
    pub const KEY: &'static str = "limits";
    /// The limit reports that a document carries, if it has them.
    pub fn of(document: &serde_json::Value) -> Option<Vec<LimitReport>> {
        serde_json::from_value(document.get(Self::KEY)?.clone()).ok()
    }
}
/// The key of the fee that the limits count for an operation, in the installed asset.
pub const FEE_CHARGED_KEY: &str = "feeCharged";
/// The fee that the spending limits count for the operation of an outcome document, if it
/// reports one. It differs from [`Settlement::fee`], which is the network fee in the fee asset.
pub fn fee_charged(document: &serde_json::Value) -> Option<Quantity> {
    serde_json::from_value(document.get(FEE_CHARGED_KEY)?.clone()).ok()
}
/// The key of the block-explorer link of an operation's transaction.
pub const EXPLORER_KEY: &str = "explorer";
/// The block-explorer link of the transaction of an outcome document, if it has one.
pub fn explorer(document: &serde_json::Value) -> Option<String> {
    document.get(EXPLORER_KEY)?.as_str().map(String::from)
}
pub struct Capabilities {
    pub curve: &'static str,
    pub address_is_key: bool,
    pub operations: &'static [&'static str],
    pub build_only: bool,
    pub verbs: &'static [&'static str],
}
pub struct Binding {
    pub wallet: String,
    pub policy: String,
}

/// Validated retained vendor state. The SDK does not inspect its native records.
pub trait RecoveryBinding: Send + Sync {
    fn binding(&self) -> Binding;
    fn signed(&self) -> bool;
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send + Sync>;
}
pub struct Recovery {
    pub binding: Box<dyn RecoveryBinding>,
}
impl Recovery {
    pub fn binding(&self) -> Binding {
        self.binding.binding()
    }
    pub fn signed(&self) -> bool {
        self.binding.signed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_settlement_round_trips_through_an_outcome_document() {
        let asset = AssetDisplay {
            symbol: "XYZ".into(),
            decimals: 7,
            name: "Example".into(),
            native: true,
        };
        let settlement = Settlement {
            height: 42,
            fee: Quantity::new(1_234, asset.clone()),
            amount: Some(Quantity::new(50_000_000, asset)),
        };
        let document = serde_json::json!({"requestId": "r", Settlement::KEY: settlement});
        assert_eq!(document["settlement"]["height"], 42);
        assert_eq!(document["settlement"]["fee"]["units"], "1234");
        assert_eq!(document["settlement"]["fee"]["text"], "0.0001234 XYZ");
        assert_eq!(document["settlement"]["amount"]["text"], "5 XYZ");
        assert_eq!(Settlement::of(&document), Some(settlement));
        assert_eq!(Settlement::of(&serde_json::json!({"requestId": "r"})), None);
    }
    #[test]
    fn limits_the_charged_fee_and_the_explorer_link_read_from_an_outcome_document() {
        let asset = AssetDisplay {
            symbol: "XYZ".into(),
            decimals: 2,
            name: "Example".into(),
            native: false,
        };
        let quantity = |units| Quantity::new(units, asset.clone());
        let limits = vec![LimitReport {
            param: "daily_spending_limit".into(),
            limit: quantity(5_000),
            spent: quantity(1_200),
            remaining: quantity(3_800),
        }];
        let document = serde_json::json!({"requestId": "r", Limits::KEY: limits,
            FEE_CHARGED_KEY: quantity(7), EXPLORER_KEY: "https://explorer.example/tx/ab"});
        assert_eq!(document["limits"][0]["param"], "daily_spending_limit");
        assert_eq!(document["limits"][0]["remaining"]["text"], "38 XYZ");
        assert_eq!(document["feeCharged"]["units"], "7");
        assert_eq!(Limits::of(&document), Some(limits));
        assert_eq!(fee_charged(&document), Some(quantity(7)));
        assert_eq!(
            explorer(&document).as_deref(),
            Some("https://explorer.example/tx/ab")
        );
        // An absent or malformed value reads as none; it never becomes a zero or an empty list.
        for document in [
            serde_json::json!({"requestId": "r"}),
            serde_json::json!({"limits": [{"param": "p"}], "feeCharged": "7", "explorer": 1}),
        ] {
            assert_eq!(Limits::of(&document), None);
            assert_eq!(fee_charged(&document), None);
            assert_eq!(explorer(&document), None);
        }
    }
}
