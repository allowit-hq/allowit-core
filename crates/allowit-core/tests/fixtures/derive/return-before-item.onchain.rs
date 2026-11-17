use allowit::prelude::*;
/// Limit every payment, then start the decision context.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    limit(ctx)?;
    return Ok(());
    fn limit(ctx: &AllowitContext) -> allowit::Result {
        allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
        Ok(())
    }
}
