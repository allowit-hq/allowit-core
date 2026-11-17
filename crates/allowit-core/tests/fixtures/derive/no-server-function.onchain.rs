use allowit::prelude::*;
/// Pay only vendors I approve, and never more than my limit.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    // "never more than my limit"
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;


    // "Pay only vendors I approve"
    allowit::destinations(ctx, &ctx.params[allowit::DESTINATIONS])?;
    Ok(())
}
