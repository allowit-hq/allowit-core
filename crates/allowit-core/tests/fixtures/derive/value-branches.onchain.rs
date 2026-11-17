use allowit::prelude::*;
/// Ask the question of the operation. Limit every payment.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    let operation = allowit::operations(ctx, &ctx.params[allowit::OPERATIONS])?;
    match operation {
        "transfer" => {},
        "redeem" => assert!(ctx.request.min_out > 0),
        _ => {
            allowit::tokens(ctx, &ctx.params[allowit::TOKENS])?;
        }
    }
    Ok(())
}
