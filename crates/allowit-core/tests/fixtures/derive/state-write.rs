use allowit::prelude::*;
use allowit::AllowitDecision;
/// Remember the last score.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::set_state(ctx, "seen", true)?;
    let score = allowit::decision(ctx, &AllowitDecision { id: "a", query: "Is this research?", deny_below: 1, allow_above: 2 })?;
    allowit::set_state(ctx, "score", score)?;
    Ok(())
}
