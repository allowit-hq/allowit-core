use allowit::prelude::*;
use allowit::AllowitRequestScreening;
/// A screened seller needs no allowlist. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let score = allowit::request_screening(
        ctx,
        &AllowitRequestScreening { query: "Is the seller listed?", deny_below: 2000 },
    )?;
    if score >= 9000 {
        return Ok(());
    }
    allowit::destinations(ctx, &ctx.params[allowit::DESTINATIONS])?;
    allowit::tokens(ctx, &ctx.params[allowit::TOKENS])?;
    Ok(())
}
