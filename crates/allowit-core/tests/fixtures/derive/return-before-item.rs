use allowit::prelude::*;
/// Limit every payment, then start the decision context.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    limit(ctx)?;
    return allowit::new_decision_context(ctx, None);
    fn limit(ctx: &AllowitContext) -> allowit::Result {
        allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
        Ok(())
    }
}
