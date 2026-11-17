use allowit::prelude::*;
use allowit::{AllowitFetch, AllowitDecision};
const QUESTION: &str = "purchase_question";
const REPORT_LIMIT: &str = "report_limit";
/// Read my vault report, then decide with my question. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    let url: String = ctx.params["report_url"].ok();
    let key: String = ctx.params["report_key"].ok();
    let question: String = ctx.params[QUESTION].ok();
    let amount = ctx.request.amount;
    allowit::payment_limit(ctx, &ctx.params[REPORT_LIMIT].ok())?;
    allowit::fetch(
        ctx,
        &AllowitFetch { method: "GET", url: url.as_str(), data: "", pubkey: key.as_str() },
    )?;
    let points = allowit::decision(
        ctx,
        &AllowitDecision { id: "purchase", query: question.as_str(), deny_below: 3000, allow_above: 8000 },
    )?;
    assert!(points > 0 && amount > 0);
    Ok(())
}
