use allowit::prelude::*;
use allowit::AllowitDecision;
/// Approve at once what the question passes. Mark the wallet on every other payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let visits: u64 = allowit::get_state(ctx, "visits")?.unwrap_or(0);
    allowit::set_state(ctx, "visits", visits + 1)?;
    let score = allowit::decision(ctx, &AllowitDecision { id: "a", query: "Is this research?", deny_below: 1, allow_above: 2 })?;
    if score > 5000 {
        return Ok(());
    }
    const { true }.then_some(allowit::set_state(ctx, "marked", true)?);
    Ok(())
}
