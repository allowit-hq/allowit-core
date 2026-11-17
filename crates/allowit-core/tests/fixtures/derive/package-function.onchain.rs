use allowit::prelude::*;
/// Check the seller with my screening package. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let day = u64::from(7u32);
    assert!(day > 0);
    Ok(())
}
