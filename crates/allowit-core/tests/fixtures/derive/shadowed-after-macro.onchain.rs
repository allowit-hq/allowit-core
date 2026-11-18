use allowit::prelude::*;
/// Keep the score that the question gave.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    r#assert! { ctx.request.amount > 0 }
    Ok(())
}
