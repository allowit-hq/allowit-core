use allowit::prelude::*;
use allowit::AllowitDecision;
/// Keep the score that the question gave.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let score: u32 = 1;
    r#assert! { ctx.request.amount > 0 }
    let score = allowit::decision(ctx, &AllowitDecision { id: "a", query: "Is this research?", deny_below: 1, allow_above: 2 })?;
    allowit::set_state(ctx, "last_score", score)?;
    Ok(())
}
