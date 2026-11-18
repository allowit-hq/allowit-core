//! The on-chain part of a policy source: golden files of the sample mandate and of each
//! tricky shape, and the properties that every derivation keeps.
use allowit_core::policy_source::{
    Cause, Keeper, derive, source_shape, source_shape_with_host_calls,
};

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
            #[allow(dead_code, unused, non_snake_case, clippy::all)]
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
    labeled_block => "labeled-block",
    text_temporary => "text-temporary",
    return_before_item => "return-before-item",
    helper_return => "helper-return",
    late_value => "late-value",
    attributed_import => "attributed-import",
    stored_state => "stored-state",
    type_named_binding => "type-named-binding",
    unsplit_tail => "unsplit-tail",
    server_range => "server-range",
    block_condition => "block-condition",
    state_on_chain => "state-on-chain",
    state_on_server => "state-on-server",
    state_helpers => "state-helpers",
);
/// A package function has no host library here, so this shape does not compile in the test.
const PACKAGE: (&str, &str) = (
    include_str!("fixtures/derive/package-function.rs"),
    include_str!("fixtures/derive/package-function.onchain.rs"),
);

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
        // The contract keeps every state key of its own part.
        assert!(
            again
                .state
                .iter()
                .all(|entry| entry.keeper == Keeper::Chain),
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

/// A mutable reference lets a statement assign a name that it does not mention. The rule
/// follows names, so it derives no part of a policy with a server part that takes one: not
/// through a named alias, and not through a reference to a value without a name.
#[test]
fn a_policy_with_a_server_part_that_takes_a_mutable_reference_is_refused() {
    let alias = include_str!("fixtures/derive/mutable-alias.rs");
    let unnamed = alias
        .replace("    let mut total: i128 = 0;\n", "")
        .replace("let alias = &mut total;", "let total = &mut 0i128;")
        .replace("*alias = {", "*total = {")
        .replace("assert!(total == 1);", "assert!(*total == 1);");
    assert!(unnamed.contains("let total = &mut 0i128;") && unnamed.contains("*total == 1"));
    let pattern = alias.replace("let alias = &mut total;", "let ref mut alias = total;");
    for source in [alias, unnamed.as_str(), pattern.as_str()] {
        let error = derive(source).unwrap_err();
        assert!(error.contains("mutable reference"), "{error}");
    }
    // The same policy without its server function has no server part: it is its own part.
    let plain = alias.replace("        allowit::new_decision_context(ctx, None)?;\n", "");
    assert_eq!(derive(&plain).unwrap().source, plain);
}

/// Two local functions with one name: the rule cannot tell which one a statement calls, so
/// it derives no part of such a policy with a server part.
#[test]
fn two_local_items_with_one_name_are_refused() {
    let source = "use allowit::prelude::*;\nfn limit(ctx: &AllowitContext) -> allowit::Result {\n    allowit::new_decision_context(ctx, None)\n}\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    fn limit(ctx: &AllowitContext) -> allowit::Result {\n        assert!(ctx.request.amount < 100);\n        Ok(())\n    }\n    limit(ctx)?;\n    Ok(())\n}\n";
    let error = derive(source).unwrap_err();
    assert!(error.contains("declare `limit` twice"), "{error}");
}

/// An import in a block keeps the names that the kept code reads. A check in a branch that
/// left is movable only when it reads no server value itself.
#[test]
fn a_block_import_keeps_its_portable_names_and_a_check_reports_its_own_cause() {
    let source = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    use allowit::{AllowitDecision, AllowitOperation};\n    let score = allowit::decision(ctx, &AllowitDecision { id: \"a\", query: \"Is this research?\", deny_below: 1, allow_above: 2 })?;\n    if score < 9000 {\n        let doubled = score + score;\n        assert!(doubled > 5);\n        assert!(ctx.request.amount > 0);\n    }\n    assert!(ctx.request.operation == AllowitOperation::transfer);\n    Ok(())\n}\n";
    let derived = derive(source).unwrap();
    assert!(
        derived
            .source
            .contains("    use allowit::AllowitOperation;\n")
    );
    assert!(derived.source.contains("AllowitOperation::transfer"));
    let reported: Vec<(u32, Cause, bool)> = derived
        .server_only
        .iter()
        .map(|check| (check.line, check.cause, check.movable()))
        .collect();
    assert_eq!(
        reported,
        [
            (7, Cause::ServerValue, false),
            (8, Cause::ServerBranch, true)
        ]
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

/// A `return` in a local function ends that function, not `execute`: when the function
/// leaves, the rules after its call stay. A name that gets its value in a leaving statement
/// leaves with every statement that reads it. An import behind an attribute leaves like any
/// other.
#[test]
fn a_helper_return_a_late_value_and_an_attributed_import_derive() {
    let helper = derive(include_str!("fixtures/derive/helper-return.rs")).unwrap();
    assert_eq!(helper.kept, ["payment_limit"]);
    assert!(!helper.source.contains("fn ask") && !helper.source.contains("ask(ctx)"));

    let late = derive(include_str!("fixtures/derive/late-value.rs")).unwrap();
    assert_eq!(late.kept, ["payment_limit"]);
    assert!(!late.source.contains("accepted"));
    let [check] = &late.server_only[..] else {
        panic!("one check leaves");
    };
    assert_eq!(
        (check.check.as_str(), check.cause),
        ("assert", Cause::ServerValue)
    );

    let import = derive(include_str!("fixtures/derive/attributed-import.rs")).unwrap();
    assert!(!import.source.contains("AllowitRequestScreening"));
    assert!(!import.source.contains("#[allow"));
    assert_eq!(import.kept, ["payment_limit"]);
}

/// A condition that is itself a block: the block after it is the body. The derivation does
/// not split such a statement, so a `break` that depends on a server function takes the
/// whole loop, with the check after it.
#[test]
fn a_statement_whose_condition_is_a_block_is_not_split() {
    let looped = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n    while { true } {\n        if allowit::new_decision_context(ctx, None).is_ok() {\n            break;\n        }\n        assert!(false);\n    }\n    Ok(())\n}\n";
    let derived = derive(looped).unwrap();
    assert_eq!(derived.kept, ["payment_limit"]);
    assert!(!derived.source.contains("while") && !derived.source.contains("assert!"));
    for (shape, body) in [
        (
            "if",
            "    if { ctx.request.amount > 0 } {\n        allowit::new_decision_context(ctx, None)?;\n        return Ok(());\n    }\n    assert!(ctx.request.amount > 5);\n",
        ),
        (
            "match",
            "    match { ctx.request.amount } {\n        _ => {\n            allowit::new_decision_context(ctx, None)?;\n            return Ok(());\n        }\n    }\n    assert!(ctx.request.amount > 5);\n",
        ),
    ] {
        let source = format!(
            "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {{\n    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n{body}    Ok(())\n}}\n"
        );
        let derived = derive(&source).unwrap();
        assert_eq!(derived.kept, ["payment_limit"], "{shape}");
        assert!(
            !derived.source.contains("assert!"),
            "{shape}: {}",
            derived.source
        );
    }
}

/// A local `Err` that approves, declared inside an expression, would make a `return Err(...)`
/// an approval. The derivation reads that form as a denial, so it refuses the source.
#[test]
fn a_local_err_inside_an_expression_is_refused() {
    let source = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    let _started = {\n        fn Err(_: AllowitError) -> allowit::Result {\n            Ok(())\n        }\n        allowit::new_decision_context(ctx, None)?;\n        return Err(AllowitError::Expired);\n    };\n    assert!(ctx.request.amount < 0);\n    Ok(())\n}\n";
    assert!(derive(source).unwrap_err().contains("local `Ok` or `Err`"));
}

/// A binding with the name of a type is a binding: a statement that reads it after a server
/// value bound it leaves. A check in a statement that leaves whole names what it reads itself:
/// one that reads a server value cannot move, and one that does not can.
#[test]
fn a_type_named_binding_is_followed_and_an_unsplit_check_names_what_it_reads() {
    let named = derive(include_str!("fixtures/derive/type-named-binding.rs")).unwrap();
    assert!(named.kept.is_empty() && !named.source.contains("assert"));
    let [check] = &named.server_only[..] else {
        panic!("one check leaves");
    };
    assert_eq!((check.cause, check.movable()), (Cause::ServerValue, false));

    let tail = derive(include_str!("fixtures/derive/unsplit-tail.rs")).unwrap();
    let reported: Vec<(u32, Cause, bool)> = tail
        .server_only
        .iter()
        .map(|check| (check.line, check.cause, check.movable()))
        .collect();
    assert_eq!(
        reported,
        [(8, Cause::ServerValue, false), (9, Cause::NotSplit, true)]
    );
}

/// A name after a range operator is a read of that name, not a field. A statement whose
/// condition holds a block ends with its own last block, so the rules after it stay.
#[test]
fn a_range_bound_is_a_read_and_a_block_condition_takes_no_later_rule() {
    let range = derive(include_str!("fixtures/derive/server-range.rs")).unwrap();
    assert_eq!(range.kept, ["payment_limit"]);
    assert!(!range.source.contains("count"));
    let causes: Vec<Cause> = range.server_only.iter().map(|check| check.cause).collect();
    assert_eq!(causes, [Cause::ServerBranch, Cause::ServerBranch]);

    let block = derive(include_str!("fixtures/derive/block-condition.rs")).unwrap();
    assert_eq!(block.kept, ["payment_limit", "destinations"]);
    let [check] = &block.server_only[..] else {
        panic!("one check leaves");
    };
    assert_eq!((check.line, check.cause), (13, Cause::NotSplit));
}

/// The keys of a derived policy with their keepers and whether they live one daily window.
fn keys(source: &str) -> Vec<(String, Keeper, bool)> {
    derive(source)
        .unwrap()
        .state
        .into_iter()
        .map(|entry| (entry.key, entry.keeper, entry.daily))
        .collect()
}

/// A policy may keep its own state. A state statement that depends on no server function
/// stays: the contract keeps its key, and the server reads the key from the contract. A source
/// with no server part is its own on-chain part, with its state.
#[test]
fn a_state_statement_that_needs_no_server_stays_and_the_chain_keeps_its_key() {
    let source = include_str!("fixtures/derive/state-on-chain.rs");
    let derived = derive(source).unwrap();
    assert!(
        derived
            .source
            .contains("allowit::get_state(ctx, \"day_payments\")")
    );
    assert!(
        derived
            .source
            .contains("allowit::set_state(ctx, \"day_payments\", payments + 1)?;")
    );
    assert_eq!(derived.kept, ["destinations", "deny"]);
    assert_eq!(
        keys(source),
        [("day_payments".to_owned(), Keeper::Chain, true)]
    );
    assert_eq!(Keeper::Chain.code(), "chain");
    // The counter is the policy's own figure: the wallet keeps no standard figure for it.
    assert!(derived.figures.is_empty());

    let plain = include_str!("fixtures/derive/no-server-function.rs").replace(
        "    Ok(())\n}",
        "    allowit::set_state(ctx, \"last\", ctx.request.amount)?;\n    Ok(())\n}",
    );
    let derived = derive(&plain).unwrap();
    assert_eq!(derived.source, plain);
    assert_eq!(keys(&plain), [("last".to_owned(), Keeper::Chain, false)]);
}

/// A write that depends on a server function leaves, so the server keeps its key. Then every
/// statement that reads or writes that key leaves too, also one that depends on no server
/// function: the contract never stores the key. A write before the server call stays.
#[test]
fn a_key_whose_write_leaves_is_the_servers_with_every_statement_that_uses_it() {
    let source = include_str!("fixtures/derive/state-on-server.rs");
    let derived = derive(source).unwrap();
    assert_eq!(
        keys(source),
        [
            ("last_score".to_owned(), Keeper::Server, false),
            ("seen".to_owned(), Keeper::Chain, false)
        ]
    );
    assert_eq!(Keeper::Server.code(), "server");
    assert!(!derived.source.contains("last_score"));
    assert_eq!(derived.kept, ["payment_limit"]);
    // The check of the stored score runs only on the server, and the owner cannot move it.
    let [check] = &derived.server_only[..] else {
        panic!("one check leaves");
    };
    assert_eq!(
        (
            check.line,
            check.check.as_str(),
            check.cause,
            check.movable()
        ),
        (8, "assert", Cause::StoredState, false)
    );
    assert_eq!(Cause::StoredState.code(), "stored_state");

    // A write in a branch that depends on a server function leaves like any statement there.
    let branched = source.replace(
        "    allowit::set_state(ctx, \"last_score\", score)?;\n",
        "    if score > 5 {\n        allowit::set_state(ctx, \"last_score\", 1u32)?;\n    }\n",
    );
    assert_ne!(branched, source);
    assert_eq!(keys(&branched), keys(source));
    // A write after an early approval that depends on a server function leaves too, with the
    // key of the write before it: one key has one keeper.
    let early = source
        .replace(
            "    allowit::set_state(ctx, \"last_score\", score)?;\n",
            "    if score > 5 {\n        return Ok(());\n    }\n    allowit::set_state(ctx, \"seen\", false)?;\n",
        )
        .replace(
            "    let last: Option<u32> = allowit::get_state(ctx, \"last_score\")?;\n    assert!(last != Some(0));\n",
            "",
        );
    let derived = derive(&early).unwrap();
    assert_eq!(keys(&early), [("seen".to_owned(), Keeper::Server, false)]);
    assert!(!derived.source.contains("set_state"));
    assert_eq!(derived.kept, ["payment_limit"]);
    // A key that only leaving statements read is the server's.
    let read = source.replace(
        "    allowit::set_state(ctx, \"last_score\", score)?;\n",
        "    let floor: u32 = allowit::get_state(ctx, \"floor\")?.unwrap_or(score);\n    assert!(floor > 0);\n",
    );
    assert!(keys(&read).contains(&("floor".to_owned(), Keeper::Server, false)));
}

/// A call of a local function that writes a key is a write of that key. When such a call
/// leaves, the server keeps the key: the function leaves, and so does every other call of it.
/// A function whose calls all stay keeps its key in the contract.
#[test]
fn a_local_function_that_writes_state_follows_its_calls() {
    let source = include_str!("fixtures/derive/state-helpers.rs");
    let derived = derive(source).unwrap();
    assert_eq!(
        keys(source),
        [
            ("flagged".to_owned(), Keeper::Server, false),
            ("paid".to_owned(), Keeper::Chain, false),
            ("passed".to_owned(), Keeper::Server, false)
        ]
    );
    assert!(derived.source.contains("fn count(") && derived.source.contains("    count(ctx)?;"));
    for gone in [
        "fn pass(",
        "fn flag(",
        "pass(ctx)",
        "\"flagged\"",
        "\"passed\"",
    ] {
        assert!(!derived.source.contains(gone), "{gone}");
    }
    assert_eq!(derived.kept, ["payment_limit", "destinations"]);
    // The denial on the stored flag is the verdict of its branch.
    let [check] = &derived.server_only[..] else {
        panic!("one check leaves");
    };
    assert_eq!(
        (check.line, check.check.as_str(), check.movable()),
        (21, "deny", false)
    );
}

/// The contract must know each key before it runs. A key that is no string literal, a key
/// that is no valid name, a write of a standard figure and a state function that is not
/// called are refused, with the line.
#[test]
fn a_state_call_that_the_contract_cannot_declare_is_refused() {
    let with = |statements: &str| {
        include_str!("fixtures/derive/no-server-function.rs")
            .replace("    Ok(())\n}", &format!("{statements}    Ok(())\n}}"))
    };
    let lines = with("").lines().count();
    for (statements, reason) in [
        (
            "    let key = \"seen\";\n    allowit::set_state(ctx, key, true)?;\n",
            "names each state key as a string literal",
        ),
        (
            "    let seen: Option<bool> = allowit::get_state(ctx, allowit::CALLS)?;\n",
            "names each state key as a string literal",
        ),
        (
            "    allowit::set_state(ctx, \"last seen\", true)?;\n",
            "is not 1 to 32 ASCII letters, digits or `_`",
        ),
        (
            "    allowit::set_state(ctx, \"a_key_that_is_longer_than_32_bytes\", true)?;\n",
            "is not 1 to 32 ASCII letters, digits or `_`",
        ),
        (
            "    allowit::set_state(ctx, \"\", true)?;\n",
            "is not 1 to 32 ASCII letters, digits or `_`",
        ),
        (
            "    allowit::set_state(ctx, \"day_spent\", 0u32)?;\n",
            "cannot write the standard figure \"day_spent\"",
        ),
    ] {
        let source = with(statements);
        assert_ne!(source, with(""));
        let error = derive(&source).unwrap_err();
        assert!(error.contains(reason), "{error}");
        let line = lines - 1 + statements.lines().count() - 1;
        assert!(error.contains(&format!("line {line}")), "{error}");
    }
    // A call in a comment or in a string is no call.
    let quiet = with(
        "    // allowit::set_state(ctx, key, true)?;\n    assert!(\"allowit::set_state(ctx, key)\" != \"\");\n",
    );
    assert!(derive(&quiet).unwrap().state.is_empty());
    // An explicit type on the read.
    let typed = with("    let n = allowit::get_state::<u64>(ctx, \"count\")?;\n");
    assert_eq!(keys(&typed), [("count".to_owned(), Keeper::Chain, false)]);
    // A raw string names a key as a plain string does.
    let raw = with(
        "    let seen: Option<bool> = allowit::get_state(ctx, r\"seen\")?;\n    allowit::set_state(ctx, r#\"seen\"#, true)?;\n",
    );
    assert_eq!(keys(&raw), [("seen".to_owned(), Keeper::Chain, false)]);
    // A local function may carry the name of a state function: it is no state call.
    let local = with("    assert!(get_state() == 1);\n").replace(
        "pub fn execute",
        "fn get_state() -> u64 {\n    1\n}\npub fn execute",
    );
    source_shape(&local).unwrap();
    let derived = derive(&local).unwrap();
    assert_eq!(derived.source, local);
    assert!(derived.state.is_empty());
}

/// A function inside a function that only the server runs can carry the name of a function
/// that writes state. The rule follows names, so it cannot tell which one a statement calls:
/// it refuses the source. With another name for the inner function, the key stays with the
/// contract.
#[test]
fn a_shadowed_writer_is_refused() {
    let source = "use allowit::prelude::*;\nfn remember(ctx: &AllowitContext) -> allowit::Result {\n    allowit::set_state(ctx, \"seen\", true)\n}\nfn ask(ctx: &AllowitContext) -> allowit::Result {\n    fn remember(_ctx: &AllowitContext) -> allowit::Result {\n        Ok(())\n    }\n    remember(ctx)?;\n    allowit::new_decision_context(ctx, None)\n}\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    remember(ctx)?;\n    let seen: Option<bool> = allowit::get_state(ctx, \"seen\")?;\n    assert!(seen != Some(false));\n    ask(ctx)?;\n    Ok(())\n}\n";
    let error = derive(source).unwrap_err();
    assert!(error.contains("declare `remember` twice"), "{error}");
    // With another name for the inner function, the contract keeps the key and its check.
    let renamed = source
        .replace("    fn remember(_ctx", "    fn note(_ctx")
        .replace(
            "    remember(ctx)?;\n    allowit::new",
            "    note(ctx)?;\n    allowit::new",
        );
    let derived = derive(&renamed).unwrap();
    assert_eq!(keys(&renamed), [("seen".to_owned(), Keeper::Chain, false)]);
    assert!(derived.source.contains("fn remember(") && derived.source.contains("assert!(seen"));
    assert!(!derived.source.contains("fn ask("));
}

/// A statement that reads a standard figure stays, and the contract keeps that figure for it,
/// beside the figures of the kept rules.
#[test]
fn a_read_of_a_standard_figure_stays_and_the_wallet_keeps_the_figure() {
    let source = include_str!("fixtures/derive/stored-state.rs");
    let derived = derive(source).unwrap();
    assert_eq!(derived.source, source);
    assert!(derived.server_only.is_empty() && derived.state.is_empty());
    assert_eq!(
        derived.kept,
        [
            "payment_limit",
            "assert",
            "destinations",
            "daily_spending_limit"
        ]
    );
    let figures = |source: &str| -> Vec<(&'static str, bool)> {
        derive(source)
            .unwrap()
            .figures
            .iter()
            .map(|figure| (figure.key, figure.daily))
            .collect()
    };
    assert_eq!(
        figures(source),
        [
            ("calls", false),
            ("day_spent", true),
            ("day_unpriced", true)
        ]
    );
    // Each standard key keeps the figures of the rules that read it; `day` keeps none.
    let read = |key: &str| {
        figures(&format!(
            "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {{\n    let _figure: Option<AllowitValue> = allowit::get_state(ctx, \"{key}\")?;\n    Ok(())\n}}\n"
        ))
    };
    assert_eq!(read("spent"), [("spent", false), ("unpriced", false)]);
    assert_eq!(read("unpriced"), read("spent"));
    assert_eq!(
        read("day_total"),
        [("day_total", true), ("day_unpriced", true)]
    );
    assert_eq!(
        read("day_unpriced"),
        [
            ("day_spent", true),
            ("day_total", true),
            ("day_unpriced", true)
        ]
    );
    assert!(read("day").is_empty());
    // The stored day is the day of the last admitted operation. A contract wallet stores no
    // day, so a statement that reads it leaves: only the server evaluates it.
    let day = source.replace(
        "    assert!(calls != Some(1_000));",
        "    let day: Option<u64> = allowit::get_state(ctx, \"day\")?;\n    assert!(day.is_none() || calls != Some(1_000));",
    );
    assert_ne!(day, source);
    let derived = derive(&day).unwrap();
    assert!(!derived.source.contains("\"day\"") && derived.source.contains("\"calls\""));
    let [check] = &derived.server_only[..] else {
        panic!("one check leaves");
    };
    assert_eq!(
        (check.check.as_str(), check.cause, check.movable()),
        ("assert", Cause::StoredState, false)
    );
    assert!(derived.state.is_empty());

    // A read that depends on a server function leaves, and keeps no figure for the contract.
    let served = source.replace(
        "    assert!(calls != Some(1_000));",
        "    let score = allowit::decision(ctx, &allowit::AllowitDecision { id: \"a\", query: \"Is this research?\", deny_below: 1, allow_above: 2 })?;\n    assert!(calls != Some(1_000) && score > 1);",
    );
    let derived = derive(&served).unwrap();
    let causes: Vec<Cause> = derived
        .server_only
        .iter()
        .map(|check| check.cause)
        .collect();
    assert_eq!(causes, [Cause::ServerValue]);
    assert!(
        derived
            .source
            .contains("allowit::get_state(ctx, \"calls\")")
    );
}

/// Each kept canonical rule names the figures that it reads; a check that keeps no figure,
/// such as `payment_limit`, names none.
#[test]
fn the_figures_are_those_that_the_kept_rules_read() {
    let sample = derive(include_str!("fixtures/derive/sample-mandate.rs")).unwrap();
    let keys: Vec<&str> = sample.figures.iter().map(|figure| figure.key).collect();
    assert_eq!(
        keys,
        [
            "spent",
            "unpriced",
            "day_spent",
            "day_total",
            "day_unpriced"
        ]
    );
    let only = derive(include_str!("fixtures/derive/only-server-functions.rs")).unwrap();
    assert!(only.figures.is_empty());
    let calls = derive(
        "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    allowit::call_limit(ctx, &ctx.params[allowit::CALL_LIMIT])?;\n    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n    Ok(())\n}\n",
    )
    .unwrap();
    assert_eq!(calls.figures.len(), 1);
    assert_eq!(
        (calls.figures[0].key, calls.figures[0].daily),
        ("calls", false)
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
/// refused for a state call that the contract cannot declare, and the result is its own
/// on-chain part.
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
                error.contains("names each state key as a string literal")
                    || error.contains("ASCII letters, digits or `_`")
                    || error.contains("cannot write the standard figure"),
                "{error}\n{source}"
            ),
        }
    }
    assert!(derived_count > 100, "{derived_count}");
    assert!(changed > 0);
}

/// The property that the derivation keeps: the on-chain part never denies a request that the
/// whole source allows, and it then stores the same figures and the same state of every key
/// that the contract keeps. Every golden source runs with its on-chain part over the same
/// requests, params, stored figures and server answers. The contract holds no key that the
/// server keeps, so the on-chain part runs without those keys.
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
            "ceiling",
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
    // No state; the figures of a wallet that made many payments today; and the same figures
    // with the policies' own state, below their limits and at them.
    let figures = || {
        let mut state = AllowitMap::new();
        let mut set = |key: &str, value: AllowitValue| {
            state.insert(key.into(), value);
        };
        set("calls", AllowitValue::Int(1_000));
        set("spent", AllowitValue::Int(450));
        set("day", AllowitValue::Int(1_800_000_000 / 86_400));
        set("day_spent", AllowitValue::Int(450));
        set("day_total", AllowitValue::Int(450));
        state
    };
    let own = |payments: i128, score: i128, paid: i128, flagged: bool| {
        let mut state = figures();
        let mut set = |key: &str, value: AllowitValue| {
            state.insert(key.into(), value);
        };
        set("day_payments", AllowitValue::Int(payments));
        set("seen", AllowitValue::Bool(true));
        set("last_score", AllowitValue::Int(score));
        set("paid", AllowitValue::Int(paid));
        set("passed", AllowitValue::Int(1));
        set("flagged", AllowitValue::Bool(flagged));
        state
    };
    let states = [
        AllowitMap::new(),
        figures(),
        own(2, 5_000, 3, false),
        own(3, 0, 9, true),
    ];
    let operations = [
        AllowitOperation::transfer,
        AllowitOperation::swap,
        AllowitOperation::redeem,
        AllowitOperation::deposit,
    ];
    let (mut allowed, mut denied_on_chain, mut runs) = (0, 0, 0);
    // For each source, the allowed runs that changed a key that the contract keeps.
    let mut wrote: std::collections::BTreeMap<&str, usize> = Default::default();
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
        ("labeled-block", labeled_block::BOTH),
        ("text-temporary", text_temporary::BOTH),
        ("return-before-item", return_before_item::BOTH),
        ("helper-return", helper_return::BOTH),
        ("late-value", late_value::BOTH),
        ("attributed-import", attributed_import::BOTH),
        ("stored-state", stored_state::BOTH),
        ("type-named-binding", type_named_binding::BOTH),
        ("unsplit-tail", unsplit_tail::BOTH),
        ("server-range", server_range::BOTH),
        ("block-condition", block_condition::BOTH),
        ("state-on-chain", state_on_chain::BOTH),
        ("state-on-server", state_on_server::BOTH),
        ("state-helpers", state_helpers::BOTH),
    ] {
        // The keys that the server keeps: the contract never holds them.
        let source = GOLDEN
            .iter()
            .find(|(golden, ..)| *golden == name)
            .unwrap()
            .1;
        let state = derive(source).unwrap().state;
        let kept_by = |keeper: Keeper| -> Vec<String> {
            state
                .iter()
                .filter(|entry| entry.keeper == keeper)
                .map(|entry| entry.key.clone())
                .collect()
        };
        let (server_keys, chain_keys) = (kept_by(Keeper::Server), kept_by(Keeper::Chain));
        let on_chain_state = |state: &AllowitMap| {
            let mut state = state.clone();
            for key in &server_keys {
                state.remove(key.as_str());
            }
            state
        };
        for operation in &operations {
            for amount in [1i128, 400, 6_000_000] {
                for recipient in ["SELLER", "STRANGER"] {
                    for limit in [500i128, 10_000_000] {
                        for (answer, state) in [(false, 0), (true, 0), (true, 4_000), (true, 9_500)]
                            .into_iter()
                            .flat_map(|answer| states.iter().map(move |state| (answer, state)))
                        {
                            let context = |state: AllowitMap| AllowitContext {
                                env: AllowitEnv::new(),
                                params: AllowitParams::from(params(limit)),
                                state,
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
                            let whole =
                                evaluate(&context(state.clone()), both[0], AllowitTier::Custom);
                            // The contract has no server: its policy must not need one.
                            served::ANSWER.set((false, 0));
                            let on_chain = evaluate(
                                &context(on_chain_state(state)),
                                both[1],
                                AllowitTier::Custom,
                            );
                            runs += 1;
                            denied_on_chain += usize::from(on_chain.is_err());
                            if let Ok(after) = &whole {
                                allowed += 1;
                                // A write of the contract's own key: its value changed.
                                let changed = chain_keys
                                    .iter()
                                    .any(|key| state.get(key.as_str()) != after.get(key.as_str()));
                                *wrote.entry(name).or_default() += usize::from(changed);
                                let state = after;
                                assert_eq!(
                                    on_chain.as_ref(),
                                    Ok(&on_chain_state(state)),
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
    // Each source that keeps a key in the contract wrote it in some allowed runs, and no other
    // source wrote one.
    for (name, count) in &wrote {
        let stateful = ["state-on-chain", "state-on-server", "state-helpers"].contains(name);
        assert_eq!(*count > 0, stateful, "{name}: {count}");
    }
    assert_eq!(wrote.values().filter(|count| **count > 0).count(), 3);
}
