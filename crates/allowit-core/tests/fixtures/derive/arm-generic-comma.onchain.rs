use allowit::AllowitContext;
/// Keep whether the server passed an amount of 1.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    match ctx.request.amount {
        1 => {},
        _ => {}
    }
    Ok(())
}
