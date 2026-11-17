use allowit::prelude::*;
use allowit::AllowitPrice;
/// Keep the priced total under my cap.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    let cap: i128 = ctx.params["cap"].ok();
    let mut total: i128 = ctx.request.amount;
    let quote: Address = ctx.params["quote"].ok();
    total = total + allowit::price(
        ctx,
        &AllowitPrice { from: ctx.request.asset.clone(), to: quote, amount: 1, price: 1_000_000, slip: 100 },
    )?;
    assert!(total <= cap);
    allowit::expires_at(ctx, &ctx.params[allowit::EXPIRES_AT])?;
    Ok(())
}
