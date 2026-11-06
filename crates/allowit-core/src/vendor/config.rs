//! Caller-configured backend authority. Native identity checks belong to the adapter.
pub use crate::client::config::parse_origin;
use crate::client::error::{Error, Result};
/// Independently configured public authority for one backend and selected rail.
#[derive(Clone, Debug)]
pub struct ServerAuthority {
    network: String,
    origin: String,
    public_key: String,
}
impl ServerAuthority {
    /// Construct after the host validates the canonical network and native public identity.
    pub fn validated(network: &str, origin: &str, public_key: &str) -> Result<Self> {
        if network.is_empty() || public_key.is_empty() || origin.is_empty() {
            return Err(Error::config(
                "Configure an explicit server origin and public identity",
            ));
        }
        Ok(Self {
            network: network.into(),
            origin: parse_origin(origin)?,
            public_key: public_key.into(),
        })
    }
    pub fn network(&self) -> &str {
        &self.network
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn public_key(&self) -> &str {
        &self.public_key
    }
}
