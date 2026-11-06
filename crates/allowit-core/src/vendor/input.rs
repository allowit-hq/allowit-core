//! Shared asset and native format capability contracts.
pub mod asset {
    pub use super::super::input_asset::*;
}
pub type Check = (fn(&str) -> bool, &'static str);
pub struct Formats {
    pub account: Check,
    pub wallet: Check,
    pub wallet_error: fn(&str) -> Option<String>,
    pub recipient: Check,
    pub token: Check,
    pub signing_digest: fn(&str) -> [u8; 32],
    pub account_key: fn(&str) -> Option<[u8; 32]>,
    pub address_candidate: fn(&str) -> bool,
    pub resolve_asset:
        fn(&str, &str, Option<&str>, bool) -> crate::client::error::Result<asset::ResolvedAsset>,
    pub same_wallet: fn(&str, &str) -> bool,
    pub asset_code: fn(&str) -> bool,
    pub asset_contract: fn(&str) -> Option<u8>,
    pub issuer_decimals: Option<u8>,
}
