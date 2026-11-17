use allowit::prelude::*;
/// Count one step when the decision context starts. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let mut total: i128 = 0;
    let alias = &mut total;
    *alias = {
        allowit::new_decision_context(ctx, None)?;
        1
    };
    assert!(total == 1);
    allowit::tokens(ctx, &ctx.params[allowit::TOKENS])?;
    Ok(())
}
