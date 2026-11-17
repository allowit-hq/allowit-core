use allowit::prelude::*;
use allowit::{AllowitDecision, AllowitValue};
/// Small payments pass when the seller looks known. Never pay more than my limit.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let score = allowit::decision(
        ctx,
        &AllowitDecision { id: "seller", query: "Is the seller known?", deny_below: 1000, allow_above: 6000 },
    )?;
    if score < 9000 {
        // "Small payments pass"
        allowit::payment_limit(ctx, &AllowitValue::Int(5_000_000))?;
        assert!(ctx.request.amount > 0);
    }
    allowit::destinations(ctx, &ctx.params[allowit::DESTINATIONS])?;
    Ok(())
}
