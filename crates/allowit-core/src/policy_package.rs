//! Typed policy packages. A package is one crate that declares evidence functions for server
//! policies under one namespace. The namespace is the package's crate name, so policy source
//! calls `<crate>::<function>(ctx, &value)`. The SDK registers each package with one line and
//! uses the same declaration for discovery, admission, dispatch and the shared package tests.
//!
//! This module names no package. It needs only `std`, so a package crate depends on nothing
//! but the portable library and JSON values.
use alloc::{format, string::String, vec::Vec};
use serde_json::Value;

/// An evidence function: guest arguments and host-supplied facts in, a value or a reason out.
/// The host authenticates the facts. The function never trusts the arguments as facts.
pub type PolicyFunction = fn(args: &Value, facts: &Value) -> Result<Value, String>;

/// Namespaces that no package may claim. The registry also reserves every rail id.
pub const RESERVED_NAMESPACES: [&str; 4] = ["allowit", "core", "std", "alloc"];
/// Most host calls that one registry may admit, as in the policy-source gate.
pub const MAX_HOST_CALLS: usize = 128;

/// Names that the policy-source gate refuses in a host call registration: Rust keywords and
/// the primitive type names. `policy_source` tests keep this list equal to the gate's lists.
const GATE_RESERVED: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "union", "unsafe", "unsized", "use", "virtual", "where", "while", "yield", "i8", "i16", "i32",
    "i64", "i128", "u8", "u16", "u32", "u64", "u128", "bool", "char", "str", "usize", "isize",
    "f16", "f32", "f64", "f128",
];

/// What the shared looped tests run on each package. A package lists the features it skips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PackageFeature {
    /// A function answers from facts that the host authenticated.
    AuthenticatedFacts,
    /// A function reaches the provider's live service.
    LiveCall,
}

/// One call of the shared package tests, with the reply that the package must give.
#[derive(Clone, Debug, PartialEq)]
pub struct PackageSample {
    pub function: &'static str,
    pub feature: PackageFeature,
    pub args: Value,
    pub facts: Value,
    /// The exact reply, or `None` when the package must refuse the call.
    pub reply: Option<Value>,
}

/// One registered policy package.
pub struct PolicyPackage {
    /// The crate name: `env!("CARGO_CRATE_NAME")` of the package's library.
    pub namespace: &'static str,
    /// The crate release: `env!("CARGO_PKG_VERSION")`. The build identity of a server policy
    /// binds it together with the locked source revision.
    pub release: &'static str,
    /// Evidence functions by name.
    pub functions: &'static [(&'static str, PolicyFunction)],
    /// Features that this package does not support, each with the reason.
    pub skips: &'static [(PackageFeature, &'static str)],
    /// Calls for the shared looped tests. Shared tests never assert a package literal.
    pub samples: fn() -> Vec<PackageSample>,
}

impl PolicyPackage {
    /// The function named `name`.
    pub fn function(&self, name: &str) -> Option<PolicyFunction> {
        self.functions
            .iter()
            .find(|(function, _)| *function == name)
            .map(|(_, function)| *function)
    }
    /// The reason this package skips `feature`, if it does.
    pub fn skip(&self, feature: PackageFeature) -> Option<&'static str> {
        self.skips
            .iter()
            .find(|(skipped, _)| *skipped == feature)
            .map(|(_, reason)| *reason)
    }
}

/// A lowercase identifier that the policy-source gate admits in a host call registration.
fn identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    value.len() <= 64
        && !GATE_RESERVED.contains(&value)
        && bytes.next().is_some_and(|b| b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Admit `packages` and return their exact host calls as `(namespace, function)` pairs, in
/// registration order. `rails` are the registered rail ids. A namespace must be a lowercase
/// identifier that is unique, not reserved and not a rail id. A package must name its release
/// and declare at least one function, each a unique lowercase identifier other than `allowit`.
/// No name is a Rust keyword or a primitive type, and the packages declare at most
/// [`MAX_HOST_CALLS`] calls.
///
/// This is the structural admission of the packages alone. The registry then checks the
/// complete list, with the built-in server calls and the rail evidence functions, with
/// `policy_source::check_host_call_registry`, which also refuses a name that policy source
/// cannot call, such as `ctx` or `execute`.
pub fn host_calls(
    packages: &[&'static PolicyPackage],
    rails: &[&str],
) -> Result<Vec<(&'static str, &'static str)>, String> {
    let mut calls: Vec<(&'static str, &'static str)> = Vec::new();
    for (index, package) in packages.iter().enumerate() {
        let namespace = package.namespace;
        if !identifier(namespace) {
            return Err(format!("Invalid policy package namespace `{namespace}`"));
        }
        if RESERVED_NAMESPACES.contains(&namespace) || rails.contains(&namespace) {
            return Err(format!(
                "The policy package namespace `{namespace}` is reserved"
            ));
        }
        if packages[..index].iter().any(|p| p.namespace == namespace) {
            return Err(format!("Duplicate policy package namespace `{namespace}`"));
        }
        let release = package.release;
        if release.is_empty()
            || release.len() > 64
            || !release.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err(format!("The policy package `{namespace}` names no release"));
        }
        if package.functions.is_empty() {
            return Err(format!(
                "The policy package `{namespace}` declares no function"
            ));
        }
        for (name, _) in package.functions {
            if !identifier(name) || *name == "allowit" || calls.contains(&(namespace, *name)) {
                return Err(format!(
                    "Invalid or duplicate policy function `{namespace}::{name}`"
                ));
            }
            calls.push((namespace, *name));
        }
    }
    if calls.len() > MAX_HOST_CALLS {
        return Err(format!(
            "The policy packages declare more than {MAX_HOST_CALLS} functions"
        ));
    }
    Ok(calls)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn echo(args: &Value, _: &Value) -> Result<Value, String> {
        Ok(args.clone())
    }
    fn none() -> Vec<PackageSample> {
        vec![]
    }
    const fn package(
        namespace: &'static str,
        functions: &'static [(&'static str, PolicyFunction)],
    ) -> PolicyPackage {
        PolicyPackage {
            namespace,
            release: "1.2.3",
            functions,
            skips: &[(PackageFeature::LiveCall, "fixture only")],
            samples: none,
        }
    }
    static QUOTES: PolicyPackage = package("quotes", &[("check", echo), ("settled", echo)]);
    static OTHER: PolicyPackage = package("other_1", &[("check", echo)]);

    #[test]
    fn registered_packages_yield_their_exact_host_calls() {
        assert_eq!(
            host_calls(&[&QUOTES, &OTHER], &["rail"]).unwrap(),
            [
                ("quotes", "check"),
                ("quotes", "settled"),
                ("other_1", "check")
            ]
        );
        assert!(host_calls(&[], &[]).unwrap().is_empty());
        assert!(QUOTES.function("settled").is_some());
        assert!(QUOTES.function("price").is_none());
        assert_eq!(QUOTES.skip(PackageFeature::LiveCall), Some("fixture only"));
        assert_eq!(QUOTES.skip(PackageFeature::AuthenticatedFacts), None);
    }

    #[test]
    fn reserved_duplicate_and_malformed_registrations_are_refused() {
        static CORE: PolicyPackage = package("allowit", &[("check", echo)]);
        static STD: PolicyPackage = package("std", &[("check", echo)]);
        static UPPER: PolicyPackage = package("Quotes", &[("check", echo)]);
        static DASHED: PolicyPackage = package("quote-s", &[("check", echo)]);
        static EMPTY: PolicyPackage = package("empty", &[]);
        static REPEATED: PolicyPackage = package("twice", &[("check", echo), ("check", echo)]);
        static PATHED: PolicyPackage = package("pathed", &[("a::b", echo)]);
        static KEYWORD: PolicyPackage = package("match", &[("check", echo)]);
        static PRIMITIVE: PolicyPackage = package("u64", &[("check", echo)]);
        static KEYWORD_FUNCTION: PolicyPackage = package("typed", &[("loop", echo)]);
        static CORE_FUNCTION: PolicyPackage = package("claims", &[("allowit", echo)]);
        static UNRELEASED: PolicyPackage = PolicyPackage {
            release: "",
            ..package("unreleased", &[("check", echo)])
        };
        for refused in [
            &CORE,
            &STD,
            &UPPER,
            &DASHED,
            &EMPTY,
            &REPEATED,
            &PATHED,
            &KEYWORD,
            &PRIMITIVE,
            &KEYWORD_FUNCTION,
            &CORE_FUNCTION,
            &UNRELEASED,
        ] {
            assert!(
                host_calls(&[refused], &[]).is_err(),
                "{}",
                refused.namespace
            );
        }
        // A rail id is reserved for its rail, and one namespace registers once.
        assert!(
            host_calls(&[&QUOTES], &["quotes"])
                .unwrap_err()
                .contains("reserved")
        );
        assert!(
            host_calls(&[&QUOTES, &QUOTES], &[])
                .unwrap_err()
                .contains("Duplicate")
        );
    }

    #[test]
    fn a_registry_admits_at_most_the_gate_limit() {
        fn many(count: usize) -> &'static PolicyPackage {
            let functions: Vec<(&'static str, PolicyFunction)> = (0..count)
                .map(|n| (&*format!("op{n}").leak(), echo as PolicyFunction))
                .collect();
            Box::leak(Box::new(package("many", functions.leak())))
        }
        assert_eq!(
            host_calls(&[many(MAX_HOST_CALLS)], &[]).unwrap().len(),
            MAX_HOST_CALLS
        );
        assert!(host_calls(&[many(MAX_HOST_CALLS + 1)], &[]).is_err());
    }

    /// The complete registry check accepts admitted packages with the built-in calls, calls
    /// each function, and refuses names that policy source cannot use and a registry over
    /// the limit. The gate refuses each name that this module reserves.
    #[cfg(feature = "policy-source")]
    #[test]
    fn admitted_host_calls_pass_the_policy_source_gate() {
        use crate::policy_source::{check_host_call_registry, source_shape_with_host_calls};
        let mut registry = host_calls(&[&QUOTES, &OTHER], &["rail"]).unwrap();
        registry.push(("allowit", "price"));
        check_host_call_registry(&registry).unwrap();
        static CONTEXT: PolicyPackage = package("ctx", &[("check", echo)]);
        static ENTRY: PolicyPackage = package("execute", &[("check", echo)]);
        static FIELD: PolicyPackage = package("fields", &[("env", echo)]);
        for unusable in [&CONTEXT, &ENTRY, &FIELD] {
            let admitted = host_calls(&[unusable], &[]).unwrap();
            assert!(
                check_host_call_registry(&admitted).is_err(),
                "{}",
                unusable.namespace
            );
        }
        // The limit covers the whole registry: full packages leave no room for a built-in.
        let mut full: Vec<(&str, &str)> = (0..MAX_HOST_CALLS)
            .map(|n| ("many", &*format!("op{n}").leak()))
            .collect();
        check_host_call_registry(&full).unwrap();
        full.push(("allowit", "price"));
        assert!(check_host_call_registry(&full).is_err());
        const SOURCE: &str = "use allowit::prelude::*;\npub fn execute(ctx: &AllowitContext) -> allowit::Result {\n    allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;\n    Ok(())\n}\n";
        let mut calls = host_calls(&[&QUOTES, &OTHER], &["rail"]).unwrap();
        calls.push(("allowit", "price"));
        source_shape_with_host_calls(SOURCE, &calls).unwrap();
        for name in GATE_RESERVED.iter().chain(&RESERVED_NAMESPACES[1..]) {
            assert!(
                source_shape_with_host_calls(SOURCE, &[(name, "check")]).is_err(),
                "the gate admits the namespace `{name}`"
            );
        }
        assert!(source_shape_with_host_calls(SOURCE, &[("quotes", "allowit")]).is_err());
        assert!(source_shape_with_host_calls(SOURCE, &[("allowit", "check")]).is_err());
    }
}
