use allowit::prelude::*;
use allowit::AllowitDecision;
/// Pay my sellers at most three times a day, and only for research.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::destinations(ctx, &ctx.params[allowit::DESTINATIONS])?;
    // The number of today's payments, in the wallet contract.
    let payments: u64 = allowit::get_state(ctx, "day_payments")?.unwrap_or(0);
    if payments >= 3 {
        return Err(AllowitError::CallLimitExceeded);
    }
    allowit::set_state(ctx, "day_payments", payments + 1)?;
    let score = allowit::decision(ctx, &AllowitDecision { id: "research", query: "Is this payment for research?", deny_below: 3000, allow_above: 8000 })?;
    assert!(score >= 3000);
    Ok(())
}
