use super::*;
use alloc::collections::BTreeMap;

fn context() -> AllowitContext {
    AllowitContext {
        env: AllowitEnv::new(),
        params: BTreeMap::new().into(),
        state: BTreeMap::new(),
        request: AllowitRequest {
            operation: AllowitOperation::transfer,
            asset: Address::new("asset"),
            recipient: Address::new("recipient"),
            amount: 40,
            token_in: Address::new("asset"),
            amount_in: 40,
            token_out: None,
            min_out: 0,
            now: DAY + 100,
            expires_at: DAY + 500,
            spent_units: 40,
            gas_units: 0,
            unpriced: false,
            args: BTreeMap::new().into(),
        },
    }
}

#[test]
fn denied_and_panicked_host_runs_cannot_leak_writes_into_a_later_approval() {
    let ctx = context();
    for denial in [
        AllowitError::PaymentLimitExceeded,
        AllowitError::Panicked,
        AllowitError::MissingParam,
    ] {
        let result = evaluate(
            &ctx,
            |ctx| {
                set_state(ctx, "memo", 9_i128)?;
                match denial {
                    AllowitError::Panicked => panic!("authored policy panic"),
                    AllowitError::MissingParam => {
                        let _ = &ctx.params["absent"];
                        Ok(())
                    }
                    _ => Err(denial),
                }
            },
            AllowitTier::Custom,
        );
        assert_eq!(result, Err(denial));
        let state = evaluate(&ctx, |_| Ok(()), AllowitTier::Custom).unwrap();
        assert!(!state.contains_key("memo"));
        assert_eq!(state.get("spent"), Some(&AllowitValue::Int(40)));
        assert_eq!(state.get("calls"), Some(&AllowitValue::Int(1)));
        assert!(ctx.state.is_empty());
    }
    assert_eq!(
        evaluate(
            &ctx,
            |ctx| set_state(ctx, "memo", 9_i128),
            AllowitTier::Standard
        ),
        Err(AllowitError::Unsupported),
    );
    let state = evaluate(&ctx, |_| Ok(()), AllowitTier::Custom).unwrap();
    assert!(!state.contains_key("memo"));
}

#[test]
fn the_decision_functions_are_stubs_that_only_a_server_fills() {
    let ctx = context();
    let question = AllowitDecision {
        id: "report_purchase",
        query: "Is this a research purchase?",
        deny_below: 3_000,
        allow_above: 8_000,
    };
    assert_eq!(decision(&ctx, &question), Err(AllowitError::Unsupported));
    for contents in [None, Some("The owner buys research data only.")] {
        assert_eq!(
            new_decision_context(&ctx, contents),
            Err(AllowitError::Unsupported)
        );
    }
}

/// A server policy passes an owner text param to a server function: a typed read yields the
/// portable text, and `as_str()` is the `&str` of a request.
#[test]
fn an_owner_text_param_passes_to_a_server_function_request() {
    let mut ctx = context();
    ctx.params = BTreeMap::from([
        ("web_query".into(), AllowitValue::Str("vault risk".into())),
        (
            "background".into(),
            AllowitValue::Str("The owner holds the Meridian vault.".into()),
        ),
    ])
    .into();
    let query: String = param(&ctx, "web_query");
    let background: String = param(&ctx, "background");
    assert_eq!(query.as_str(), "vault risk");
    assert_eq!(
        internet_search(
            &ctx,
            &AllowitInternetSearch {
                query: query.as_str()
            }
        ),
        Err(AllowitError::Unsupported)
    );
    assert_eq!(
        new_decision_context(&ctx, Some(background.as_str())),
        Err(AllowitError::Unsupported)
    );
    let question = AllowitDecision {
        id: "position",
        query: query.as_str(),
        deny_below: 3_000,
        allow_above: 8_000,
    };
    assert_eq!(decision(&ctx, &question), Err(AllowitError::Unsupported));
}
