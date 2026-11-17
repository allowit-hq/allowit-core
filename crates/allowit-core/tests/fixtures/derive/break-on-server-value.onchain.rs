use allowit::prelude::*;
/// Check each listed vendor until public sources name one.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    allowit::tokens(ctx, &ctx.params[allowit::TOKENS])?;
    Ok(())
}
