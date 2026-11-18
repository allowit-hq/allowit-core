use allowit::prelude::*;
/// Mark the wallet once, unless the question passes first.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    loop {
        for _ in {
            if allowit::new_decision_context(ctx, None).is_ok() {
                break;
            }
            0u64..1
        } {}
        allowit::set_state(ctx, "seen", true)?;
        break;
    }
    Ok(())
}
