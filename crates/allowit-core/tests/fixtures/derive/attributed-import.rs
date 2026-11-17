use allowit::prelude::*;
#[allow(unused_imports)]
use allowit::AllowitRequestScreening;
/// Screen the seller. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    allowit::request_screening(
        ctx,
        &AllowitRequestScreening { query: "Is the seller listed?", deny_below: 2000 },
    )?;
    Ok(())
}
