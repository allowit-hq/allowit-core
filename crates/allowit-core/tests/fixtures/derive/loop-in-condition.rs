use allowit::prelude::*;
/// Remember a large payment that the server passed.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    if ctx.request.amount > 5 && allowit::new_decision_context(ctx, None).is_ok() && loop { break true; } {
        allowit::set_state(ctx, "seen", true)?;
    }
    Ok(())
}
