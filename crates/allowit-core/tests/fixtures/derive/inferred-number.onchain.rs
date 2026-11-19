use allowit::prelude::*;
/// Keep whether the top bit of the step is set. The question narrows the step.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    Ok(())
}
