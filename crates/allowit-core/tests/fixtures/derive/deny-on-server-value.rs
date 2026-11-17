use allowit::prelude::*;
use allowit::AllowitRequestScreening;
/// Deny a seller that screens badly. Pay only vendors I approve.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    let score = allowit::request_screening(
        ctx,
        &AllowitRequestScreening { query: "Is the seller listed?", deny_below: 2000 },
    )?;
    if score < 5000 {
        return Err(AllowitError::DestinationNotAllowed);
    }
    allowit::destinations(ctx, &ctx.params[allowit::DESTINATIONS])?;
    Ok(())
}
