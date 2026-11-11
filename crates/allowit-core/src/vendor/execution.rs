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
}
