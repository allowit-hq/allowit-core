use allowit::prelude::*;
/// Never pay more than my limits. After many payments, pay only my sellers.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let calls: Option<u64> = allowit::get_state(ctx, "calls")?;
    assert!(calls != Some(1_000));
    if calls > Some(10) {
        allowit::destinations(ctx, &ctx.params[allowit::DESTINATIONS])?;
    }
    allowit::daily_spending_limit(ctx, &ctx.params[allowit::DAILY_SPENDING_LIMIT])?;
    Ok(())
}
