use allowit::prelude::*;
/// Keep the server's last answer. Count to a bound first.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;
    for i in 0u64.. {
        if i > 2 {
            break;
        }
    }
    Ok(())
}
