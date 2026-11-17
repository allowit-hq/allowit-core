use allowit::prelude::*;
use allowit::AllowitInternetSearch;
/// Check each listed vendor until public sources name one.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    let vendors: Vec<Address> = ctx.params["vendors"].ok();
    let mut checked: i128 = 0;
    for vendor in &vendors {
        checked = checked + 1;
        let found = allowit::internet_search(ctx, &AllowitInternetSearch { query: "vendor registry" });
        if found.is_ok() {
            break;
        }
        assert!(vendor != ctx.request.asset);
    }
    assert!(checked < 10);
    allowit::tokens(ctx, &ctx.params[allowit::TOKENS])?;
    Ok(())
}
