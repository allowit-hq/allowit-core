use allowit::prelude::*;
/// Approve at once what the question passes. Mark the wallet on every other payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let visits: u64 = allowit::get_state(ctx, "visits")?.unwrap_or(0);
    allowit::set_state(ctx, "visits", visits + 1)?;
    Ok(())
}
