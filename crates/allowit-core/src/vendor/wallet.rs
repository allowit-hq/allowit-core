//! Shared signer capability. Native signature rules stay with each vendor.
use crate::client::error::Result;
use zeroize::Zeroizing;
pub trait Signer: Send + Sync {
    fn network(&self) -> &str;
    fn address(&self) -> &str;
    fn account(&self) -> &str {
        self.address()
    }
    fn sign_hash(&self, digest: &[u8]) -> Result<Vec<u8>>;
}
pub trait PrivateSigner: Send + Sync {
    fn address(&self) -> &str;
    fn public_key(&self) -> &str;
    fn key_type(&self) -> &str;
    fn encoded_secret(&self) -> Zeroizing<String>;
    fn sign_hash(&self, digest: &[u8]) -> Result<Vec<u8>>;
}
pub trait KeyProvider: Sync {
    fn entropy_bytes(&self) -> usize;
    fn derive(&self, entropy: &[u8]) -> Result<Box<dyn PrivateSigner>>;
    fn parse(&self, secret: &str) -> Result<Box<dyn PrivateSigner>>;
    fn validate_identity(&self, address: &str, public_key: &str, key_type: &str) -> Result<()>;
}
