use allowit::prelude::*;
use allowit::AllowitDecision;
/// Remember the last score, and stop after a score of zero.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::set_state(ctx, "seen", true)?;
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let last: Option<u32> = allowit::get_state(ctx, "last_score")?;
    assert!(last != Some(0));
    let score = allowit::decision(ctx, &AllowitDecision { id: "a", query: "Is this research?", deny_below: 1, allow_above: 2 })?;
    allowit::set_state(ctx, "last_score", score)?;
    Ok(())
}
