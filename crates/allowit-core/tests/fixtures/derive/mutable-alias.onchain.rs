use allowit::prelude::*;
/// Count one step when the decision context starts. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    allowit::tokens(ctx, &ctx.params[allowit::TOKENS])?;
    Ok(())
}
