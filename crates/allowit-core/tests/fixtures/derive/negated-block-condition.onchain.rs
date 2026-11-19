use allowit::prelude::*;
/// Deny a payment when the server gives no decision context.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    Ok(())
}
