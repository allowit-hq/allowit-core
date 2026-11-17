use allowit::prelude::*;
/// Deny a seller that screens badly. Pay only vendors I approve.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::destinations(ctx, &ctx.params[allowit::DESTINATIONS])?;
    Ok(())
}
