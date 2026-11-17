use allowit::prelude::*;
/// Check the seller with my screening package. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let checked = sanctions::screen(ctx, &AllowitValue::Addr(ctx.request.recipient.clone()))?; // a package call
    assert!(checked == AllowitValue::Bool(true));
    let day = u64::from(7u32);
    assert!(day > 0);
    Ok(())
}
