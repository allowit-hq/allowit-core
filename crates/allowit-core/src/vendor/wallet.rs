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
/// The text-message signature of a rail: the signature that its browser wallets make of a text
/// message. A server signs its own statements with it, such as the request of a signed fetch,
/// and anyone verifies them with the server's public identity.
pub trait MessageSignature: Sync {
    /// Sign `message` as a wallet of this rail signs a text message.
    fn sign_message(&self, signer: &dyn Signer, message: &str) -> Result<Vec<u8>>;
    /// Verify the signature of `address` over `message`, as [`Self::sign_message`] made it.
    fn verify_message(
        &self,
        network: &str,
        address: &str,
        message: &str,
        signature: &[u8],
    ) -> Result<()>;
    /// The name of this signature scheme, which a host states beside a statement that it asks a
    /// wallet to sign, so that the wallet signs it as this rail's text message. The default
    /// refuses: a rail names its scheme.
    fn method(&self) -> Result<&'static str> {
        Err(crate::client::error::Error::unsupported(
            "This rail does not name its text-message signature",
        ))
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    /// A rail that does not name its text-message signature is refused, never given a name.
    #[test]
    fn a_signature_without_a_name_is_refused() {
        struct Unnamed;
        impl MessageSignature for Unnamed {
            fn sign_message(&self, _: &dyn Signer, _: &str) -> Result<Vec<u8>> {
                unimplemented!()
            }
            fn verify_message(&self, _: &str, _: &str, _: &str, _: &[u8]) -> Result<()> {
                unimplemented!()
            }
        }
        let refused = Unnamed.method().unwrap_err();
        assert_eq!(
            refused.message,
            "This rail does not name its text-message signature"
        );
    }
}
