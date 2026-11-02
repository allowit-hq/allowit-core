//! Rust and Python must reject the same nonportable source and derive the same prefix.
use allowit_sdk::policy_source::source_shape;
use serde_json::Value;

const CANONICAL: &str = include_str!("fixtures/portable-policy.rs");

fn vectors(text: &str) -> Vec<Value> {
    serde_json::from_str(text).expect("vector file is JSON")
}

#[test]
fn source_gate_matches_policy_source_py() {
    let vectors = vectors(include_str!("fixtures/policy-source-vectors.json"));
    assert!(vectors.len() > 300);
    let mut accepted = 0;
    for v in &vectors {
        let head = v["head"].as_u64().unwrap() as usize;
        let tail = v["tail"].as_u64().unwrap() as usize;
        let source = format!(
            "{}{}{}",
            &CANONICAL[..head],
            v["middle"].as_str().unwrap(),
            &CANONICAL[CANONICAL.len() - tail..]
        );
        let shape = source_shape(&source);
        if v["accepted"].as_bool().unwrap() {
            accepted += 1;
            let shape =
                shape.unwrap_or_else(|e| panic!("Python accepts, Rust rejects: {e}\n{source}"));
            let prefix: Vec<String> = serde_json::from_value(v["prefix"].clone()).unwrap();
            assert_eq!(shape.prefix, prefix, "{source}");
            assert_eq!(
                shape.writes_state,
                v["writes_state"].as_bool().unwrap(),
                "{source}"
            );
        } else {
            assert!(
                shape.is_err(),
                "Python rejects ({}), Rust accepts:\n{source}",
                v["error"]
            );
        }
    }
    assert!(accepted > 100 && accepted < vectors.len());
}
