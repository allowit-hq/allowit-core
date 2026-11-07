//! Common policy wire values and canonical-function metadata.
//!
//! Addresses remain opaque text. Native adapters validate their address formats and supply
//! verified amount units. These codecs grant no policy admission or signing authority.
use alloc::{string::String, vec::Vec};
use allowit::AllowitValue;
use serde::{Deserialize, Serialize, de::Error, ser::SerializeMap, ser::SerializeSeq};

/// A JSON view of the existing portable value, without a second semantic value enum.
/// Integers serialize as canonical decimal strings, including the full signed i128 range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyValue(pub AllowitValue);

impl From<AllowitValue> for PolicyValue {
    fn from(value: AllowitValue) -> Self {
        Self(value)
    }
}

impl PolicyValue {
    /// Recover the existing portable value. Its address text still needs native validation.
    pub fn into_inner(self) -> AllowitValue {
        self.0
    }
}

struct ValueRef<'a>(&'a AllowitValue);
struct ListRef<'a>(&'a [AllowitValue]);

impl Serialize for ListRef<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut list = serializer.serialize_seq(Some(self.0.len()))?;
        for item in self.0 {
            list.serialize_element(&ValueRef(item))?;
        }
        list.end()
    }
}

impl Serialize for ValueRef<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        match self.0 {
            AllowitValue::Int(value) => map.serialize_entry("Int", &value.to_string())?,
            AllowitValue::Bool(value) => map.serialize_entry("Bool", value)?,
            AllowitValue::Str(value) => map.serialize_entry("Str", value.as_str())?,
            AllowitValue::Addr(value) => map.serialize_entry("Addr", value.as_str())?,
            AllowitValue::List(items) => map.serialize_entry("List", &ListRef(items.as_slice()))?,
        }
        map.end()
    }
}

impl Serialize for PolicyValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ValueRef(&self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PolicyValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = PolicyValue;
            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("one Int, Bool, Str, Addr or List entry")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let key = map
                    .next_key::<String>()?
                    .ok_or_else(|| A::Error::custom("A policy value needs one type entry"))?;
                let value = match key.as_str() {
                    "Int" => {
                        let text = map.next_value::<String>()?;
                        let number = text.parse::<i128>().map_err(A::Error::custom)?;
                        if number.to_string() != text {
                            return Err(A::Error::custom("Int requires canonical decimal text"));
                        }
                        AllowitValue::Int(number)
                    }
                    "Bool" => AllowitValue::Bool(map.next_value()?),
                    "Str" => AllowitValue::Str(map.next_value::<String>()?.into()),
                    "Addr" => {
                        AllowitValue::Addr(allowit::Address::new(map.next_value::<String>()?))
                    }
                    "List" => {
                        let values = map.next_value::<Vec<PolicyValue>>()?;
                        AllowitValue::List(
                            values.into_iter().map(PolicyValue::into_inner).collect(),
                        )
                    }
                    other => {
                        return Err(A::Error::unknown_variant(
                            other,
                            &["Int", "Bool", "Str", "Addr", "List"],
                        ));
                    }
                };
                if map.next_key::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(A::Error::custom(
                        "A policy value needs exactly one type entry",
                    ));
                }
                Ok(PolicyValue(value))
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

/// Exact metadata bytes shared by Rust and the JavaScript package.
pub const CANONICAL_FUNCTIONS_JSON: &str = include_str!("../resources/canonical-functions.json");

/// Inclusive decimal bounds. Text prevents loss through a JavaScript number.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegerRange {
    pub min: String,
    pub max: String,
}

/// Meaning and storage shape of a canonical function's second argument.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterDescriptor {
    /// Zero-based argument index. Canonical rule values use index 1.
    pub index: u8,
    pub semantic_type: String,
    pub storage_types: Vec<String>,
    pub item_type: Option<String>,
    pub integer_range: Option<IntegerRange>,
    /// Amount controls require unit metadata from the bound host and native adapter.
    pub requires_amount_context: bool,
    /// Canonical expiry values use Unix seconds. Relative input belongs to source bindings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wildcard: Option<String>,
}

/// The value returned when a canonical function passes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultDescriptor {
    pub kind: String,
    pub description: String,
}

/// A canonical function's common description, parameter and result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalFunctionDescriptor {
    pub name: String,
    pub path: String,
    pub label: String,
    pub description: String,
    pub parameter: ParameterDescriptor,
    pub result: ResultDescriptor,
}

/// A versioned registry. Its ordering does not prescribe authored source or workflow ordering.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanonicalFunctionRegistry {
    pub schema_version: u32,
    pub core_version: String,
    pub integer_range: IntegerRange,
    pub functions: Vec<CanonicalFunctionDescriptor>,
}

/// Read the metadata compiled from this Core source.
pub fn canonical_function_registry() -> Result<CanonicalFunctionRegistry, serde_json::Error> {
    serde_json::from_str(CANONICAL_FUNCTIONS_JSON)
}

/// Bind a presentation to the exact common metadata bytes.
pub fn canonical_function_metadata_hash() -> String {
    crate::digest(CANONICAL_FUNCTIONS_JSON.as_bytes())
}
