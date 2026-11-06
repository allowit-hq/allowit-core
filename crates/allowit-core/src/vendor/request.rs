//! Opaque host request identity validation.
pub fn request_id_valid(value: &str) -> bool {
    (8..=100).contains(&value.len())
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._:-".contains(&c))
}
