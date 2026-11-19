use allowit::prelude::*;
/// Remember the last score, and stop after a score of zero.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::set_state(ctx, "seen", true)?;
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    Ok(())
}
