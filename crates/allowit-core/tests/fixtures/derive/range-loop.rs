use allowit::prelude::*;
/// Keep the server's last answer. Count to a bound first.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let passed: bool = false;
    for i in 0u64.. {
        if i > 2 {
            break;
        }
    }
    let passed = allowit::new_decision_context(ctx, None).is_ok();
    allowit::set_state(ctx, "passed_last", passed)?;
    Ok(())
}
