use allowit::prelude::*;
use allowit::AllowitDecision;
/// Limit every payment, then decide.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    match allowit::decision(ctx, &AllowitDecision { id: "a", query: "Is this research?", deny_below: 1, allow_above: 2 }) {
        Ok(_) => Ok(()),
        Err(error) => Err(error),
    }
}
