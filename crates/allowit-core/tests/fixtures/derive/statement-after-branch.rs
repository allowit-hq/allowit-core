use allowit::prelude::*;
/// Keep the limit of the policy in its state. A block after a branch asks the server.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    if ctx.request.amount > 0 {} {
        allowit::new_decision_context(ctx, None)?;
    }
    const LIMIT: i128 = 3;
    fn limit() -> i128 {
        LIMIT
    }
    allowit::set_state(ctx, "limit", limit())?;
    assert!(ctx.request.amount < limit() * 1_000_000);
    Ok(())
}
