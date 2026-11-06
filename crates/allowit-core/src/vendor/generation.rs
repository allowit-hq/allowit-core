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
}
/// A fully checked backend document and its remaining required values.
/// The selected capability validates its native binding before returning this document.
pub struct GeneratedDraft {
    pub document: serde_json::Value,
    pub unresolved: Vec<String>,
}

use super::error::{Error, Result};
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
