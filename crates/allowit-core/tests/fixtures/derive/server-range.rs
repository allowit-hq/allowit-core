use allowit::prelude::*;
use allowit::AllowitRequestScreening;
/// A range whose bound is a server value leaves with its loop.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let count = allowit::request_screening(ctx, &AllowitRequestScreening { query: "Is this research?", deny_below: 1 })?;
    for i in 0..count {
        assert!(i < 10_000);
    }
    for i in 0..=count {
        assert!(i <= 10_000);
    }
    Ok(())
}
