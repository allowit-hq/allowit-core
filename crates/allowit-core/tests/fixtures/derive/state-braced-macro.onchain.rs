use allowit::prelude::*;
/// Count every visit. Remember whether the question passed the last payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    assert! { ctx.request.amount > 0 }
    let visits: u64 = allowit::get_state(ctx, "visits")?.unwrap_or(0);
    allowit::set_state(ctx, "visits", visits + 1)?;
    Ok(())
}
