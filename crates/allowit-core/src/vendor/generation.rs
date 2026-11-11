#[derive(Clone, Debug)]
pub enum Target {
    NewWallet,
    Installation(String),
}
#[derive(Clone, Debug)]
pub struct GenerateRequest {
    pub prompt: String,
    pub network: String,
    pub profile: String,
    pub target: Target,
    /// The wallet kind that the owner chose. The rail sends it to the backend, which binds it or
    /// refuses with a typed error. `None` lets the backend propose a kind.
    pub wallet_kind: Option<WalletKind>,
}
/// A fully checked backend document and its remaining required values.
/// The selected capability validates its native binding before returning this document.
pub struct GeneratedDraft {
    pub document: serde_json::Value,
    pub unresolved: Vec<String>,
}
impl GeneratedDraft {
    /// The key of the wallet kind at the top level of a draft, policy or wallet document.
    pub const WALLET_KIND_KEY: &'static str = "walletKind";
    /// The wallet kind that the backend bound to this draft, if the document names one.
    pub fn wallet_kind(&self) -> Option<WalletKind> {
        wallet_kind_of(&self.document)
    }
}
/// The wallet kind at the top level of a draft, policy or wallet document, if it names one.
pub fn wallet_kind_of(document: &serde_json::Value) -> Option<WalletKind> {
    WalletKind::parse(document.get(GeneratedDraft::WALLET_KIND_KEY)?.as_str()?)
}

use super::error::{Error, Result};
use super::kinds::WalletKind;
impl GenerateRequest {
    pub fn validate(&self) -> Result<()> {
        self.validate_with_network(|_| Ok(()))
    }
    pub fn validate_with_network(
        &self,
        validate_network: impl FnOnce(&str) -> Result<()>,
    ) -> Result<()> {
        validate_fields(
            &self.prompt,
            &self.network,
            &self.profile,
            &self.target,
            validate_network,
        )
    }
}

/// Validate common fields in order, with network admission supplied by the selected host.
pub fn validate_fields(
    prompt: &str,
    network: &str,
    profile: &str,
    target: &Target,
    validate_network: impl FnOnce(&str) -> Result<()>,
) -> Result<()> {
    if !(5..=3000).contains(&prompt.len()) {
        return Err(Error::usage("Describe the policy in 5–3000 UTF-8 bytes"));
    }
    validate_network(network)?;
    if profile.is_empty()
        || profile.len() > 128
        || !profile
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    {
        return Err(Error::usage("Select an advertised wallet profile"));
    }
    if let Target::Installation(id) = target
        && (id.len() != 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    {
        return Err(Error::config("Invalid lowercase installation identity"));
    }
    Ok(())
}

impl<'a> From<&'a GenerateRequest> for std::borrow::Cow<'a, GenerateRequest> {
    fn from(request: &'a GenerateRequest) -> Self {
        Self::Borrowed(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_draft_reports_the_wallet_kind_that_its_document_names() {
        let draft = |document| GeneratedDraft {
            document,
            unresolved: Vec::new(),
        };
        for kind in WalletKind::ALL {
            let document = serde_json::json!({"id": "p", GeneratedDraft::WALLET_KIND_KEY: kind});
            assert_eq!(draft(document).wallet_kind(), Some(kind));
        }
        // Ids are exact, and a document without one names no kind.
        for document in [
            serde_json::json!({"id": "p"}),
            serde_json::json!({"walletKind": "common"}),
            serde_json::json!({"walletKind": "Standard"}),
            serde_json::json!({"walletKind": 3}),
            serde_json::json!({"runtime": {"walletKind": "standard"}}),
        ] {
            assert_eq!(draft(document).wallet_kind(), None);
        }
    }
}
