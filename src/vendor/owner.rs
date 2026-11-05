//! Owner capability inputs. Keys remain caller-owned.
use super::wallet::Signer;
use crate::client::error::Result;
use std::path::Path;
pub struct Context<'a> {
    pub network: &'a str,
    pub backend: &'a str,
    pub authority: &'a crate::vendor::config::ServerAuthority,
    pub origin: &'a str,
    pub rpc_url: &'a str,
    pub root: &'a Path,
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
