use allowit::prelude::*;
use allowit::AllowitRequestScreening;
/// Accept a seller that screens well. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    let accepted: bool;
    let score = allowit::request_screening(
        ctx,
        &AllowitRequestScreening { query: "Is the seller listed?", deny_below: 2000 },
    )?;
    if score >= 5000 {
        accepted = true;
    } else {
        accepted = false;
    }
    assert!(accepted);
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    Ok(())
}
