use allowit::prelude::*;
use allowit::AllowitDecision;
/// Pay only for research data.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::new_decision_context(ctx, None)?;
    // "Pay only for research data"
    allowit::decision(
        ctx,
        &AllowitDecision {
            id: "research",
            query: "Does this payment buy research data?",
            deny_below: 3000,
            allow_above: 8000,
        },
    )?;
    Ok(())
}
