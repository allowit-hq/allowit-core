use allowit::{AllowitContext, AllowitError};
/// Keep whether the server passed an amount of 1.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let mut flag: bool = false;
    match ctx.request.amount {
        1 => flag = allowit::new_decision_context(ctx, None).is_ok() == Result::Ok::<(), AllowitError>(()).is_ok(),
        _ => {}
    }
    allowit::set_state(ctx, "flag", flag)?;
    Ok(())
}
