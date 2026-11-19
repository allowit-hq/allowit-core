use allowit::prelude::*;
use allowit::AllowitDecision;
/// Count every payment. Count the payments that the question passed, and flag a low score.
fn count(ctx: &AllowitContext) -> allowit::Result {
    let paid: u64 = allowit::get_state(ctx, "paid")?.unwrap_or(0);
    allowit::set_state(ctx, "paid", paid + 1)
}
fn pass(ctx: &AllowitContext) -> allowit::Result {
    let passed: u64 = allowit::get_state(ctx, "passed")?.unwrap_or(0);
    allowit::set_state(ctx, "passed", passed + 1)
}
fn flag(ctx: &AllowitContext) -> allowit::Result {
    pass(ctx)?;
    allowit::set_state(ctx, "flagged", true)
}
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    count(ctx)?;
    let flagged: bool = allowit::get_state(ctx, "flagged")?.unwrap_or(false);
    if flagged {
        return Err(AllowitError::DestinationNotAllowed);
    }
    let paid: u64 = allowit::get_state(ctx, "paid")?.unwrap_or(0);
    let score = allowit::decision(ctx, &AllowitDecision { id: "a", query: "Is this research?", deny_below: 1, allow_above: 2 })?;
    if score < 5000 && paid > 2 {
        flag(ctx)?;
    }
    if ctx.request.amount > 100 {
        pass(ctx)?;
    }
    allowit::destinations(ctx, &ctx.params[allowit::DESTINATIONS])?;
    Ok(())
}
