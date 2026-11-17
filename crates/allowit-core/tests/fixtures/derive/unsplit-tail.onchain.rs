use allowit::prelude::*;
/// The last expression of `execute` is a block with a value: it leaves whole.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    Ok(())
}
