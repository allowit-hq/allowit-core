use allowit::prelude::*;
use allowit::AllowitDecision;
/// A binding may carry the name of a type. The rule follows it like any other name.
pub fn execute(ctx: &AllowitContext) -> allowit::Result {
    let AllowitValue = 0;
    {
        let AllowitValue = allowit::decision(ctx, &AllowitDecision { id: "a", query: "Is this research?", deny_below: 1, allow_above: 2 })?;
        assert!(AllowitValue > 0);
    }
    Ok(())
}
