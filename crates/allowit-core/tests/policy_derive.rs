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
    state_braced_macro => "state-braced-macro",
    state_after_approval => "state-after-approval",
    negated_block_condition => "negated-block-condition",
    shadowed_after_macro => "shadowed-after-macro",
    statement_after_branch => "statement-after-branch",
    loop_in_condition => "loop-in-condition",
    range_loop => "range-loop",
    iterator_break => "iterator-break",
    inferred_number => "inferred-number",
    pruned_type => "pruned-type",
    arm_without_comma => "arm-without-comma",
    for_pattern => "for-pattern",
    arm_generic_comma => "arm-generic-comma",
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

/// A macro with braces is a statement that needs no `;`. A function that stands after it is
/// a function of its own: when one of its calls leaves, the server keeps the key that it
/// writes, and the other call leaves too. The same holds after a `const` block.
#[test]
fn a_function_after_a_statement_with_braces_follows_its_calls() {
    let source = include_str!("fixtures/derive/state-braced-macro.rs");
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    let derived = derive(source).unwrap();
    assert_eq!(
        keys(source),
        [
            ("seen".to_owned(), Keeper::Server, false),
            ("visits".to_owned(), Keeper::Chain, false)
        ]
    );
    for gone in ["fn remember(", "remember(ctx", "\"seen\""] {
        assert!(!derived.source.contains(gone), "{gone}");
    }
    assert_eq!(derived.kept, ["payment_limit", "assert"]);
    // With every call of the function before the server function, the contract keeps the key.
    let kept = source.replace("        remember(ctx, true)?;\n", "");
    assert!(keys(&kept).contains(&("seen".to_owned(), Keeper::Chain, false)));
    assert!(derive(&kept).unwrap().source.contains("fn remember("));
    // A macro with braces that a method call continues is one statement with that call.
    let continued = source.replace(
        "    assert! { ctx.request.amount > 0 }\n",
        "    matches! { ctx.request.amount, 1 }.then_some(());\n",
    );
    assert!(keys(&continued).contains(&("seen".to_owned(), Keeper::Server, false)));
    assert!(!derive(&continued).unwrap().source.contains("fn remember("));
    // A `const` block is such a statement too.
    let constant = source.replace(
        "    assert! { ctx.request.amount > 0 }\n",
        "    const { assert!(u8::MAX == 255) }\n",
    );
    assert!(keys(&constant).contains(&("seen".to_owned(), Keeper::Server, false)));
    assert!(!derive(&constant).unwrap().source.contains("fn remember("));
}

/// `!` before a block negates the block: only the name of a macro before `!` starts a macro.
/// A branch whose condition is such a block is one statement, so its denial never stays
/// without its condition.
#[test]
fn a_negated_block_condition_is_no_macro() {
    let source = include_str!("fixtures/derive/negated-block-condition.rs");
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    let derived = derive(source).unwrap();
    assert!(!derived.source.contains("DestinationNotAllowed"));
    assert_eq!(derived.kept, ["payment_limit"]);
    let [check] = &derived.server_only[..] else {
        panic!("one check leaves");
    };
    assert_eq!((check.line, check.cause), (6, Cause::NotSplit));
    for header in [
        "while ! { allowit::new_decision_context(ctx, None).is_ok() } {",
        "if matches! { allowit::new_decision_context(ctx, None), Ok(()) } == false {",
        "if !matches! { allowit::new_decision_context(ctx, None), Ok(()) } {",
        "if ! const { false } || allowit::new_decision_context(ctx, None).is_err() {",
    ] {
        let other = source.replace(
            "if ! { allowit::new_decision_context(ctx, None).is_ok() } {",
            header,
        );
        assert_ne!(other, source);
        let derived = derive(&other).unwrap();
        assert!(
            !derived.source.contains("DestinationNotAllowed"),
            "{header}"
        );
    }
}

/// A name that a leaving `let` binds leaves with it, also when the `let` stands after a
/// statement that needs no `;`: a later statement never reads an earlier value of the name.
#[test]
fn a_shadowed_name_after_a_statement_with_braces_leaves() {
    let source = include_str!("fixtures/derive/shadowed-after-macro.rs");
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    for (from, to) in [
        ("", ""),
        // A plain macro name, and an empty block after it.
        (
            "r#assert! { ctx.request.amount > 0 }",
            "assert! { ctx.request.amount > 0 } {}",
        ),
        // A `const` block, and a branch with a block after it.
        (
            "r#assert! { ctx.request.amount > 0 }",
            "const { assert!(true) }",
        ),
        (
            "r#assert! { ctx.request.amount > 0 }",
            "if ctx.request.amount > 0 {} {}",
        ),
        (
            "r#assert! { ctx.request.amount > 0 }",
            "if ctx.request.amount > 0 {} - 1;",
        ),
    ] {
        let other = source.replace(from, to);
        assert_eq!(other == source, from.is_empty());
        let derived = derive(&other).unwrap();
        assert_eq!(
            keys(&other),
            [("last_score".to_owned(), Keeper::Server, false)],
            "{to}"
        );
        assert!(!derived.source.contains("last_score"), "{to}");
    }
}

/// A raw name such as `r#mut` is a name. When a `let` that binds it leaves, a later statement
/// that reads the name leaves too.
#[test]
fn a_binding_with_a_raw_keyword_name_is_followed() {
    let source = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    let r#mut = false;\n    let r#mut = allowit::new_decision_context(ctx, None).is_ok();\n    allowit::set_state(ctx, \"paid\", r#mut)?;\n    assert!(r#mut);\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    for name in ["r#mut", "r#ref", "seen"] {
        let other = source.replace("r#mut", name);
        let derived = derive(&other).unwrap();
        assert_eq!(
            keys(&other),
            [("paid".to_owned(), Keeper::Server, false)],
            "{name}"
        );
        assert!(
            !derived.source.contains("paid") && !derived.source.contains("assert"),
            "{name}"
        );
    }
}

/// A call names a local function only in the block that holds the function. Outside that
/// block the name is another item, such as `Some`, and its call writes nothing.
#[test]
fn a_call_outside_the_block_of_a_function_is_no_call_of_it() {
    let source = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    {\n        fn Some(ctx: &AllowitContext) -> allowit::Result {\n            allowit::set_state(ctx, \"paid\", true)\n        }\n    }\n    let paid: bool = allowit::get_state(ctx, \"paid\")?.unwrap_or(false);\n    assert!(!paid);\n    allowit::set_state(ctx, \"paid\", true)?;\n    let _answer = Some(allowit::new_decision_context(ctx, None)?);\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(keys(source), [("paid".to_owned(), Keeper::Chain, false)]);
    assert!(derive(source).unwrap().source.contains("assert!(!paid)"));
    // Inside the block, the call is a call of the function: a leaving call moves the key.
    let inside = source.replace(
        "        }\n    }\n",
        "        }\n        if allowit::new_decision_context(ctx, None).is_ok() {\n            Some(ctx)?;\n        }\n    }\n",
    );
    assert_ne!(inside, source);
    assert_eq!(keys(&inside), [("paid".to_owned(), Keeper::Server, false)]);
}

/// A `const` block in the condition of a branch is an operand, and the branch ends at its own
/// body: a statement after the branch stays by itself.
#[test]
fn a_const_block_in_a_condition_is_an_operand() {
    let source = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    if const { true } && allowit::new_decision_context(ctx, None).is_ok() {}\n    allowit::set_state(ctx, \"paid\", true)?;\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(keys(source), [("paid".to_owned(), Keeper::Chain, false)]);
    let derived = derive(source).unwrap();
    assert!(derived.source.contains("\"paid\"") && !derived.source.contains("const"));
}

/// A block-like statement ends at its last `}`, as the Rust parser ends it: a block, a
/// declaration or an operator after it starts the next statement. So the statements after
/// `if ok {} {}` are steps of their own, and a block that asks the server leaves alone.
#[test]
fn a_statement_after_a_block_like_statement_is_a_step_of_its_own() {
    let source = include_str!("fixtures/derive/statement-after-branch.rs");
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(keys(source), [("limit".to_owned(), Keeper::Chain, false)]);
    let derived = derive(source).unwrap();
    assert!(derived.source.contains("    const LIMIT: i128 = 3;\n"));
    assert!(
        derived
            .source
            .contains("assert!(ctx.request.amount < limit() * 1_000_000)")
    );
    assert!(!derived.source.contains("new_decision_context"));
    // A function after such a statement: a leaving call of it moves its key, and every
    // statement that names it leaves with it.
    let function = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    if ctx.request.amount > 0 {} {}\n    fn Some(ctx: &AllowitContext) -> Option<bool> {\n        allowit::set_state(ctx, \"n\", true).is_ok();\n        None\n    }\n    Some(ctx);\n    assert!(Some(ctx).is_none());\n    if allowit::new_decision_context(ctx, None).is_ok() {\n        Some(ctx);\n    }\n    Ok(())\n}\n";
    source_shape_with_host_calls(function, REGISTRY).unwrap();
    assert_eq!(keys(function), [("n".to_owned(), Keeper::Server, false)]);
    let derived = derive(function).unwrap();
    assert!(!derived.source.contains("Some"), "{}", derived.source);
    assert!(
        derived
            .source
            .contains("    if ctx.request.amount > 0 {} {}\n")
    );
    // A constant that shadows a constant of the file is refused: the rule cannot tell which
    // one a statement reads.
    let shadowed = source.replace("pub fn execute", "const LIMIT: i128 = 2;\npub fn execute");
    let error = derive(&shadowed).unwrap_err();
    assert!(error.contains("declare `LIMIT` twice"), "{error}");
    // So is one inside a statement that is not split.
    let nested = source.replace(
        "    const LIMIT: i128 = 3;\n",
        "    while { const LIMIT: i128 = 4; ctx.request.amount > LIMIT } {\n        return Err(AllowitError::DestinationNotAllowed);\n    }\n    const LIMIT: i128 = 3;\n",
    );
    assert_ne!(nested, source);
    let error = derive(&nested).unwrap_err();
    assert!(error.contains("declare `LIMIT` twice"), "{error}");
    // A method call after the last `}` continues the statement, as in Rust.
    let continued = source.replace(
        "    if ctx.request.amount > 0 {} {\n        allowit::new_decision_context(ctx, None)?;\n    }\n",
        "    if ctx.request.amount > 0 { 1 } else { 2 }.max(allowit::new_decision_context(ctx, None).map_or(0, |_| 1));\n",
    );
    assert_ne!(continued, source);
    let derived = derive(&continued).unwrap();
    assert!(!derived.source.contains("max(") && derived.source.contains("const LIMIT"));
}

/// A branch or a loop inside the condition of a branch is a value with its own blocks. The
/// body of the outer branch is the block after that value, so the body leaves with the
/// condition that asks the server.
#[test]
fn a_branch_or_a_loop_in_a_condition_keeps_the_body_with_its_condition() {
    let source = include_str!("fixtures/derive/loop-in-condition.rs");
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(keys(source), [("seen".to_owned(), Keeper::Server, false)]);
    for value in [
        "loop { break true; }",
        "if ctx.request.amount > 1 { true } else { false }",
        "match ctx.request.amount { 0 => false, _ => true }",
        "'a: loop { break 'a true; }",
        "{ while false {} true }",
    ] {
        let other = source.replace("loop { break true; }", value);
        assert_eq!(other == source, value == "loop { break true; }");
        source_shape_with_host_calls(&other, REGISTRY).unwrap();
        let derived = derive(&other).unwrap();
        assert_eq!(
            keys(&other),
            [("seen".to_owned(), Keeper::Server, false)],
            "{value}"
        );
        assert!(!derived.source.contains("seen"), "{value}");
    }
    // Without the server function in the condition, the branch stays whole on chain.
    let local = source.replace(" && allowit::new_decision_context(ctx, None).is_ok()", "");
    let derived = derive(&local).unwrap();
    assert_eq!(derived.source, local);
    assert_eq!(keys(&local), [("seen".to_owned(), Keeper::Chain, false)]);
}

/// In a header, a `{` after `..` opens the body, as the Rust parser reads it. The loop ends
/// at that body, and the `let` after it is a statement of its own: when it leaves, the write
/// that reads its name leaves too.
#[test]
fn a_block_after_an_open_range_is_the_body_of_the_loop() {
    let source = include_str!("fixtures/derive/range-loop.rs");
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(
        keys(source),
        [("passed_last".to_owned(), Keeper::Server, false)]
    );
    let derived = derive(source).unwrap();
    assert!(derived.source.contains("for i in 0u64.. {") && !derived.source.contains("passed"));
    let header = source.replace(
        "    for i in 0u64.. {\n        if i > 2 {\n            break;\n        }\n    }\n",
        "    while ctx.request.amount > 0.. {\n        break;\n    }\n",
    );
    assert_ne!(header, source);
    assert_eq!(
        keys(&header),
        [("passed_last".to_owned(), Keeper::Server, false)]
    );
}

/// Where a `;` is missing, the parser reads the next statement with the one before. A `let`
/// or an item inside such a group would stay seen by later statements while it left with the
/// group, so a source with a server part is refused there.
#[test]
fn a_declaration_that_the_parser_reads_with_the_statement_before_is_refused() {
    let source = include_str!("fixtures/derive/range-loop.rs");
    for (from, to, line) in [
        // After a call.
        (
            "PAYMENT_LIMIT])?;\n    let passed: bool",
            "PAYMENT_LIMIT])?\n    let passed: bool",
            5,
        ),
        // After a `let`, with a loop between.
        ("let passed: bool = false;", "let passed: bool = false", 11),
        // An attribute before the `let`.
        (
            "PAYMENT_LIMIT])?;\n    let passed: bool",
            "PAYMENT_LIMIT])?\n    #[allow(unused_variables)]\n    let passed: bool",
            6,
        ),
    ] {
        let other = source.replace(from, to);
        assert_ne!(other, source);
        source_shape_with_host_calls(&other, REGISTRY).unwrap();
        let error = derive(&other).unwrap_err();
        let expected = format!("must end the statement before line {line} with `;`");
        assert!(error.contains(&expected), "{to}: {error}");
        // Without a server part, the source is its own on-chain part.
        let local = other.replace("allowit::new_decision_context(ctx, None).is_ok()", "true");
        assert_eq!(derive(&local).unwrap().source, local);
    }
    // Generic arguments end where they close: the loop is a statement of its own.
    let generic = source.replace(
        "    for i in 0u64.. {\n        if i > 2 {\n            break;\n        }\n    }\n",
        "    while ctx.request.amount == u64::MAX::<> {}\n",
    );
    assert_ne!(generic, source);
    assert_eq!(
        keys(&generic),
        [("passed_last".to_owned(), Keeper::Server, false)]
    );
    assert!(!derive(&generic).unwrap().source.contains("passed"));
}

/// The iterated value of a `for` runs before the loop: a `break` there ends the enclosing loop.
/// When that `break` depends on the server, the enclosing loop leaves with its write.
#[test]
fn an_exit_in_the_iterated_value_of_a_for_ends_the_enclosing_loop() {
    let source = include_str!("fixtures/derive/iterator-break.rs");
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(keys(source), [("seen".to_owned(), Keeper::Server, false)]);
    assert!(!derive(source).unwrap().source.contains("loop"));
    // An exit there that needs no server stays, with the loop and its write.
    let local = source.replace(
        "allowit::new_decision_context(ctx, None).is_ok()",
        "ctx.request.amount > 5",
    );
    assert_ne!(local, source);
    assert_eq!(keys(&local), [("seen".to_owned(), Keeper::Chain, false)]);
    assert_eq!(derive(&local).unwrap().source, local);
}

/// A `const _` that uses a key of the server or calls the server leaves. No statement that
/// stays then uses that key.
#[test]
fn an_anonymous_constant_that_uses_a_server_key_leaves() {
    let source = "use allowit::prelude::*;\nconst _: () = {\n    fn unused(ctx: &AllowitContext) -> bool {\n        let flag: Option<bool> = allowit::get_state(ctx, \"flag\").unwrap_or(None);\n        flag.unwrap_or(false)\n    }\n};\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    let flag: bool = allowit::get_state(ctx, \"flag\")?.unwrap_or(false);\n    if flag {\n        return Err(AllowitError::DestinationNotAllowed);\n    }\n    if allowit::new_decision_context(ctx, None).is_ok() {\n        allowit::set_state(ctx, \"flag\", true)?;\n    }\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(keys(source), [("flag".to_owned(), Keeper::Server, false)]);
    let derived = derive(source).unwrap();
    assert!(!derived.source.contains("flag") && !derived.source.contains("const _"));
    // A `const _` that uses no server value stays.
    let local = source.replace("allowit::get_state(ctx, \"flag\").unwrap_or(None)", "None");
    assert!(derive(&local).unwrap().source.contains("const _"));
}

/// A local function is seen only in the block that declares it. When it leaves, a statement
/// outside that block that names another item of the same name, such as `Some`, stays.
#[test]
fn a_helper_is_named_only_in_the_block_that_declares_it() {
    let source = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    {\n        fn Some(ctx: &AllowitContext) -> allowit::Result {\n            allowit::set_state(ctx, \"flag\", true)\n        }\n        if allowit::new_decision_context(ctx, None).is_ok() {\n            Some(ctx)?;\n        }\n    }\n    let seen = Some(true);\n    allowit::set_state(ctx, \"keep\", seen.is_some())?;\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(
        keys(source),
        [
            ("flag".to_owned(), Keeper::Server, false),
            ("keep".to_owned(), Keeper::Chain, false)
        ]
    );
    let derived = derive(source).unwrap();
    assert!(derived.source.contains("let seen = Some(true);"));
    assert!(!derived.source.contains("fn Some") && !derived.source.contains("\"flag\""));
}

/// A number without a type takes its type from the code that uses it, and `i32` when no code
/// does. A leaving statement can give that type, so a kept statement that reads a name of such
/// a number, or a name that shares its type, leaves too. A stated type keeps it on chain.
#[test]
fn a_number_whose_type_a_leaving_statement_gives_is_followed() {
    let source = include_str!("fixtures/derive/inferred-number.rs");
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(keys(source), [("flag".to_owned(), Keeper::Server, false)]);
    assert!(!derive(source).unwrap().source.contains("flag"));
    let flag = |other: &str| keys(other).into_iter().next().map(|(_, keeper, _)| keeper);
    for (from, to, keeper) in [
        // A stated type, or a suffix, fixes the type on both sides.
        ("let n = ", "let n: u64 = ", Keeper::Chain),
        // A branch is no form whose type the rule knows: it counts as open.
        (
            "if ctx.request.amount > 0 { 1 } else { 2 }",
            "if ctx.request.amount > 0i128 { 1u64 } else { 2u64 }",
            Keeper::Server,
        ),
        // A value of the request has its own type.
        (
            "if ctx.request.amount > 0 { 1 } else { 2 }",
            "ctx.request.amount",
            Keeper::Chain,
        ),
        // A name that takes its type from the number shares it.
        (
            "    let n = if",
            "    let m = 1;\n    let n = m + if",
            Keeper::Server,
        ),
        // So does a name that one statement uses with it.
        (
            "    allowit::set_state(ctx, \"flag\", (n << 31) > 0)?;",
            "    let k = 1;\n    assert!(k <= n);\n    allowit::set_state(ctx, \"flag\", (k << 31) > 0)?;",
            Keeper::Server,
        ),
    ] {
        let other = source.replace(from, to);
        assert_ne!(other, source, "{to}");
        source_shape_with_host_calls(&other, REGISTRY).unwrap();
        assert_eq!(flag(&other), Some(keeper), "{to}");
    }
    // A name that a `for` binds from numbers without a type.
    let looped = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    for i in 0..2 {\n        if allowit::new_decision_context(ctx, None).is_ok() {\n            assert!(i < 2u64);\n        }\n        allowit::set_state(ctx, \"flag\", (i << 31) > 0)?;\n    }\n    Ok(())\n}\n";
    source_shape_with_host_calls(looped, REGISTRY).unwrap();
    assert_eq!(flag(looped), Some(Keeper::Server));
    assert_eq!(
        flag(&looped.replace("0..2", "0u64..2u64")),
        Some(Keeper::Chain)
    );
}

/// The type of a number can also pass through a name that holds no number, such as `None`, or
/// through a read whose type its uses give. Such names are followed too; a stated type or a
/// turbofish fixes them.
#[test]
fn an_open_type_that_passes_through_another_name_is_followed() {
    let source = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    let n = 1;\n    let other = None;\n    assert_ne!(other, Some(n));\n    if allowit::new_decision_context(ctx, None).is_ok() {\n        assert_ne!(other, Some(1u64));\n    }\n    allowit::set_state(ctx, \"flag\", (n << 31) > 0)?;\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    let flag = |source: &str| {
        keys(source)
            .into_iter()
            .find(|(key, ..)| key == "flag")
            .map(|(_, keeper, _)| keeper)
    };
    assert_eq!(flag(source), Some(Keeper::Server));
    let typed = source
        .replace("let n = 1;", "let n: u64 = 1;")
        .replace("let other = None;", "let other: Option<u64> = None;");
    assert_eq!(flag(&typed), Some(Keeper::Chain));
    // A state read without a stated type takes its type from its uses.
    let read = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    let paid = allowit::get_state(ctx, \"paid\")?;\n    if allowit::new_decision_context(ctx, None).is_ok() {\n        assert!(paid != Some(5u64));\n    }\n    allowit::set_state(ctx, \"flag\", paid == Some(1))?;\n    Ok(())\n}\n";
    source_shape_with_host_calls(read, REGISTRY).unwrap();
    assert_eq!(flag(read), Some(Keeper::Server));
    let turbofish = read.replace("get_state(ctx", "get_state::<u64>(ctx");
    assert_eq!(flag(&turbofish), Some(Keeper::Chain));
    let open = read.replace("get_state(ctx", "get_state::<_>(ctx");
    assert_eq!(flag(&open), Some(Keeper::Server));
    // Any form that the rule does not know counts as open: a raw spelling, a call of a local
    // function, an owner param read with `.ok()`.
    for (from, to) in [
        ("let other = None;", "let other = r#None;"),
        ("let other = None;", "let other = empty();"),
    ] {
        let other = source.replace(from, to).replace(
            "pub fn execute",
            "fn empty<Result>() -> Option<Result> {\n    None\n}\npub fn execute",
        );
        source_shape_with_host_calls(&other, REGISTRY).unwrap();
        assert_eq!(flag(&other), Some(Keeper::Server), "{to}");
    }
    let param = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    let n = ctx.params[\"threshold\"].ok();\n    if allowit::new_decision_context(ctx, None).is_ok() {\n        assert_eq!(n, 1u64);\n    }\n    assert_eq!(Some(n), Some(1));\n    allowit::set_state(ctx, \"flag\", true)?;\n    Ok(())\n}\n";
    source_shape_with_host_calls(param, REGISTRY).unwrap();
    let derived = derive(param).unwrap();
    assert!(
        !derived.source.contains("assert_eq!(Some(n)"),
        "{}",
        derived.source
    );
    let typed_param = param.replace("let n = ", "let n: u64 = ");
    assert!(
        derive(&typed_param)
            .unwrap()
            .source
            .contains("assert_eq!(Some(n)")
    );
    // `Ok(())` leaves its error type open.
    let result = source
        .replace("let other = None;", "let other = Ok(());")
        .replace("assert_ne!(other, Some(n));", "assert!(other != Err(n));")
        .replace(
            "assert_ne!(other, Some(1u64));",
            "assert!(other != Err(1u64));",
        );
    assert_ne!(result, source);
    source_shape_with_host_calls(&result, REGISTRY).unwrap();
    assert_eq!(flag(&result), Some(Keeper::Server));
}

/// A `let` that only leaving code reads leaves too. When it states a type, it can be what
/// gives the type of a name that a kept statement reads: that name is followed too.
#[test]
fn a_type_that_a_removed_let_gives_is_followed() {
    let source = include_str!("fixtures/derive/pruned-type.rs");
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(keys(source), [("flag".to_owned(), Keeper::Server, false)]);
    let derived = derive(source).unwrap();
    assert!(!derived.source.contains("flag") && !derived.source.contains("let copy"));
    let typed = source.replace("let n = 1;", "let n: u64 = 1;");
    assert_eq!(keys(&typed), [("flag".to_owned(), Keeper::Chain, false)]);
}

/// A match arm whose body is block-like, such as `if ok {}`, needs no comma. When it leaves,
/// the arm after it stays.
#[test]
fn an_arm_without_a_comma_leaves_alone() {
    let source = include_str!("fixtures/derive/arm-without-comma.rs");
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    let derived = derive(source).unwrap();
    assert!(
        derived
            .source
            .contains("        1 => {}\n        400 => {},\n")
    );
    assert_eq!(
        keys(source),
        [("fallback".to_owned(), Keeper::Chain, false)]
    );
    // A method call after the body continues it, up to the comma.
    let continued = source.replace(
        "1 => if allowit::new_decision_context(ctx, None).is_ok() {}\n",
        "1 => if allowit::new_decision_context(ctx, None).is_ok() { 1 } else { 2 }.max(1),\n",
    );
    assert_ne!(continued, source);
    assert!(derive(&continued).unwrap().source.contains("400 => {},"));
    // A `const` block needs no comma either.
    let constant = source
        .replace(
            "1 => if allowit::new_decision_context(ctx, None).is_ok() {}\n",
            "1 => const {}\n",
        )
        .replace(
            "400 => {},",
            "400 => {\n            allowit::new_decision_context(ctx, None)?;\n        },",
        );
    assert_ne!(constant, source);
    source_shape_with_host_calls(&constant, REGISTRY).unwrap();
    let derived = derive(&constant).unwrap();
    assert!(
        derived.source.contains("1 => const {}\n"),
        "{}",
        derived.source
    );
    assert!(
        derived.source.contains("400 => {") && !derived.source.contains("new_decision_context")
    );
}

/// The pattern of a `for` can hold braces, and a `,` between generic arguments ends no arm:
/// both sources derive to a part that compiles (the golden test compiles it).
#[test]
fn a_for_pattern_with_braces_and_a_generic_comma_in_an_arm_derive_whole() {
    for name in ["for-pattern", "arm-generic-comma"] {
        let source = GOLDEN
            .iter()
            .find(|(golden, ..)| *golden == name)
            .unwrap()
            .1;
        source_shape_with_host_calls(source, REGISTRY).unwrap();
        assert_eq!(
            keys(source),
            [("flag".to_owned(), Keeper::Server, false)],
            "{name}"
        );
    }
    let arm = derive(include_str!("fixtures/derive/arm-generic-comma.rs")).unwrap();
    assert!(
        arm.source.contains("        1 => {},\n        _ => {}\n"),
        "{}",
        arm.source
    );
}

/// The on-chain part leaves code out, so it states `warn` where the source states `deny` or
/// `forbid`: a lint that turns unused code into an error could stop it from compiling.
#[test]
fn a_lint_that_denies_is_a_warning_in_the_on_chain_part() {
    let source = "use allowit::prelude::*;\n#[deny(unused_variables)]\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    let calls: u64 = allowit::get_state(ctx, \"calls\")?.unwrap_or(0);\n    assert!(ctx.request.amount > 0);\n    if allowit::new_decision_context(ctx, None).is_ok() {\n        assert!(calls < 5);\n    }\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    let derived = derive(source).unwrap();
    assert!(
        derived.source.contains("#[warn(unused_variables)]"),
        "{}",
        derived.source
    );
    assert_eq!(derived.left, [(6, 8)]);
    // Also under the raw spelling of the lint level.
    let raw = source.replace("#[deny(", "#[r#forbid(");
    assert_ne!(raw, source);
    source_shape_with_host_calls(&raw, REGISTRY).unwrap();
    assert!(
        derive(&raw)
            .unwrap()
            .source
            .contains("#[warn(unused_variables)]")
    );
    // Without a server part, the source is its own on-chain part.
    let local = source.replace("allowit::new_decision_context(ctx, None).is_ok()", "true");
    assert_eq!(derive(&local).unwrap().source, local);
}

/// `const fn` is one declaration, not a statement read with the next one.
#[test]
fn a_const_fn_is_one_declaration() {
    let source = "use allowit::prelude::*;\nconst fn limit() -> u64 {\n    1\n}\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    allowit::new_decision_context(ctx, None)?;\n    assert!(limit() > 0);\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    let derived = derive(source).unwrap();
    assert!(
        derived.source.contains("const fn limit()")
            && derived.source.contains("assert!(limit() > 0)")
    );
}

/// A pattern that names a constructor of the language, such as `None`, binds no name: a
/// later `None` keeps its open type.
#[test]
fn a_constructor_in_a_pattern_is_no_binding_of_a_fixed_type() {
    let source = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    match ctx.request.token_out {\n        None => {}\n        _ => {}\n    }\n    let x = None;\n    let n = 1;\n    assert!(x != Some(n));\n    if allowit::new_decision_context(ctx, None).is_ok() {\n        let _typed: Option<u64> = x;\n    }\n    allowit::set_state(ctx, \"flag\", (n << 31) > 0)?;\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(keys(source), [("flag".to_owned(), Keeper::Server, false)]);
}

/// The `const` parameters of generic functions are no items: two of them with one name are
/// no two items with one name, and none of them is a declaration inside a statement.
#[test]
fn const_parameters_are_no_items() {
    let source = "use allowit::prelude::*;\nfn first<const N: u64>() -> u64 {\n    N\n}\nfn second<const N: u64>() -> u64 {\n    N\n}\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    allowit::new_decision_context(ctx, None)?;\n    Ok(())\n}\n";
    derive(source).unwrap();
    // Also with an attribute before the parameter.
    let attributed = source.replace("<const N", "<#[allow(dead_code)] const N");
    assert_ne!(attributed, source);
    derive(&attributed).unwrap();
}

/// A check outside the block of a helper that left names another item of the helper's name.
/// It leaves because of its own branch, so the report says that the owner can move it out.
#[test]
fn a_check_that_names_another_item_of_a_helpers_name_reports_its_own_cause() {
    let source = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    {\n        fn Some(ctx: &AllowitContext) -> allowit::Result {\n            allowit::set_state(ctx, \"flag\", true)\n        }\n        if allowit::new_decision_context(ctx, None).is_ok() {\n            Some(ctx)?;\n        }\n    }\n    if allowit::new_decision_context(ctx, None).is_ok() {\n        assert!(Some(true).is_some());\n    }\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    let derived = derive(source).unwrap();
    let [check] = &derived.server_only[..] else {
        panic!("one check leaves: {:?}", derived.server_only);
    };
    assert_eq!(
        (
            check.line,
            check.check.as_str(),
            check.cause,
            check.movable()
        ),
        (12, "assert", Cause::ServerBranch, true)
    );
}

/// `const _` binds no name: two of them are no two items with one name.
#[test]
fn anonymous_constants_are_no_duplicate_names() {
    let source = "use allowit::prelude::*;\nfn check() -> u64 {\n    const _: u64 = 0;\n    const _: u64 = 1;\n    1\n}\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    assert!(check() == 1);\n    allowit::new_decision_context(ctx, None)?;\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    let derived = derive(source).unwrap();
    assert!(derived.source.contains("assert!(check() == 1)") && derived.source.contains("const _"));
}

/// A function that leaves writes nothing by that: only a call of it writes. A function that
/// no statement calls moves no key to the server.
#[test]
fn a_function_that_no_statement_calls_moves_no_key() {
    let source = "use allowit::prelude::*;\nfn ask(ctx: &AllowitContext) -> allowit::Result {\n    fn reset(ctx: &AllowitContext) -> allowit::Result {\n        allowit::set_state(ctx, \"n\", 0u64)\n    }\n    allowit::new_decision_context(ctx, None)\n}\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    let n: u64 = allowit::get_state(ctx, \"n\")?.unwrap_or(0);\n    assert!(n < 3);\n    allowit::set_state(ctx, \"n\", n + 1)?;\n    ask(ctx)?;\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(keys(source), [("n".to_owned(), Keeper::Chain, false)]);
    let derived = derive(source).unwrap();
    assert!(derived.source.contains("assert!(n < 3)") && derived.source.contains("n + 1"));
    assert!(!derived.source.contains("fn ask(") && !derived.source.contains("fn reset("));
    // With a call of the inner function, the leaving call of the outer one moves the key.
    let called = source.replace(
        "    allowit::new_decision_context(ctx, None)\n",
        "    reset(ctx)?;\n    allowit::new_decision_context(ctx, None)\n",
    );
    assert_ne!(called, source);
    assert_eq!(keys(&called), [("n".to_owned(), Keeper::Server, false)]);
    assert!(!derive(&called).unwrap().source.contains("\"n\""));
    // A function at the file level that no statement calls moves no key either.
    let unused = source.replace(
        "pub fn execute",
        "fn unused(ctx: &AllowitContext) -> allowit::Result {\n    allowit::new_decision_context(ctx, None)?;\n    allowit::set_state(ctx, \"n\", 9u64)\n}\npub fn execute",
    );
    assert_eq!(keys(&unused), [("n".to_owned(), Keeper::Chain, false)]);
}

/// A function that stands inside another function runs only where it is called. Its write is
/// no write of the function around it.
#[test]
fn a_function_inside_a_function_writes_only_where_it_is_called() {
    let source = "use allowit::prelude::*;\nuse allowit::AllowitDecision;\nfn outer(ctx: &AllowitContext) -> allowit::Result {\n    fn reset(ctx: &AllowitContext) -> allowit::Result {\n        allowit::set_state(ctx, \"n\", 0u64)\n    }\n    Ok(())\n}\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    outer(ctx)?;\n    let n: u64 = allowit::get_state(ctx, \"n\")?.unwrap_or(0);\n    assert!(n < 3);\n    allowit::set_state(ctx, \"n\", n + 1)?;\n    let score = allowit::decision(ctx, &AllowitDecision { id: \"a\", query: \"Is this research?\", deny_below: 1, allow_above: 2 })?;\n    if score > 5000 {\n        outer(ctx)?;\n    }\n    Ok(())\n}\n";
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    assert_eq!(keys(source), [("n".to_owned(), Keeper::Chain, false)]);
    let derived = derive(source).unwrap();
    assert!(derived.source.contains("assert!(n < 3)") && derived.source.contains("n + 1"));
    assert_eq!(derived.source.matches("outer(ctx)?;").count(), 1);
    // With a call of the inner function, a leaving call of the outer one moves the key.
    let called = source.replace("    Ok(())\n}\npub fn", "    reset(ctx)\n}\npub fn");
    assert_ne!(called, source);
    assert_eq!(keys(&called), [("n".to_owned(), Keeper::Server, false)]);
    assert!(!derive(&called).unwrap().source.contains("\"n\""));
}

/// A `const` block that a method call continues is a statement, and no item. After an early
/// approval on a server value it leaves as every later statement does, and the server keeps
/// the key that it writes.
#[test]
fn a_continued_const_block_after_an_early_approval_leaves() {
    let source = include_str!("fixtures/derive/state-after-approval.rs");
    source_shape_with_host_calls(source, REGISTRY).unwrap();
    let derived = derive(source).unwrap();
    assert_eq!(
        keys(source),
        [
            ("marked".to_owned(), Keeper::Server, false),
            ("visits".to_owned(), Keeper::Chain, false)
        ]
    );
    assert!(!derived.source.contains("marked") && !derived.source.contains("const"));
    assert_eq!(derived.kept, ["payment_limit"]);
    // Before the server function, the statement stays and the contract keeps its key.
    let early = source
        .replace(
            "    const { true }.then_some(allowit::set_state(ctx, \"marked\", true)?);\n",
            "",
        )
        .replace(
            "    let visits",
            "    const { true }.then_some(allowit::set_state(ctx, \"marked\", true)?);\n    let visits",
        );
    assert!(keys(&early).contains(&("marked".to_owned(), Keeper::Chain, false)));
    assert!(derive(&early).unwrap().source.contains("\"marked\""));
}

/// A write reaches its callers through any number of local functions, also when a caller
/// stands before the function that it calls.
#[test]
fn a_write_reaches_its_callers_through_a_chain_of_local_functions() {
    let mut source = String::from("use allowit::prelude::*;\n");
    for i in 0..30 {
        source.push_str(&format!(
            "fn step{i}(ctx: &AllowitContext) -> allowit::Result {{\n    step{}(ctx)\n}}\n",
            i + 1
        ));
    }
    source.push_str("fn step30(ctx: &AllowitContext) -> allowit::Result {\n    allowit::set_state(ctx, \"seen\", true)\n}\n");
    let kept = format!(
        "{source}pub fn execute(ctx: &AllowitContext) -> allowit::Result {{\n    step0(ctx)?;\n    allowit::new_decision_context(ctx, None)?;\n    Ok(())\n}}\n"
    );
    assert_eq!(keys(&kept), [("seen".to_owned(), Keeper::Chain, false)]);
    let left = format!(
        "{source}pub fn execute(ctx: &AllowitContext) -> allowit::Result {{\n    let score = allowit::decision(ctx, &allowit::AllowitDecision {{ id: \"a\", query: \"Is this research?\", deny_below: 1, allow_above: 2 }})?;\n    if score > 5 {{\n        step0(ctx)?;\n    }}\n    Ok(())\n}}\n"
    );
    assert_eq!(keys(&left), [("seen".to_owned(), Keeper::Server, false)]);
    assert!(!derive(&left).unwrap().source.contains("fn step"));
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
    // A `,` between the generic arguments of a path ends no argument of the call.
    let generic = with(
        "    let n = allowit::get_state::<u64>(Result::Ok::<_, AllowitError>(ctx).unwrap(), \"count\")?;\n    allowit::set_state(Result::Ok::<&AllowitContext, AllowitError>(ctx).unwrap(), \"count\", n.unwrap_or(0) + 1)?;\n",
    );
    source_shape_with_host_calls(&generic, REGISTRY).unwrap();
    assert_eq!(keys(&generic), [("count".to_owned(), Keeper::Chain, false)]);
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
    // A read of a standard key keeps that figure and no other.
    let read = |key: &str| {
        figures(&format!(
            "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {{\n    let _figure: Option<AllowitValue> = allowit::get_state(ctx, \"{key}\")?;\n    Ok(())\n}}\n"
        ))
    };
    for (key, daily) in [
        ("calls", false),
        ("spent", false),
        ("unpriced", false),
        ("day_spent", true),
        ("day_total", true),
        ("day_unpriced", true),
    ] {
        assert_eq!(read(key), [(key, daily)]);
    }
    // The stored day is the day of the last admitted operation. A contract wallet stores no
    // day and keeps its daily figures in a window of its own, so the day has no value that the
    // chain and the server share: no policy reads it, in a statement that stays or in one that
    // leaves, and no policy writes it.
    let day = source.replace(
        "    assert!(calls != Some(1_000));",
        "    let day: Option<u64> = allowit::get_state(ctx, \"day\")?;\n    assert!(day.is_none() || calls != Some(1_000));",
    );
    assert_ne!(day, source);
    // In a branch that leaves whatever key it reads: its condition reads a server value.
    let served = source.replace(
        "    assert!(calls != Some(1_000));",
        "    let answer = allowit::decision(ctx, &allowit::AllowitDecision { id: \"day\", query: \"Is today a working day?\", deny_below: 5000, allow_above: 5000 })?;\n    if answer > 1 {\n        let day: Option<u64> = allowit::get_state(ctx, \"day\")?;\n        assert!(day.is_none() || calls != Some(1_000));\n    }",
    );
    assert_ne!(served, source);
    let branch = derive(&served.replace("\"day\")?", "\"spent\")?")).unwrap();
    assert!(!branch.source.contains("answer > 1") && branch.server_only.len() == 1);
    // The wallet keeps `day_spent` for the kept rule, in a window of its own: a statement that
    // left cannot read it.
    let error = derive(&served.replace("\"day\")?", "\"day_spent\")?")).unwrap_err();
    assert!(
        error.contains("depends on the daily figure \"day_spent\""),
        "{error}"
    );
    let written = source.replace(
        "    assert!(calls != Some(1_000));",
        "    allowit::set_state(ctx, \"day\", 1u64)?;",
    );
    assert_ne!(written, source);
    for refused in [&day, &served, &written] {
        let error = derive(refused).unwrap_err();
        assert!(error.contains("stored day \"day\""), "{error}");
    }

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
        set("visits", AllowitValue::Int(paid));
        set("marked", AllowitValue::Bool(flagged));
        state
    };
    // The first request of a new day. The keeper of a daily figure gives the figure of the
    // current window: the wallet leaves an ended one out, and the server evaluates on the
    // wallet's figures.
    let rolled = || {
        let mut state = figures();
        for key in ["day_spent", "day_total"] {
            state.remove(key);
        }
        state
    };
    let states = [
        AllowitMap::new(),
        figures(),
        rolled(),
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
        ("state-braced-macro", state_braced_macro::BOTH),
        ("state-after-approval", state_after_approval::BOTH),
        ("negated-block-condition", negated_block_condition::BOTH),
        ("shadowed-after-macro", shadowed_after_macro::BOTH),
        ("statement-after-branch", statement_after_branch::BOTH),
        ("loop-in-condition", loop_in_condition::BOTH),
        ("range-loop", range_loop::BOTH),
        ("iterator-break", iterator_break::BOTH),
        ("inferred-number", inferred_number::BOTH),
        ("pruned-type", pruned_type::BOTH),
        ("arm-without-comma", arm_without_comma::BOTH),
        ("for-pattern", for_pattern::BOTH),
        ("arm-generic-comma", arm_generic_comma::BOTH),
    ] {
        // The keys that the server keeps: the contract never holds them.
        let source = GOLDEN
            .iter()
            .find(|(golden, ..)| *golden == name)
            .unwrap()
            .1;
        let derived = derive(source).unwrap();
        let chain_keys: Vec<String> = derived
            .state
            .iter()
            .filter(|entry| entry.keeper == Keeper::Chain)
            .map(|entry| entry.key.clone())
            .collect();
        // What the contract holds: the keys that it keeps and the figures that the derivation
        // names. It holds no key of the server, no other figure and no key of another policy.
        let held: Vec<&str> = chain_keys
            .iter()
            .map(String::as_str)
            .chain(derived.figures.iter().map(|figure| figure.key))
            .collect();
        let project = |state: &AllowitMap| {
            let mut kept = AllowitMap::new();
            for key in &held {
                if let Some(value) = state.get(*key) {
                    kept.insert((*key).into(), value.clone());
                }
            }
            kept
        };
        // The contract's state for one evaluation. The stored day is no policy-visible value of
        // a contract: no policy reads it. The evaluator's own accounting of the daily figures
        // still needs it, as a contract knows the window of its daily entry.
        let on_chain_state = |state: &AllowitMap| {
            let mut kept = project(state);
            if let Some(day) = state.get("day") {
                kept.insert("day".into(), day.clone());
            }
            kept
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
                                    on_chain.as_ref().map(&project),
                                    Ok(project(state)),
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
        let stateful = [
            "state-on-chain",
            "state-on-server",
            "state-helpers",
            "state-braced-macro",
            "state-after-approval",
            "statement-after-branch",
            "arm-without-comma",
        ]
        .contains(name);
        assert_eq!(*count > 0, stateful, "{name}: {count}");
    }
    assert_eq!(wrote.values().filter(|count| **count > 0).count(), 7);
}

/// `source_calls` lists the canonical rules and the host calls of a source in source order, as
/// the derivation reads them: a raw identifier names the same rule, a comment or a literal holds no
/// call, whitespace and line breaks inside a path are part of the call, a longer path is no call
/// of its tail, a path that is not called is no call, and a portable function such as
/// `u64::from` is no host call.
#[test]
fn source_calls_lists_rules_and_host_calls_in_source_order() {
    use allowit_core::policy_source::{SourceCalls, source_calls};
    let source = "use allowit::prelude::*;\nuse allowit::AllowitDecision;\n// allowit::payment_limit(ctx) in a comment\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    /* allowit::tokens(ctx) /* catalog::quote(ctx) */ */\n    let text = \"allowit::destinations(ctx) catalog::quote(ctx)\";\n    let score = allowit::decision(ctx, &AllowitDecision { id: \"a\", query: \"Is this research?\", deny_below: 1, allow_above: 2 })?;\n    if score < 9000 {\n        allowit::call_limit(ctx, &ctx.params[allowit::CALL_LIMIT])?;\n    }\n    allowit::r#payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n    allowit :: call_limit (ctx, &ctx.params[allowit::CALL_LIMIT])?;\n    catalog\n        ::report(ctx)?;\n    outer::catalog::quote(ctx)?;\n    let f = allowit::tokens;\n    let n = u64::from(1u32);\n    assert!(text.len() > n as usize && f == f);\n    Ok(())\n}\n";
    assert_eq!(
        source_calls(source).unwrap(),
        SourceCalls {
            rules: vec!["call_limit".into(), "payment_limit".into()],
            host_calls: vec!["catalog::report".into()],
        }
    );
    // The rules that the on-chain part keeps are those of `Derived::kept`, in its order.
    let split = "use allowit::prelude::*;\nuse allowit::AllowitDecision;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    let score = allowit::decision(ctx, &AllowitDecision { id: \"a\", query: \"Is this research?\", deny_below: 1, allow_above: 2 })?;\n    if score < 9000 {\n        allowit::call_limit(ctx, &ctx.params[allowit::CALL_LIMIT])?;\n    }\n    allowit::r#payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n    allowit::call_limit(ctx, &ctx.params[allowit::CALL_LIMIT])?;\n    Ok(())\n}\n";
    assert_eq!(
        source_calls(split).unwrap().rules,
        ["call_limit", "payment_limit"]
    );
    assert_eq!(derive(split).unwrap().kept, ["payment_limit", "call_limit"]);
    // A rule called with generic arguments is a call; a host function called so is not.
    let turbofish = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    allowit::payment_limit::<>(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n    Ok(())\n}\n";
    assert_eq!(source_calls(turbofish).unwrap().rules, ["payment_limit"]);
    assert_eq!(derive(turbofish).unwrap().kept, ["payment_limit"]);
    assert!(source_calls("let s = \"unterminated;").is_err());
}

/// `daily_window_end`: the keeper of a daily value keeps its window after an operation by
/// Core's portable rule, which a rail's contract includes too. The end is the window's last
/// unit, inclusive. A read renews nothing, also when the operation takes effect after the window
/// ended; the first write after the window ended opens one at the operation's time; a write in
/// an open window keeps its end. An end past the clock's range is an error, never an ended window.
#[test]
fn a_daily_window_opens_with_the_first_write_after_it_ended() {
    use allowit_core::policy_source::{daily_window_end, daily_window_open};
    use allowit_core::{AllowitError, AllowitValue};
    const DAY: u64 = 86_400;
    let (one, two) = (AllowitValue::Int(1), AllowitValue::Int(2));
    // The policy source module re-exports the portable rule.
    assert_eq!(
        allowit_core::daily_window_end(None, &one, 0, DAY),
        daily_window_end(None, &one, 0, DAY)
    );
    // The end is inclusive: the window is open at its end and closed one unit later.
    assert!(daily_window_open(5_000, 5_000));
    assert!(!daily_window_open(5_000, 5_001));
    // No value before: the write opens a window of `length` units from the operation's time.
    assert_eq!(
        daily_window_end(None, &one, 1_000, DAY),
        Ok(1_000 + DAY - 1)
    );
    // A read just before the end that takes effect just after it: no renewal.
    assert_eq!(
        daily_window_end(Some((&one, 5_000)), &one, 5_001, DAY),
        Ok(5_000)
    );
    // A read inside the window keeps the end.
    assert_eq!(
        daily_window_end(Some((&one, 5_000)), &one, 4_000, DAY),
        Ok(5_000)
    );
    // A write inside the window, up to its last unit, keeps the end.
    for at in [4_000, 5_000] {
        assert_eq!(
            daily_window_end(Some((&one, 5_000)), &two, at, DAY),
            Ok(5_000)
        );
    }
    // A write read in an open window that takes effect after its end opens the next window.
    assert_eq!(
        daily_window_end(Some((&one, 5_000)), &two, 5_001, DAY),
        Ok(5_001 + DAY - 1)
    );
    // The keeper's clock may count ledgers.
    assert_eq!(daily_window_end(None, &two, 77, 17_280), Ok(77 + 17_279));
    // A one-unit window ends where it opens.
    assert_eq!(daily_window_end(None, &two, 77, 1), Ok(77));
    // An end that the clock cannot state, and a window of no unit, are errors.
    assert_eq!(
        daily_window_end(None, &one, u64::MAX, 2),
        Err(AllowitError::Overflow)
    );
    assert_eq!(daily_window_end(None, &one, u64::MAX, 1), Ok(u64::MAX));
    assert_eq!(
        daily_window_end(None, &one, 0, 0),
        Err(AllowitError::InvalidParam)
    );
}

/// A policy with a daily counter that the contract keeps, a question of the server and the
/// lines of `chain` and `server` at their places.
fn daily_and_question(chain: &str, server: &str) -> String {
    format!(
        "use allowit::prelude::*;\nuse allowit::AllowitDecision;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {{\n    let n: u64 = allowit::get_state(ctx, \"day_n\")?.unwrap_or(0);\n    if n >= 5 {{\n        return Err(AllowitError::DestinationNotAllowed);\n    }}\n    allowit::set_state(ctx, \"day_n\", n + 1)?;\n{chain}    let score = allowit::decision(ctx, &AllowitDecision {{ id: \"a\", query: \"Is this research?\", deny_below: 1, allow_above: 2 }})?;\n{server}    if score < 5000 {{\n        return Err(AllowitError::DestinationNotAllowed);\n    }}\n    Ok(())\n}}\n"
    )
}

/// The refusal of a source in which a daily value that the chain keeps reaches a statement
/// that left: it names the key and the line of that statement.
fn reaches(source: &str, key: &str, line: u32) {
    let error = derive(source).unwrap_err();
    assert!(
        error.contains(&format!("(line {line}) depends on the daily"))
            && error.contains(&format!("{key:?}, which the wallet contract keeps")),
        "{error}"
    );
}

/// The chain keeps a daily value in a window of its own clock, and it does not run a statement
/// that left. A daily counter that only kept statements use stands beside a question: the
/// contract checks the counter again when it executes. A statement that left and reads the
/// value is refused, also when the value comes through a name or a local function.
#[test]
fn a_daily_value_of_the_chain_that_a_leaving_statement_reads_is_refused() {
    let plain = daily_and_question("", "");
    assert_eq!(keys(&plain), [("day_n".to_owned(), Keeper::Chain, true)]);
    // A deterministic statement sets a permit, and a branch on the question requires it.
    let permit = daily_and_question(
        "    if ctx.request.amount < 10 {\n        allowit::set_state(ctx, \"day_permit\", true)?;\n    }\n",
        "    if score < 9000 {\n        let permit: Option<bool> = allowit::get_state(ctx, \"day_permit\")?;\n        assert!(permit.is_some());\n    }\n",
    );
    reaches(&permit, "day_permit", 14);
    // The same read in one statement with the server value.
    let direct = daily_and_question(
        "",
        "    assert!(score > 9000 || allowit::get_state::<u64>(ctx, \"day_n\")?.is_some());\n",
    );
    reaches(&direct, "day_n", 10);
    // The value comes through the name that a kept statement binds.
    let named = daily_and_question("", "    assert!(score > 9000 || n < 3);\n");
    reaches(&named, "day_n", 10);
    // Through a second name. Its `let` only reads, and only a statement that left reads the
    // name, so the `let` leaves too: it is the statement that reads `n`.
    let second = daily_and_question(
        "    let busy = n > 2;\n",
        "    assert!(score > 9000 || !busy);\n",
    );
    reaches(&second, "day_n", 9);
    // Through a name that a statement in a branch on the value assigns. The assignment leaves
    // with the statement that reads the name, and it stands in that branch.
    let assigned = daily_and_question(
        "    let mut busy = false;\n    if n > 2 {\n        busy = true;\n    }\n",
        "    assert!(score > 9000 || !busy);\n",
    );
    reaches(&assigned, "day_n", 11);
    // Through a local function that reads the key, also one that another function calls. The
    // refusal names the statement that calls the function.
    let function = daily_and_question("", "    assert!(score > 9000 || paid(ctx)? < 3);\n")
        .replace(
            "pub fn execute",
            "fn paid(ctx: &AllowitContext) -> Result<u64> {\n    Ok(allowit::get_state(ctx, \"day_n\")?.unwrap_or(0))\n}\npub fn execute",
        );
    reaches(&function, "day_n", 13);
    // A kept statement calls the function too, so it stays.
    let shared = function.replace(
        "    allowit::set_state(ctx, \"day_n\", n + 1)?;\n",
        "    allowit::set_state(ctx, \"day_n\", paid(ctx)? + 1)?;\n",
    );
    reaches(&shared, "day_n", 13);
    let nested = function
        .replace(
            "pub fn execute",
            "fn count(ctx: &AllowitContext) -> Result<u64> {\n    paid(ctx)\n}\npub fn execute",
        )
        .replace("|| paid(ctx)? < 3", "|| count(ctx)? < 3");
    reaches(&nested, "day_n", 16);
    let shared = shared
        .replace(
            "pub fn execute",
            "fn count(ctx: &AllowitContext) -> Result<u64> {\n    paid(ctx)\n}\npub fn execute",
        )
        .replace("|| paid(ctx)? < 3", "|| count(ctx)? < 3");
    reaches(&shared, "day_n", 16);
    // A function that no statement calls reads nothing, also when it stands in a branch that
    // left: the kept write alone gives the chain the key.
    let unused = daily_and_question(
        "",
        "    if score < 9000 {\n        fn unused(ctx: &AllowitContext) -> Result<Option<u64>> {\n            allowit::get_state(ctx, \"day_n\")\n        }\n    }\n",
    );
    assert!(derive(&unused).is_ok());
    // The line is that of the statement, also when the read stands on a later line of it.
    let spread = daily_and_question(
        "",
        "    assert!(\n        score > 9000\n            || allowit::get_state::<u64>(ctx, \"day_n\")?.is_some()\n    );\n",
    );
    reaches(&spread, "day_n", 10);
}

/// A statement that left can depend on a daily value of the chain without reading it: it
/// stands in a branch or a loop on the value, or it comes after an approval that depends on the
/// value. A denial that depends on the value hides no statement: the contract denies again.
#[test]
fn a_leaving_statement_that_a_daily_value_of_the_chain_decides_is_refused() {
    let question = "allowit::decision(ctx, &AllowitDecision { id: \"b\", query: \"Is this seller known?\", deny_below: 1, allow_above: 2 })?";
    // The question is asked only while the counter is low.
    let branch = daily_and_question(
        &format!(
            "    if n > 2 {{\n        let known = {question};\n        assert!(known > 5000);\n    }}\n"
        ),
        "",
    );
    reaches(&branch, "day_n", 10);
    // An approval that depends on the counter skips the question.
    let approved = daily_and_question("    if n == 0 {\n        return Ok(());\n    }\n", "");
    reaches(&approved, "day_n", 12);
    // A loop that the counter ends decides what a later statement reads. The statement of the
    // loop that assigns the name leaves with the statement that reads it.
    let looped = daily_and_question(
        "    let mut rounds = 0u64;\n    for _ in 0..3u64 {\n        if n > 1 {\n            break;\n        }\n        rounds += 1;\n    }\n",
        "    assert!(score > 9000 || rounds > 0);\n",
    );
    reaches(&looped, "day_n", 14);
    // A denial on the counter stands before the question in the plain policy: it is admitted.
    assert!(derive(&daily_and_question("", "")).is_ok());
    // The rule resolves no label and no target of an exit: a `break` on the counter counts for
    // every loop around it, whatever its label and however the label is spelled.
    let inner = format!(
        "    let review = 2u64;\n    'review: for _ in 0..review {{\n        for _ in 0..3u64 {{\n            if n > 2 {{\n                break;\n            }}\n        }}\n        let known = {question};\n        assert!(known > 5000);\n    }}\n"
    );
    reaches(&daily_and_question(&inner, ""), "day_n", 16);
    for exit in ["break 'review;", "break 'r#review;", "continue 'r#review;"] {
        let labeled = inner.replace(
            "                break;\n",
            &format!("                {exit}\n"),
        );
        reaches(&daily_and_question(&labeled, ""), "day_n", 16);
    }
}

/// The rule holds only for a daily value that the chain keeps. The server keeps a key whose
/// write leaves and a key that only leaving statements read, and it holds an operation inside
/// their window. A value that is not daily has no window. A standard daily figure follows the
/// rule when the wallet keeps it and a statement that left reads it with `get_state`; its
/// canonical rule needs no bound, because the figure only grows inside its window.
#[test]
fn a_daily_value_that_the_server_keeps_and_a_lasting_value_reach_a_leaving_statement() {
    // The write depends on the question: the server keeps the key, and its reader leaves.
    let server = daily_and_question(
        "",
        "    let low: u64 = allowit::get_state(ctx, \"day_low\")?.unwrap_or(0);\n    if score < 9000 {\n        allowit::set_state(ctx, \"day_low\", low + 1)?;\n    }\n",
    );
    assert!(keys(&server).contains(&("day_low".to_owned(), Keeper::Server, true)));
    // No statement writes the key, and only a statement that left reads it.
    let unwritten = daily_and_question(
        "",
        "    assert!(score > 9000 || allowit::get_state::<u64>(ctx, \"day_seen\")?.is_none());\n",
    );
    assert!(keys(&unwritten).contains(&("day_seen".to_owned(), Keeper::Server, true)));
    // A lasting value of the chain has no window.
    let lasting = daily_and_question(
        "    let paid: u64 = allowit::get_state(ctx, \"paid\")?.unwrap_or(0);\n    allowit::set_state(ctx, \"paid\", paid + 1)?;\n",
        "    assert!(score > 9000 || paid < 3);\n",
    );
    assert!(keys(&lasting).contains(&("paid".to_owned(), Keeper::Chain, false)));
    // A daily figure: the wallet keeps it for the kept rule, and a statement that left reads it.
    let rule =
        "    allowit::daily_spending_limit(ctx, &ctx.params[allowit::DAILY_SPENDING_LIMIT])?;\n";
    let read =
        "    assert!(score > 9000 || allowit::get_state::<i128>(ctx, \"day_spent\")?.is_none());\n";
    let error = derive(&daily_and_question(rule, read)).unwrap_err();
    assert!(
        error.contains("(line 11) depends on the daily figure \"day_spent\"")
            && error.contains("canonical rule"),
        "{error}"
    );
    // The wallet keeps no figure that only a statement that left reads: the server gives it.
    let derived = derive(&daily_and_question("", read)).unwrap();
    assert!(derived.figures.is_empty());
    // The canonical rule in a branch on the question, beside the same rule on chain.
    let ruled = daily_and_question(
        rule,
        "    if score < 9000 {\n        allowit::daily_spending_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n    }\n",
    );
    let derived = derive(&ruled).unwrap();
    assert_eq!(derived.server_only.len(), 2);
    assert!(
        derived
            .figures
            .iter()
            .any(|figure| figure.key == "day_spent")
    );
    // The amount that the kept rule leaves is a value of the figure.
    let left = daily_and_question(
        "    let left = allowit::daily_spending_limit(ctx, &ctx.params[allowit::DAILY_SPENDING_LIMIT])?;\n",
        "    assert!(score > 9000 || left > 100);\n",
    );
    let error = derive(&left).unwrap_err();
    assert!(
        error.contains("(line 11) depends on the daily figure \"day_spent\""),
        "{error}"
    );
    // The same amount through a local function that a kept statement calls.
    let helper = left
        .replace(
            "    let left = allowit::daily_spending_limit(ctx, &ctx.params[allowit::DAILY_SPENDING_LIMIT])?;\n",
            "    let left: i128 = headroom(ctx)?;\n",
        )
        .replace(
            "pub fn execute",
            "fn headroom(ctx: &AllowitContext) -> Result<i128> {\n    allowit::daily_spending_limit(ctx, &ctx.params[allowit::DAILY_SPENDING_LIMIT])\n}\npub fn execute",
        );
    let error = derive(&helper).unwrap_err();
    assert!(
        error.contains("(line 14) depends on the daily figure \"day_spent\""),
        "{error}"
    );
    // A rule that only the server runs and whose value its statement uses, beside the kept rule.
    let consumed = "    if score < 9000 {\n        let left: i128 = allowit::daily_spending_limit(ctx, &ctx.params[allowit::DAILY_SPENDING_LIMIT])?;\n        assert!(left < 100);\n    }\n";
    let error = derive(&daily_and_question(rule, consumed)).unwrap_err();
    assert!(
        error.contains("(line 12) depends on the daily figure \"day_spent\""),
        "{error}"
    );
    // Without a kept rule or read the wallet keeps no figure: the server gives it.
    let derived = derive(&daily_and_question("", consumed)).unwrap();
    assert!(derived.figures.is_empty());
    // A rule in a local function denies only that function: its caller can catch the denial.
    let caught = daily_and_question(rule, "    assert!(score > 9000 || under_cap(ctx).is_err());\n")
        .replace(
            "pub fn execute",
            "fn under_cap(ctx: &AllowitContext) -> allowit::Result {\n    allowit::daily_spending_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n    Ok(())\n}\npub fn execute",
        );
    let error = derive(&caught).unwrap_err();
    assert!(
        error.contains("depends on the daily figure \"day_spent\""),
        "{error}"
    );
    // A rule that only denies `execute` is admitted behind an attribute too.
    let attributed = daily_and_question(
        rule,
        "    if score < 9000 {\n        #[allow(unused_must_use)]\n        allowit::daily_spending_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n    }\n",
    );
    assert_eq!(derive(&attributed).unwrap().server_only.len(), 2);
    // In a branch that is not split, because its value can be used, the derivation does not read
    // the statements one by one: there the rule counts as one whose value can be used.
    let whole = daily_and_question(
        rule,
        "    if score < 9000 {\n        allowit::daily_spending_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n    } else {\n        allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n    };\n",
    );
    assert!(derive(&whole).is_err());
    // A closure is no statement of `execute`: its `?` ends the closure, and the statement
    // that calls it can catch the denial. The source gate admits no closure; the rule does not
    // rely on that.
    let closure = daily_and_question(
        rule,
        "    let under = || -> allowit::Result {\n        allowit::daily_spending_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n        Ok(())\n    };\n    assert!(score > 9000 || under().is_err());\n",
    );
    let error = derive(&closure).unwrap_err();
    assert!(
        error.contains("depends on the daily figure \"day_spent\""),
        "{error}"
    );
}
