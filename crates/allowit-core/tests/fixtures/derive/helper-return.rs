use allowit::prelude::*;
use allowit::AllowitDecision;
/// Decide with my question. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    fn ask(ctx: &AllowitContext) -> allowit::Result {
        allowit::decision(
            ctx,
            &AllowitDecision { id: "a", query: "Is this research?", deny_below: 3000, allow_above: 8000 },
        )?;
        return Ok(());
    }
    ask(ctx)?;
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    Ok(())
}
