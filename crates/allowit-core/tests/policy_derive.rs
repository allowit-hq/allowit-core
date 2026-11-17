//! The on-chain part of a policy source: golden files of the sample mandate and of each
//! tricky shape, and the properties that every derivation keeps.
use allowit_core::policy_source::{Cause, derive, source_shape, source_shape_with_host_calls};

/// The server functions of Core and one package function, as a server's registry.
const REGISTRY: &[(&str, &str)] = &[
    ("allowit", "price"),
    ("allowit", "fetch"),
    ("allowit", "internet_search"),
    ("allowit", "marketplace_search"),
    ("allowit", "decision"),
    ("allowit", "request_screening"),
    ("allowit", "new_decision_context"),
    ("sanctions", "screen"),
];

/// The policy library as a server gives it to the owner's source: every server function
/// answers with the values of [`served::ANSWER`]. A refused server function ends the run, as
/// it does on the server, so the source cannot catch it.
#[allow(dead_code, unused)]
mod served {
    pub use allowit_core::*;
    use std::cell::Cell;
    std::thread_local! {
        /// Whether the server functions pass, and the score or price that they give.
        pub static ANSWER: Cell<(bool, u32)> = const { Cell::new((false, 0)) };
    }
    fn answer<T>(value: impl FnOnce(u32) -> T) -> Result<T> {
        let (passes, points) = ANSWER.get();
        if !passes {
            panic!("the server refused the request");
        }
        Ok(value(points))
    }
    pub fn price(_ctx: &AllowitContext, _request: &AllowitPrice) -> Result<i128> {
        answer(i128::from)
    }
    pub fn fetch(_ctx: &AllowitContext, _request: &AllowitFetch) -> Result<AllowitValue> {
        answer(|points| AllowitValue::Int(points.into()))
    }
    pub fn internet_search(
        _ctx: &AllowitContext,
        _request: &AllowitInternetSearch,
    ) -> Result<AllowitValue> {
        answer(|points| AllowitValue::Int(points.into()))
    }
    pub fn marketplace_search(
        _ctx: &AllowitContext,
        _request: &AllowitMarketplaceSearch,
    ) -> Result<AllowitValue> {
        answer(|points| AllowitValue::Int(points.into()))
    }
    pub fn request_screening(
        _ctx: &AllowitContext,
        _request: &AllowitRequestScreening,
    ) -> Result<u32> {
        answer(|points| points)
    }
    pub fn decision(_ctx: &AllowitContext, _request: &AllowitDecision) -> Result<u32> {
        answer(|points| points)
    }
    pub fn new_decision_context(_ctx: &AllowitContext, _contents: Option<&str>) -> Result {
        answer(|_| ())
    }
}

/// Each owner source with the on-chain part that it must give, byte for byte. The modules
/// compile both: the owner's source against the served library, and its on-chain part against
/// this package alone, as the contract build does.
macro_rules! golden {
    ($($module:ident => $name:literal),* $(,)?) => {
        const GOLDEN: &[(&str, &str, &str)] = &[$((
            $name,
            include_str!(concat!("fixtures/derive/", $name, ".rs")),
            include_str!(concat!("fixtures/derive/", $name, ".onchain.rs")),
        )),*];
        $(
            // Authored policy source keeps its own style.
            #[allow(dead_code, unused, clippy::all)]
            mod $module {
                pub mod owner {
                    use crate::served as allowit;
                    include!(concat!("fixtures/derive/", $name, ".rs"));
                }
                pub mod onchain {
                    use allowit_core as allowit;
                    include!(concat!("fixtures/derive/", $name, ".onchain.rs"));
                }
                pub const BOTH: [fn(&allowit_core::AllowitContext) -> allowit_core::Result; 2] =
                    [owner::execute, onchain::execute];
            }
        )*
    };
}
golden!(
    sample_mandate => "sample-mandate",
    no_server_function => "no-server-function",
    only_server_functions => "only-server-functions",
    rule_in_server_branch => "rule-in-server-branch",
    early_return_on_server_value => "early-return-on-server-value",
    deny_on_server_value => "deny-on-server-value",
    break_on_server_value => "break-on-server-value",
    owner_text_params => "owner-text-params",
    local_functions => "local-functions",
    mutable_name => "mutable-name",
    value_branches => "value-branches",
    server_tail => "server-tail",
    mutable_alias => "mutable-alias",
    labeled_block => "labeled-block",
    text_temporary => "text-temporary",
    return_before_item => "return-before-item",
);
/// A package function has no host library here, so this shape does not compile in the test.
const PACKAGE: (&str, &str) = (
    include_str!("fixtures/derive/package-function.rs"),
    include_str!("fixtures/derive/package-function.onchain.rs"),
);
const STATE_WRITE: &str = include_str!("fixtures/derive/state-write.rs");

fn cases() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut cases = GOLDEN.to_vec();
    cases.push(("package-function", PACKAGE.0, PACKAGE.1));
    cases
}

#[test]
fn each_source_gives_its_golden_on_chain_part_byte_for_byte() {
    for (name, source, expected) in cases() {
        source_shape_with_host_calls(source, REGISTRY)
            .unwrap_or_else(|error| panic!("{name} is no server policy source: {error}"));
        let derived = derive(source).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(derived.source, expected, "{name}");
        // The same source gives the same derived policy.
        assert_eq!(derive(source).unwrap(), derived, "{name}");
    }
}

/// The derived policy is portable source that names no server function, and it is its own
/// on-chain part.
#[test]
fn a_derived_policy_is_portable_and_derives_to_itself() {
    for (name, _, expected) in cases() {
        source_shape(expected).unwrap_or_else(|error| panic!("{name}: {error}"));
        source_shape_with_host_calls(expected, &[])
            .unwrap_or_else(|error| panic!("{name} names a server function: {error}"));
        let again = derive(expected).unwrap();
        assert_eq!(again.source, expected, "{name}");
        assert!(
            again.left.is_empty() && again.server_only.is_empty(),
            "{name}"
        );
    }
}

#[test]
fn a_source_without_a_server_function_derives_to_itself() {
    for source in [
        include_str!("fixtures/portable-policy.rs"),
        include_str!("fixtures/treasury-policy.rs"),
        include_str!("fixtures/derive/no-server-function.rs"),
    ] {
        let derived = derive(source).unwrap();
        assert_eq!(derived.source, source);
        assert!(derived.left.is_empty() && derived.server_only.is_empty());
        assert!(!derived.kept.is_empty());
    }
}

/// A source that is only server calls derives to a policy that checks nothing.
#[test]
fn a_source_of_server_calls_only_keeps_no_check() {
    let derived = derive(include_str!("fixtures/derive/only-server-functions.rs")).unwrap();
    assert!(derived.kept.is_empty());
    assert!(derived.server_only.is_empty());
    assert!(derived.source.contains("{\n    Ok(())\n}"));
}

#[test]
fn the_sample_mandate_keeps_every_limit_and_reports_what_left() {
    let source = include_str!("fixtures/derive/sample-mandate.rs");
    let derived = derive(source).unwrap();
    assert_eq!(
        derived.kept,
        [
            "operations",
            "tokens",
            "expires_at",
            "lifetime_spending_limit",
            "daily_spending_limit",
            "daily_volume_limit",
            "destinations",
            "payment_limit",
            "assert",
            "deny"
        ]
    );
    // No deterministic check of the sample runs only on the server.
    assert!(derived.server_only.is_empty());
    assert_eq!(derived.left, [(2, 2), (55, 91), (95, 103)]);
    for gone in [
        "as_str",
        "decision",
        "fetch",
        "internet_search",
        "marketplace_search",
        "web_query",
    ] {
        assert!(
            source.contains(gone) && !derived.source.contains(gone),
            "{gone}"
        );
    }
}

/// A rule in a branch that depends on a server function leaves, and the report says that the
/// owner can move it. A plain denial in such a branch is the verdict of that branch.
#[test]
fn a_rule_in_a_server_branch_leaves_and_is_reported_as_movable() {
    let derived = derive(include_str!("fixtures/derive/rule-in-server-branch.rs")).unwrap();
    let reported: Vec<(u32, &str, Cause, bool)> = derived
        .server_only
        .iter()
        .map(|check| {
            (
                check.line,
                check.check.as_str(),
                check.cause,
                check.movable(),
            )
        })
        .collect();
    assert_eq!(
        reported,
        [
            (12, "payment_limit", Cause::ServerBranch, true),
            (13, "assert", Cause::ServerBranch, true)
        ]
    );
    assert_eq!(derived.kept, ["payment_limit", "destinations"]);

    let denial = derive(include_str!("fixtures/derive/deny-on-server-value.rs")).unwrap();
    let [check] = &denial.server_only[..] else {
        panic!("one check leaves");
    };
    assert_eq!((check.check.as_str(), check.movable()), ("deny", false));
    // A denial does not end `execute` with an approval, so the later rule stays.
    assert_eq!(denial.kept, ["destinations"]);
}

/// After an early approval that depends on a server function, the whole source can allow a
/// request without the later rules. The chain must not apply them.
#[test]
fn the_rules_after_an_early_approval_on_a_server_value_leave() {
    let derived = derive(include_str!(
        "fixtures/derive/early-return-on-server-value.rs"
    ))
    .unwrap();
    assert_eq!(derived.kept, ["payment_limit"]);
    let causes: Vec<(&str, Cause)> = derived
        .server_only
        .iter()
        .map(|check| (check.check.as_str(), check.cause))
        .collect();
    assert_eq!(
        causes,
        [
            ("destinations", Cause::AfterServerExit),
            ("tokens", Cause::AfterServerExit)
        ]
    );
    // `return Err(...).or(Ok(()))` approves, so it is no denial: the later rule leaves.
    let denial = include_str!("fixtures/derive/deny-on-server-value.rs");
    let approval = denial.replace(
        "return Err(AllowitError::DestinationNotAllowed);",
        "return Err(AllowitError::DestinationNotAllowed).or(Ok(()));",
    );
    assert_ne!(approval, denial);
    assert_eq!(derive(denial).unwrap().kept, ["destinations"]);
    assert!(derive(&approval).unwrap().kept.is_empty());
    // A loop that a server value can end leaves whole, with what it assigns.
    let looped = derive(include_str!("fixtures/derive/break-on-server-value.rs")).unwrap();
    assert_eq!(looped.kept, ["payment_limit", "tokens"]);
    assert!(!looped.source.contains("for ") && !looped.source.contains("checked"));
}

/// A mutable reference lets a leaving statement assign a name that it does not mention. The
/// check that reads that name leaves too, and a state write of it is refused.
#[test]
fn a_name_that_a_leaving_statement_assigns_through_a_reference_leaves() {
    let source = include_str!("fixtures/derive/mutable-alias.rs");
    let derived = derive(source).unwrap();
    assert!(!derived.source.contains("total") && !derived.source.contains("alias"));
    assert_eq!(derived.kept, ["payment_limit", "tokens"]);
    let [check] = &derived.server_only[..] else {
        panic!("one check leaves");
    };
    assert_eq!(
        (check.check.as_str(), check.cause),
        ("assert", Cause::ServerValue)
    );
    let written = source.replace(
        "    assert!(total == 1);\n",
        "    allowit::set_state(ctx, \"total\", total)?;\n",
    );
    assert!(
        derive(&written)
            .unwrap_err()
            .contains("cannot write policy state")
    );
}

/// A `break` out of a labeled block skips the rest of the block. When the `break` depends on a
/// server function, the block leaves whole.
#[test]
fn a_labeled_block_that_a_server_value_can_end_leaves_whole() {
    let derived = derive(include_str!("fixtures/derive/labeled-block.rs")).unwrap();
    assert_eq!(derived.kept, ["payment_limit", "tokens"]);
    assert!(!derived.source.contains("'checks"));
    let [check] = &derived.server_only[..] else {
        panic!("one check leaves");
    };
    assert_eq!(
        (check.check.as_str(), check.cause, check.movable()),
        ("destinations", Cause::AfterServerExit, true)
    );
}

/// Owner text that a statement reads with `as_str()` is server vocabulary: the statement
/// leaves with the server call that takes the text. When the last step of `execute` leaves
/// and items follow it, the derived `execute` approves with a `return`.
#[test]
fn server_text_leaves_and_an_erased_last_step_approves_before_its_items() {
    let text = derive(include_str!("fixtures/derive/text-temporary.rs")).unwrap();
    assert_eq!(text.kept, ["payment_limit"]);
    assert!(!text.source.contains("as_str") && !text.source.contains("catalog_query"));

    let item = derive(include_str!("fixtures/derive/return-before-item.rs")).unwrap();
    assert!(
        item.source
            .contains("    limit(ctx)?;\n    return Ok(());\n    fn limit(")
    );
    assert_eq!(item.kept, ["payment_limit"]);
}

#[test]
fn a_state_write_that_leaves_is_refused() {
    let error = derive(STATE_WRITE).unwrap_err();
    assert!(
        error.contains("cannot write policy state") && error.contains("line 7"),
        "{error}"
    );
    // The same write before the server call stays, and the source derives.
    let moved = STATE_WRITE.replace("    allowit::set_state(ctx, \"score\", score)?;\n", "");
    let derived = derive(&moved).unwrap();
    assert!(
        derived
            .source
            .contains("allowit::set_state(ctx, \"seen\", true)?;")
    );
    // A local function that writes state, called from a leaving statement, is refused too.
    let helper = STATE_WRITE
        .replace(
            "pub fn execute",
            "fn remember(ctx: &AllowitContext, score: u32) -> allowit::Result {\n    allowit::set_state(ctx, \"score\", score)\n}\npub fn execute",
        )
        .replace("allowit::set_state(ctx, \"score\", score)?;", "remember(ctx, score)?;");
    assert!(
        derive(&helper)
            .unwrap_err()
            .contains("cannot write policy state")
    );
}

/// An item that stands after an early approval is no step of the evaluation: a rule before
/// the approval still calls it. A local `Ok` or `Err` would change what the derivation reads
/// as an approval or a denial, so such a source is refused.
#[test]
fn an_item_after_an_early_approval_stays_and_a_local_ok_is_refused() {
    let source = include_str!("fixtures/derive/early-return-on-server-value.rs")
        .replace(
            "    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n",
            "    limit(ctx)?;\n",
        )
        .replace(
            "    Ok(())\n}",
            "    fn limit(ctx: &AllowitContext) -> allowit::Result {\n        allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n        Ok(())\n    }\n    Ok(())\n}",
        );
    let derived = derive(&source).unwrap();
    assert!(derived.source.contains("limit(ctx)?;") && derived.source.contains("fn limit("));
    assert_eq!(derived.kept, ["payment_limit"]);

    let shadowed = source.replace(
        "pub fn execute",
        "fn Ok(_: ()) -> allowit::Result {\n    Err(AllowitError::Expired)\n}\npub fn execute",
    );
    assert!(
        derive(&shadowed)
            .unwrap_err()
            .contains("local `Ok` or `Err`")
    );
    // Without a server function nothing is derived, so the source stays as it is.
    let plain = include_str!("fixtures/derive/no-server-function.rs").replace(
        "pub fn execute",
        "fn Ok(_: ()) -> allowit::Result {\n    Err(AllowitError::Expired)\n}\npub fn execute",
    );
    assert_eq!(derive(&plain).unwrap().source, plain);
}

#[test]
fn a_source_that_is_no_policy_is_refused() {
    assert!(
        derive("use allowit::prelude::*;\n")
            .unwrap_err()
            .contains("pub fn execute")
    );
    assert!(derive("pub fn execute(ctx: &AllowitContext) -> allowit::Result { \"open").is_err());
    assert!(derive("/* open").is_err());
}

/// Every source of the gate's vector file that the portable profile admits derives, or is
/// refused for a state write, and the result is its own on-chain part.
#[test]
fn every_admitted_vector_derives_to_portable_source() {
    let canonical = include_str!("fixtures/portable-policy.rs");
    let vectors: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/policy-source-vectors.json")).unwrap();
    let (mut derived_count, mut changed) = (0, 0);
    for vector in &vectors {
        let head = vector["head"].as_u64().unwrap() as usize;
        let tail = vector["tail"].as_u64().unwrap() as usize;
        let source = format!(
            "{}{}{}",
            &canonical[..head],
            vector["middle"].as_str().unwrap(),
            &canonical[canonical.len() - tail..]
        );
        if source_shape(&source).is_err() {
            continue;
        }
        match derive(&source) {
            Ok(derived) => {
                derived_count += 1;
                changed += usize::from(derived.source != source);
                source_shape(&derived.source).unwrap();
                assert_eq!(
                    derive(&derived.source).unwrap().source,
                    derived.source,
                    "{source}"
                );
            }
            Err(error) => assert!(
                error.contains("cannot write policy state"),
                "{error}\n{source}"
            ),
        }
    }
    assert!(derived_count > 100, "{derived_count}");
    assert!(changed > 0);
}

/// The property that the derivation keeps: the on-chain part never denies a request that the
/// whole source allows, and it then stores the same policy state. Every golden source runs
/// with its on-chain part over the same requests, params and server answers.
#[test]
fn the_on_chain_part_allows_what_the_whole_source_allows_with_the_same_state() {
    use allowit_core::{
        Address, AllowitArgs, AllowitContext, AllowitEnv, AllowitMap, AllowitOperation,
        AllowitParams, AllowitRequest, AllowitTier, AllowitValue, evaluate,
    };
    let address = |text: &str| Address::new(text);
    let addresses = |items: &[&str]| {
        AllowitValue::List(
            items
                .iter()
                .map(|item| AllowitValue::Addr(address(item)))
                .collect(),
        )
    };
    let text = |value: &str| AllowitValue::Str(value.into());
    let params = |limit: i128| -> AllowitMap {
        let mut map = AllowitMap::new();
        let mut set = |key: &str, value: AllowitValue| {
            map.insert(key.into(), value);
        };
        for key in [
            "payment_limit",
            "lifetime_spending_limit",
            "daily_spending_limit",
            "daily_volume_limit",
            "report_limit",
            "swap_limit",
            "daily_limit",
            "cap",
            "share_limit",
        ] {
            set(key, AllowitValue::Int(limit));
        }
        set("call_limit", AllowitValue::Int(100));
        set("expires_at", AllowitValue::Int(2_000_000_000));
        set("operations", text("*"));
        for key in [
            "destinations",
            "evidence_sellers",
            "swap_venues",
            "vaults",
            "vendors",
        ] {
            set(key, addresses(&["SELLER", "VENUE"]));
        }
        set("tokens", addresses(&["ASSET"]));
        set("swap_token", AllowitValue::Addr(address("OTHER")));
        set("quote", AllowitValue::Addr(address("OTHER")));
        for key in [
            "reserved_volume",
            "swap_output_floor",
            "redeem_output_floor",
        ] {
            set(key, AllowitValue::Int(1));
        }
        for key in [
            "catalog_query",
            "report_url",
            "report_key",
            "web_query",
            "report_purchase",
            "position_review",
            "purchase_question",
        ] {
            set(key, text("owner text"));
        }
        set("deny_below", AllowitValue::Int(3_000));
        set("allow_above", AllowitValue::Int(8_000));
        map
    };
    let operations = [
        AllowitOperation::transfer,
        AllowitOperation::swap,
        AllowitOperation::redeem,
        AllowitOperation::deposit,
    ];
    let (mut allowed, mut denied_on_chain, mut runs) = (0, 0, 0);
    for (name, both) in [
        ("sample-mandate", sample_mandate::BOTH),
        ("no-server-function", no_server_function::BOTH),
        ("only-server-functions", only_server_functions::BOTH),
        ("rule-in-server-branch", rule_in_server_branch::BOTH),
        (
            "early-return-on-server-value",
            early_return_on_server_value::BOTH,
        ),
        ("deny-on-server-value", deny_on_server_value::BOTH),
        ("break-on-server-value", break_on_server_value::BOTH),
        ("owner-text-params", owner_text_params::BOTH),
        ("local-functions", local_functions::BOTH),
        ("mutable-name", mutable_name::BOTH),
        ("value-branches", value_branches::BOTH),
        ("server-tail", server_tail::BOTH),
        ("mutable-alias", mutable_alias::BOTH),
        ("labeled-block", labeled_block::BOTH),
        ("text-temporary", text_temporary::BOTH),
        ("return-before-item", return_before_item::BOTH),
    ] {
        for operation in &operations {
            for amount in [1i128, 400, 6_000_000] {
                for recipient in ["SELLER", "STRANGER"] {
                    for limit in [500i128, 10_000_000] {
                        for answer in [(false, 0), (true, 0), (true, 4_000), (true, 9_500)] {
                            let context = || AllowitContext {
                                env: AllowitEnv::new(),
                                params: AllowitParams::from(params(limit)),
                                state: AllowitMap::new(),
                                request: AllowitRequest {
                                    operation: operation.clone(),
                                    asset: address("ASSET"),
                                    recipient: address(recipient),
                                    amount,
                                    token_in: address("ASSET"),
                                    amount_in: amount,
                                    token_out: (*operation != AllowitOperation::transfer)
                                        .then(|| address("OTHER")),
                                    min_out: 5,
                                    now: 1_800_000_000,
                                    expires_at: 1_800_000_600,
                                    spent_units: 1,
                                    gas_units: 1,
                                    unpriced: false,
                                    args: AllowitArgs::from(AllowitMap::new()),
                                },
                            };
                            served::ANSWER.set(answer);
                            let whole = evaluate(&context(), both[0], AllowitTier::Custom);
                            // The contract has no server: its policy must not need one.
                            served::ANSWER.set((false, 0));
                            let on_chain = evaluate(&context(), both[1], AllowitTier::Custom);
                            runs += 1;
                            denied_on_chain += usize::from(on_chain.is_err());
                            if let Ok(state) = &whole {
                                allowed += 1;
                                assert_eq!(
                                    on_chain.as_ref(),
                                    Ok(state),
                                    "{name}: {operation:?} of {amount} to {recipient}, limit {limit}, server {answer:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    // The runs cover allowed requests of every kind and requests that the chain denies.
    assert!(
        allowed > 300 && denied_on_chain > 300,
        "{allowed} allowed, {denied_on_chain} denied of {runs}"
    );
}
