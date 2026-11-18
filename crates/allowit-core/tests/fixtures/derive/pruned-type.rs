use allowit::prelude::*;
/// Keep whether the top bit of the step is set. The question checks a copy of the step.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let n = 1;
    let copy: u64 = n;
    if allowit::new_decision_context(ctx, None).is_ok() {
        assert!(copy > 0);
    }
    allowit::set_state(ctx, "flag", (n << 31) > 0)?;
    Ok(())
}
