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
        if !(5..=3000).contains(&self.prompt.len()) {
            return Err(Error::usage("Describe the policy in 5–3000 UTF-8 bytes"));
        }
        if self.profile.is_empty()
            || self.profile.len() > 128
            || !self
                .profile
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        {
            return Err(Error::usage("Select an advertised wallet profile"));
        }
        if let Target::Installation(id) = &self.target
            && (id.len() != 64
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        {
            return Err(Error::config("Invalid lowercase installation identity"));
        }
        Ok(())
    }
}
