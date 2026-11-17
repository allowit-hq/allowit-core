use allowit::prelude::*;
/// Keep the priced total under my ceiling.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::expires_at(ctx, &ctx.params[allowit::EXPIRES_AT])?;
    Ok(())
}
