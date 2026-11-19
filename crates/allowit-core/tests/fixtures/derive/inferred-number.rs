use allowit::prelude::*;
/// Keep whether the top bit of the step is set. The question narrows the step.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let n = if ctx.request.amount > 0 { 1 } else { 2 };
    if allowit::new_decision_context(ctx, None).is_ok() {
        assert!(n == 1u64 || n == 2u64);
    }
    allowit::set_state(ctx, "flag", (n << 31) > 0)?;
    Ok(())
}
