use allowit::prelude::*;
/// Mark the wallet for an amount that no arm names.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    match ctx.request.amount {
        1 => {}
        400 => {},
        _ => {
            assert!(ctx.request.amount != 400);
            allowit::set_state(ctx, "fallback", true)?;
        }
    }
    {}
    Ok(())
}
