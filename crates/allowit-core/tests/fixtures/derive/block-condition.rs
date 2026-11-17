use allowit::prelude::*;
/// A statement whose condition holds a block ends with its own last block.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    if { ctx.request.amount > 0 } {
        allowit::new_decision_context(ctx, None)?;
    }
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    if ctx.request.amount == { 1 } {
        allowit::new_decision_context(ctx, None)?;
    } else if { ctx.request.amount > 2 } {
        allowit::new_decision_context(ctx, None)?;
    } else {
        assert!(ctx.request.amount >= 0);
    }
    allowit::destinations(ctx, &ctx.params[allowit::DESTINATIONS])?;
    Ok(())
}
