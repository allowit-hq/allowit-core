use allowit::prelude::*;
const REPORT_LIMIT: &str = "report_limit";
/// Read my vault report, then decide with my question. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[REPORT_LIMIT].ok())?;
    Ok(())
}
