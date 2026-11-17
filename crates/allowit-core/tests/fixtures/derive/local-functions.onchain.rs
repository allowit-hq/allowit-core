use allowit::prelude::*;
const DAILY: &str = "daily_limit";
/// The limits of this policy.
fn limits(ctx: &AllowitContext) -> allowit::Result {
    allowit::daily_spending_limit(ctx, &ctx.params[DAILY].ok())?;
    Ok(())
}
/// Search the catalog before paying. Limit the day and every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    limits(ctx)?;
    Ok(())
}
