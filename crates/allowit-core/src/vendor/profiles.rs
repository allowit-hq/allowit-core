//! Common public installation description.
use crate::policy::PolicyValue;
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Description {
    pub network: String,
    pub wallet: String,
    pub agent: String,
    pub policy_id: String,
    pub owner: String,
    pub server: String,
    pub asset: String,
    pub asset_symbol: String,
    pub source: String,
    pub source_hash: String,
    pub module_digest: String,
    pub params: BTreeMap<String, PolicyValue>,
}

/// Vendor-validated profile state. The SDK owns cache selection and persistence.
pub trait ProfileBinding: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn verify_import(&self, endpoint: &str) -> super::error::Result<()>;
    fn verify_server_role(&self, server: &str) -> super::error::Result<()>;
    /// The wallet kind that this profile binds, when the vendor reports one.
    fn wallet_kind(&self) -> Option<super::kinds::WalletKind> {
        None
    }
}
pub struct Profile {
    pub description: Description,
    pub backend: String,
    pub binding: Box<dyn ProfileBinding>,
    pub descriptor: Box<serde_json::value::RawValue>,
    pub authority: Option<super::config::ServerAuthority>,
}
impl Profile {
    pub fn description(&self) -> &Description {
        &self.description
    }
    /// The wallet kind that this profile binds, when the vendor reports one.
    pub fn wallet_kind(&self) -> Option<super::kinds::WalletKind> {
        self.binding.wallet_kind()
    }
    pub fn server_origin(&self) -> &str {
        &self.backend
    }
    pub fn require_authority(&self) -> super::error::Result<&super::config::ServerAuthority> {
        self.authority.as_ref().ok_or_else(|| {
            super::error::Error::config(
                "Fresh execution requires independently configured server authority",
            )
        })
    }
    pub fn with_authority(
        mut self,
        a: &super::config::ServerAuthority,
    ) -> super::error::Result<Self> {
        if self.description.network != a.network() || self.backend != a.origin() {
            return Err(super::error::Error::config(
                "Configured server authority belongs to another backend or rail",
            ));
        }
        self.binding.verify_server_role(a.public_key())?;
        self.authority = Some(a.clone());
        Ok(self)
    }
    pub fn document(&self) -> super::error::Result<serde_json::Value> {
        serde_json::from_str(self.descriptor.get())
            .map_err(|_| super::error::Error::config("Cannot encode public profile"))
    }
}
