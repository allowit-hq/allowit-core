use allowit::prelude::*;
use allowit::AllowitMarketplaceSearch;
const CATALOG: &str = "vault reports";
const DAILY: &str = "daily_limit";
/// The limits of this policy.
fn limits(ctx: &AllowitContext) -> allowit::Result {
    allowit::daily_spending_limit(ctx, &ctx.params[DAILY].ok())?;
    Ok(())
}
/// The catalog search of this policy.
fn catalog(ctx: &AllowitContext) -> allowit::Result {
    allowit::marketplace_search(ctx, &AllowitMarketplaceSearch { query: CATALOG })?;
    Ok(())
}
fn listed(ctx: &AllowitContext) -> allowit::Result {
    catalog(ctx)?;
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    Ok(())
}
/// Search the catalog before paying. Limit the day and every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    limits(ctx)?;
    listed(ctx)?;
    Ok(())
}
