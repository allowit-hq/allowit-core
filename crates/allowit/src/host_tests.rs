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
