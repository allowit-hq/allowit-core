use allowit::prelude::*;
/// A screened seller needs no allowlist. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    allowit::tokens(ctx, &ctx.params[allowit::TOKENS])?;
    Ok(())
}
