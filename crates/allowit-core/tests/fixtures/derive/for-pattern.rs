use allowit::prelude::*;
/// Keep the amount of a request that the server passed.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    for AllowitRequest { amount, .. } in [&ctx.request] {
        if allowit::new_decision_context(ctx, None).is_ok() {
            allowit::set_state(ctx, "flag", *amount)?;
        }
    }
    Ok(())
}
