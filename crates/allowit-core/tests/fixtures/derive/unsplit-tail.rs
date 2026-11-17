use allowit::prelude::*;
use allowit::AllowitDecision;
/// The last expression of `execute` is a block with a value: it leaves whole.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    {
        let score = allowit::decision(ctx, &AllowitDecision { id: "a", query: "Is this research?", deny_below: 1, allow_above: 2 })?;
        assert!(score > 0);
        assert!(ctx.request.amount > 0);
        Ok(())
    }
}
