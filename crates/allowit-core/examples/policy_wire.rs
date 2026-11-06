//! Wire values for the actual JavaScript package interoperability check.
use allowit_core::policy::{PolicyValue, canonical_function_metadata_hash};
use allowit_core::{Address, AllowitValue};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("roundtrip") {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        let values: Vec<PolicyValue> = serde_json::from_str(&input)?;
        println!("{}", serde_json::to_string(&values)?);
        return Ok(());
    }
    let values: Vec<PolicyValue> = vec![
        AllowitValue::Int(i128::MIN),
        AllowitValue::Int(i128::MAX),
        AllowitValue::Int(0),
        AllowitValue::Bool(true),
        AllowitValue::Str("policy text".into()),
        AllowitValue::Addr(Address::new("0xabc123")),
        AllowitValue::List(vec![AllowitValue::Int(9_007_199_254_740_993)].into()),
    ]
    .into_iter()
    .map(PolicyValue::from)
    .collect();
    println!(
        "{}",
        serde_json::json!({"values": values, "metadataHash": canonical_function_metadata_hash()})
    );
    Ok(())
}
