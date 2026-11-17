use allowit::prelude::*;
/// Start the decision context with my note. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    let text: String = ctx.params["catalog_query"].ok();
    let note = text.as_str();
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    allowit::new_decision_context(ctx, Some(note))?;
    Ok(())
}
