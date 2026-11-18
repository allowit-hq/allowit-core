use allowit::prelude::*;
/// Count every payment. Count the payments that the question passed, and flag a low score.
fn count(ctx: &AllowitContext) -> allowit::Result {
    let paid: u64 = allowit::get_state(ctx, "paid")?.unwrap_or(0);
    allowit::set_state(ctx, "paid", paid + 1)
}
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    count(ctx)?;
    let paid: u64 = allowit::get_state(ctx, "paid")?.unwrap_or(0);
    if ctx.request.amount > 100 {}
    allowit::destinations(ctx, &ctx.params[allowit::DESTINATIONS])?;
    Ok(())
}
