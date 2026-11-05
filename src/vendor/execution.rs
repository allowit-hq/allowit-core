//! Common execution inputs and results.
use super::input::asset::Amount;
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
