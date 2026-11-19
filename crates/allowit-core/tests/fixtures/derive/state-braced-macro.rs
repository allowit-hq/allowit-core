use allowit::prelude::*;
use allowit::AllowitDecision;
/// Count every visit. Remember whether the question passed the last payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    assert! { ctx.request.amount > 0 }
    fn remember(ctx: &AllowitContext, passed: bool) -> allowit::Result {
        allowit::set_state(ctx, "seen", passed)
    }
    remember(ctx, false)?;
    let visits: u64 = allowit::get_state(ctx, "visits")?.unwrap_or(0);
    allowit::set_state(ctx, "visits", visits + 1)?;
    let score = allowit::decision(ctx, &AllowitDecision { id: "a", query: "Is this research?", deny_below: 1, allow_above: 2 })?;
    if score > 5000 {
        remember(ctx, true)?;
    }
    Ok(())
}
