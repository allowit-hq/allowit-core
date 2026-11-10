use allowit::prelude::*;
const EVIDENCE_SELLERS: &str = "evidence_sellers";
const REPORT_LIMIT: &str = "report_limit";
const SWAP_VENUES: &str = "swap_venues";
const SWAP_TOKEN: &str = "swap_token";
const SWAP_LIMIT: &str = "swap_limit";
const RESERVED_VOLUME: &str = "reserved_volume";
const SWAP_OUTPUT_FLOOR: &str = "swap_output_floor";
const VAULTS: &str = "vaults";
const SHARE_LIMIT: &str = "share_limit";
const REDEEM_OUTPUT_FLOOR: &str = "redeem_output_floor";
/// Buy evidence only from approved sellers. Rebalance through approved exchanges,
/// retaining capacity for later work. Redeem only approved vault positions.
/// Bound expenses, volume, assets, expiry and each operation's native effects.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    let operation = allowit::operations(ctx, &ctx.params[allowit::OPERATIONS].ok())?;
    allowit::tokens(ctx, &ctx.params[allowit::TOKENS].ok())?;
    allowit::expires_at(ctx, &ctx.params[allowit::EXPIRES_AT].ok())?;
    allowit::lifetime_spending_limit(ctx, &ctx.params[allowit::LIFETIME_SPENDING_LIMIT].ok())?;
    allowit::daily_spending_limit(ctx, &ctx.params[allowit::DAILY_SPENDING_LIMIT].ok())?;
    let remaining = allowit::daily_volume_limit(ctx, &ctx.params[allowit::DAILY_VOLUME_LIMIT].ok())?;
    match operation {
        "transfer" => {
            allowit::destinations(ctx, &ctx.params[EVIDENCE_SELLERS].ok())?;
            allowit::payment_limit(ctx, &ctx.params[REPORT_LIMIT].ok())?;
        }
        "swap" => {
            allowit::destinations(ctx, &ctx.params[SWAP_VENUES].ok())?;
            allowit::payment_limit(ctx, &ctx.params[SWAP_LIMIT].ok())?;
            let reserve: i128 = ctx.params[RESERVED_VOLUME].ok();
            let floor: i128 = ctx.params[SWAP_OUTPUT_FLOOR].ok();
            let output: Address = ctx.params[SWAP_TOKEN].ok();
            assert!(ctx.request.token_in == ctx.request.asset);
            assert!(ctx.request.token_out == Some(output));
            assert!(reserve >= 0 && floor > 0);
            assert!(remaining >= reserve);
            assert!(ctx.request.min_out >= floor);
        }
        "redeem" => {
            allowit::destinations(ctx, &ctx.params[VAULTS].ok())?;
            let shares: i128 = ctx.params[SHARE_LIMIT].ok();
            let floor: i128 = ctx.params[REDEEM_OUTPUT_FLOOR].ok();
            assert!(shares == 0 || (shares > 0 && ctx.request.amount_in <= shares));
            assert!(floor >= 0 && ctx.request.min_out >= floor);
        }
        _ => panic!("unsupported operation"),
    }
    Ok(())
}
