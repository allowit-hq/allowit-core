use allowit::prelude::*;
use allowit::{AllowitDecision, AllowitInternetSearch};
/// Ask the question of the operation. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    let operation = allowit::operations(ctx, &ctx.params[allowit::OPERATIONS])?;
    let id = match operation {
        "transfer" => {
            allowit::internet_search(ctx, &AllowitInternetSearch { query: "seller news" })?;
            "purchase"
        }
        _ => "position",
    };
    match operation {
        "transfer" => allowit::payment_limit(ctx, &ctx.params["report_limit"].ok())?,
        _ => i128::from(allowit::decision(ctx, &AllowitDecision { id: id, query: "Is this safe?", deny_below: 1, allow_above: 2 })?),
    };
    match operation {
        "transfer" => allowit::new_decision_context(ctx, None)?,
        "redeem" => assert!(ctx.request.min_out > 0),
        _ => {
            allowit::tokens(ctx, &ctx.params[allowit::TOKENS])?;
            allowit::new_decision_context(ctx, Some("position"))?;
        }
    }
    Ok(())
}
