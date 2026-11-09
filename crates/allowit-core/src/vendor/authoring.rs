//! Portable source identity and validation shared by host adapters.
use crate::client::error::{Error, Result};
use crate::policy_source;
const MAX_SOURCE_BYTES: usize = 65_536;
/// Hash canonical common data with Core's digest.
pub fn canonical_digest(value: &impl serde::Serialize) -> Result<String> {
    serde_json_canonicalizer::to_vec(value)
        .map(|bytes| crate::digest(&bytes))
        .map_err(|_| Error::config("Cannot encode canonical policy data"))
}
/// Exact portable source/parameter identity shared by authoring and imported profiles.
pub fn portable_identity(
    language: &str,
    tier: &str,
    rules: &[String],
    source_hash: &str,
    params_hash: &str,
) -> Result<String> {
    canonical_digest(
        &serde_json::json!({"language":language,"tier":tier,"rules":rules,
        "sourceHash":source_hash,"paramsHash":params_hash}),
    )
}

pub fn check(source: &str) -> Result<policy_source::SourceShape> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(Error::usage("Portable policy source exceeds 65536 bytes"));
    }
    policy_source::source_shape(source).map_err(Error::usage)
}
