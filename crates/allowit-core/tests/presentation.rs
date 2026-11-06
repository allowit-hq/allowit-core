use allowit_core::policy::{PolicyValue, canonical_function_registry};
use allowit_core::{Address, AllowitMap, AllowitRule, AllowitTier, AllowitValue};

#[test]
fn wire_values_keep_exact_integers_and_opaque_native_identities() {
    let values = [
        AllowitValue::Int(i128::MIN),
        AllowitValue::Int(i128::MAX),
        AllowitValue::Int(0),
        AllowitValue::Bool(false),
        AllowitValue::Str("common text".into()),
        AllowitValue::Addr(Address::new("0xabc123")),
        AllowitValue::List(vec![AllowitValue::Int(9_007_199_254_740_993)].into()),
    ];
    for value in values {
        let encoded = serde_json::to_string(&PolicyValue(value.clone())).unwrap();
        let decoded: PolicyValue = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.into_inner(), value);
    }
    assert_eq!(
        serde_json::to_string(&PolicyValue(AllowitValue::Int(i128::MAX))).unwrap(),
        format!(r#"{{"Int":"{}"}}"#, i128::MAX)
    );
}

#[test]
fn wire_decoder_rejects_rounded_noncanonical_and_ambiguous_values() {
    for input in [
        r#"{"Int":9007199254740993}"#,
        r#"{"Int":"+1"}"#,
        r#"{"Int":"01"}"#,
        r#"{"Int":"-0"}"#,
        r#"{"Int":"1.0"}"#,
        r#"{"Int":"1e2"}"#,
        r#"{"Int":" 1"}"#,
        r#"{"Int":"170141183460469231731687303715884105728"}"#,
        r#"{"Int":"-170141183460469231731687303715884105729"}"#,
        r#"{}"#,
        r#"{"Int":"1","Bool":true}"#,
        r#"{"Int":"1","Int":"2"}"#,
        r#"{"List":[{"Int":1}]}"#,
        r#"{"Unknown":"1"}"#,
    ] {
        assert!(
            serde_json::from_str::<PolicyValue>(input).is_err(),
            "{input}"
        );
    }
}

#[test]
fn function_descriptors_match_executable_rules_and_integer_bounds() {
    let registry = canonical_function_registry().unwrap();
    assert_eq!(registry.core_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(registry.schema_version, 1);
    assert_eq!(registry.integer_range.min, i128::MIN.to_string());
    assert_eq!(registry.integer_range.max, i128::MAX.to_string());
    assert_eq!(
        registry
            .functions
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        allowit_core::RULES.map(AllowitRule::key)
    );
    for descriptor in registry.functions {
        assert_eq!(descriptor.path, format!("allowit::{}", descriptor.name));
        assert_eq!(descriptor.parameter.index, 1);
        if descriptor.name == "expires_at" {
            assert_eq!(
                descriptor.parameter.time_unit.as_deref(),
                Some("unix_seconds")
            );
        } else {
            assert_eq!(descriptor.parameter.time_unit, None);
        }
        if let Some(range) = descriptor.parameter.integer_range {
            let min = range.min.parse::<i128>().unwrap();
            let max = range.max.parse::<i128>().unwrap();
            for bound in [min, max] {
                let params: AllowitMap =
                    [(descriptor.name.clone(), AllowitValue::Int(bound))].into();
                assert!(allowit_core::check_params(&params, AllowitTier::Standard).is_ok());
            }
            let below: AllowitMap = [(descriptor.name.clone(), AllowitValue::Int(min - 1))].into();
            assert!(allowit_core::check_params(&below, AllowitTier::Standard).is_err());
            if let Some(above) = max.checked_add(1) {
                let params: AllowitMap = [(descriptor.name, AllowitValue::Int(above))].into();
                assert!(allowit_core::check_params(&params, AllowitTier::Standard).is_err());
            }
        }
    }
    for value in [
        AllowitValue::Str("*".into()),
        AllowitValue::List(vec![AllowitValue::Str("transfer".into())].into()),
    ] {
        let params = [("operations".into(), value)].into();
        assert!(allowit_core::check_params(&params, AllowitTier::Standard).is_ok());
    }
}
