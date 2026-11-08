#!/usr/bin/env python3
"""Maintain the fixed corpus shared by the Rust and Python portable source gates.

Run with --check to compare the corpus, or without arguments to regenerate it."""
import json
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
GATE_FILE = ROOT / "crates/allowit-core/tests/fixtures/policy-source-vectors.json"
CANONICAL = (ROOT / "crates/allowit-core/tests/fixtures/portable-policy.rs").read_text()
BODY = "pub fn execute(ctx: &AllowitContext) -> allowit::Result {"

# Top-level items appended to the canonical source. Each exercises one allowlist rule; most of them
# reject. The two from the review are first.
ITEMS = [
    "use std::fs;",
    "fn helper(a: &[i128]) -> bool { a.len() > 0 }",
    "use allowit::AllowitError;",
    "use allowit::{AllowitError, AllowitValue};",
    "use allowit::{prelude::*, AllowitError};",
    "use allowit::{payment_limit as Ok};",
    "use allowit::Bogus;",
    "const LIMIT: i128 = 5;",
    "const LIMIT: usize = 5;",
    "const LIMIT: f64 = 5.0;",
    "const LIMIT: i128 = 5i128;\nfn helper() -> i128 { LIMIT }",
    "fn helper(a: i128) -> i128 { a.checked_add(1i128).unwrap_or(0i128) }",
    "fn helper(v: Vec<i128>) -> i128 { v[0] }",
    "fn helper() -> i128 { let f = |x: i128| x; f(1i128) }",
    "fn helper() -> fn() -> i128 { helper2 }\nfn helper2() -> i128 { 1i128 }",
    "fn helper() -> i128 { helper2() }\nfn helper2() -> i128 { 1i128 }",
    "fn helper(a: i128) -> i128 { a as i128 }",
    "fn helper() -> i128 { core::mem::size_of::<i128>() }",
    "fn helper() -> i128 { size_of::<u8>() }",
    "fn helper() -> i128 { i128::MAX }",
    "fn helper() -> i128 { i128::from(1u8) }",
    "fn helper() -> i128 { i128::try_from(1u8).unwrap() }",
    "fn helper() -> i128 { let g = i128::from; g(1u8) }",
    "fn helper() -> Option<i128> { Some(1i128) }",
    "fn helper() -> Option<Option<i128>> { Some(Some) }",
    "fn helper() -> Option<i128> { Option::Some(1i128) }",
    "struct S;",
    "impl S {}",
    "mod m {}",
    "static X: i128 = 1i128;",
    "unsafe fn helper() {}",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.params.0.is_some() }",
    "fn helper(ctx: &AllowitContext) -> i128 { ctx.params\n. 1 }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.state.is_some() }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.request.amount > 0i128 }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.request.spent_units > ctx.request.gas_units }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.params.contains(allowit::PAYMENT_LIMIT) }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.request.args.contains(\"x\") }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.request.contains(\"x\") }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.params.get(allowit::TOKENS).is_some() }",
    "fn helper(ctx: &AllowitContext) -> AllowitValue { ctx.params[allowit::TOKENS].clone() }",
    "fn helper(ctx: &AllowitContext) -> AllowitValue { ctx.params[\"vendor_payment_limit\"].clone() }",
    "fn helper(ctx: &AllowitContext) -> AllowitValue { ctx . params [ allowit::TOKENS ].clone() }",
    "fn helper(ctx: &AllowitContext) -> AllowitValue { ctx.request.args[\"x\"].clone() }",
    "fn helper(ctx: &AllowitContext) -> AllowitValue { (ctx.params)[\"x\"].clone() }",
    "fn helper(ctx: &AllowitContext) -> bool { matches!(&ctx.params[\"x\"], AllowitValue::Int(_)) }",
    "fn helper(params: i128) -> i128 { params }",
    "fn args() -> i128 { 1i128 }",
    "fn helper(ctx: &AllowitContext) -> AllowitContext { AllowitContext { ..*ctx } }",
    "fn helper(ctx: &AllowitContext) -> i128 { let AllowitContext { env, .. } = ctx; 0i128 }",
    "fn helper(ctx: &'static mut AllowitContext) -> &'static mut AllowitContext { ctx }",
    "fn helper(x: Option<i128>) -> i128 { match x { Some(v) if v > 0i128 => v, Some(w) => w, None => 0i128 } }",
    "fn helper(x: i128) -> i128 { match x { 1i128 | 2i128 => 0i128, _ => 1i128 } }",
    "fn helper() { panic!(\"bad {}\", 1i128) }",
    "fn helper() { panic!(\"bad {:?}\", 1i128) }",
    "fn helper() { panic!(\"bad {:5}\", 1i128) }",
    "fn helper() { panic!(\"bad {0}\", 1i128) }",
    "fn helper() { panic!(\"{{x}}\") }",
    "fn helper() { panic!(\"\\u{7b}:1$}\", 1i128) }",
    "fn helper() { panic!(\"\\x7b:.*}\", 1i128) }",
    "fn helper() { panic!(\"line \\\n   {}\", 1i128) }",
    "fn helper() { panic!(r\"{:1$}\") }",
    "fn helper() { panic!(r#\"{\"#) }",
    "fn helper() { panic!(\"\\u{110000}\") }",
    "fn helper() { panic!(\"\\u{__}\") }",
    "fn helper() { panic!(\"\\u{d800}\") }",
    "fn helper() { panic!(\"\\u{7_b}\") }",
    "fn helper() { panic!(\"\\u{000000007b}\") }",
    "fn helper() { panic!(0) }",
    "fn helper(a: bool) { assert!(a, \"oops {:?}\", a); }",
    "fn helper(a: bool) { assert!(a, concat!(\"x\")); }",
    "fn helper(a: bool) { assert!(a,); }",
    "fn helper(a: i128) { assert_eq!(a, 1i128, \"{:x}\", a); }",
    "fn helper(a: i128) { assert_ne!(a, 1i128, \"{}\", a); }",
    "fn helper() { unreachable!(\"x{}\", 1i128) }",
    "fn helper() { unreachable!(\"x{\") }",
    "fn helper() -> i128 { 1 }",
    "fn helper() -> i128 { 1.5 }",
    "fn helper() -> i128 { 1.max(2) }",
    "fn helper() -> i128 { 0xffi128 }",
    "fn helper() -> i128 { 0b101i128 }",
    "fn helper() -> i128 { 0o17u8 }",
    "fn helper() -> i128 { 0b2 }",
    "fn helper() -> i128 { 1_000i128 }",
    "fn helper() -> i128 { 1e3 }",
    "fn helper() -> i128 { 1usize }",
    "fn helper() -> i128 { let x = [1i128, 2i128]; 0i128 }",
    "fn helper() -> i128 { let r#loop = 1i128; r#loop }",
    "fn r#helper() -> i128 { 1i128 }",
    "fn helper(a: &mut i128) { *a = 1i128; }",
    "fn helper() -> String { String::new() }",
    "fn helper() -> u8 { b'a' }",
    "fn helper() -> char { '\\u{7b}' }",
    "fn helper(ctx: &AllowitContext) -> i128 { ctx.env.clone(); 0i128 }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.env == ctx.env }",
    "fn helper(ctx: &mut AllowitContext) { ctx.env = ctx.env; }",
    "fn helper() -> i128 { loop { break 1i128; } }",
    "fn helper() -> i128 { 'a: loop { break 'a 1i128; } }",
    "fn helper(x: i128) -> core::cmp::Ordering { x.cmp(&0i128) }",
    "fn helper() -> i128 { core::cmp::max(1i128, 2i128) }",
    "fn helper() -> i128 { let m = core::cmp::max; m(1i128, 2i128) }",
    "fn helper(v: Vec<i128>) -> i128 { for x in v { return x; } 0i128 }",
    "fn helper(v: Vec<i128>) -> i128 { v.iter().count() }",
    "fn helper(ctx: &AllowitContext) -> allowit::Result { allowit::set_state(ctx, \"k\", AllowitValue::Int(1i128)) }",
    "fn helper(ctx: &AllowitContext) -> allowit::Result { allowit::r#set_state(ctx, \"k\", AllowitValue::Int(1i128)) }",
    "fn helper(ctx: &AllowitContext) -> Option<AllowitValue> { allowit::get_state(ctx, \"k\") }",
    "fn helper() -> allowit::Result { Err(allowit::AllowitError::Unsupported) }",
    "fn helper() -> allowit::Result { Err(AllowitError::Bogus) }",
    "fn helper() -> AllowitRule { AllowitRule::expires_at }",
    "fn helper() -> AllowitValue { allowit::AllowitValue::Bool(true) }",
    "fn helper() -> fn(bool) -> AllowitValue { AllowitValue::Bool }",
    "#[cfg(test)] fn helper() {}",
    "#[inline] fn helper() {}",
    "#[must_use] fn helper() -> bool { true }",
    "#[path = \"x.rs\"] fn helper() {}",
    "fn helper() { asm!(\"\"); }",
    "fn helper() -> &'static str { include_str!(\"x\") }",
    "fn helper() -> &'static str { env!(\"HOME\") }",
    "fn helper() -> i128 { let x: Box<i128> = 1i128; 0i128 }",
    "fn helper() -> i128 { self::helper2() }\nfn helper2() -> i128 { 1i128 }",
    "fn helper(x: &i128) -> *const i128 { x }",
    "fn helper() -> i128 { let x = 1i128; x.pow(2u32) }",
    "fn helper() -> Option<i128> { Some(1i128).map(|x| x) }",
    "fn helper() -> bool { true || false }",
    "fn helper(a: bool) -> bool { a | false }",
    "fn helper(a: bool) -> bool { a || !a }",
    "fn helper() { let _ = drop; }",
    "fn hélper() {}",
    "// comment with é\nfn helper() {}",
    "fn helper() -> &'static str { \"é {x}\" }",
    "fn helper() -> &'static str { r#\"raw \"{\"# }",
    "fn Ok(_: ()) -> allowit::Result { Err(allowit::AllowitError::MissingParam) }",
    "fn helper(x: (i128, i128)) -> i128 { x.0 }",
    "fn helper<T>(x: T) -> T { x }",
    "fn helper() -> i128 { <i128>::MAX }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.request.operation == AllowitOperation::transfer }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.request.operation == allowit::AllowitOperation::redeem }",
    "fn helper(ctx: &AllowitContext) -> bool { matches!(ctx.request.operation, AllowitOperation::custom(..)) }",
    "fn helper(ctx: &AllowitContext) -> bool { let f = AllowitOperation::custom; true }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.request.amount_in > 0i128 && ctx.request.token_out.is_some() }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.request.token_in == ctx.request.asset && ctx.request.min_out == 0i128 }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.request.operation == allowit::AllowitOperation::sep99 }",
    "fn helper() -> AllowitPrice { AllowitPrice { from: 1i128, to: 2i128 } }",
    "fn helper(x: i128) -> i128 { let y = x; y }",
    "fn helper(x: i128) -> i128 { z }",
    "fn helper(ctx: &AllowitContext) -> i128 { ctx.foo }",
    "fn helper(x: Option<i128>) -> bool { x.contains(0i128) }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx.params .contains(allowit::TOKENS) }",
    "fn helper(ctx: &AllowitContext) -> bool { ctx . params . contains(allowit::TOKENS) }",
    "fn helper() -> i128 { $x }",
    "fn helper(a: i128) -> bool { a!=1i128 }",
    "fn helper(a: i128) -> i128 { -a }",
    "fn helper(a: [i128; 2]) -> i128 { 0i128 }",
    "fn helper() -> i128 { ::core::cmp::max(1i128, 2i128) }",
    "fn helper() -> i128 { crate::helper() }",
    "fn helper() -> u64 { allowit::DAY }",
    "fn helper() -> Address { Address::from(1i128) }",
    "fn helper(a: Address) -> Address { a.clone() }",
    "fn helper(a: AllowitValue) -> bool { matches!(a, AllowitValue::Int(_)) }",
    "fn helper(a: AllowitValue) -> bool { match a { AllowitValue::List(items) => items.is_empty(), _ => false } }",
    "fn helper(a: AllowitValue) -> bool { if let AllowitValue::Int(v) = a { v > 0i128 } else { false } }",
    "fn helper(a: i128) -> i128 { let (b, c) = (a, a); b + c }",
    "fn helper(a: i128) -> i128 { while a > 0i128 { return a; } 0i128 }",
    "fn helper(a: i128) -> i128 { a.saturating_sub(1i128).abs() }",
    "fn helper(a: i128) -> i128 { a.wrapping_add(1i128) }",
    "fn helper(a: i128) -> i128 { let ref b = a; *b }",
    "fn helper() -> i128 { let mut a = 0i128; a += 1i128; a }",
    "fn helper() -> i128 { const fn_in: i128 = 1i128; fn_in }",
    "fn helper() -> i128 {\n    // nested /* comment */\n    1i128\n}",
    "/* outer /* nested */ still comment */ fn helper() -> i128 { 1i128 }",
    "fn helper() -> i128 { 1i128 } /* unterminated",
    "fn helper() -> &'static str { \"unterminated }",
    "//! inner doc",
    "#![allow(dead_code)]",
    "fn helper() -> bool { cfg_attr }",
    "fn helper(x: &i128) -> i128 { let p = &raw const x; 0i128 }",
    "macro_rules! m { () => {} }",
    "fn helper() -> i128 { 1i128 }\nfn execute() {}",
    "fn helper() -> i128 { 1i128 }\x1cfn other() {}",
    "fn helper() -> i128 { async { 1i128 } }",
    "fn helper(x: i128) -> i128 { x?; x }",
    "fn helper(x: Option<i128>) -> Option<i128> { let y = x?; Some(y) }",
    "fn helper(x: Option<i128>) -> i128 { x.unwrap_or_default() }",
    "fn helper(x: Option<i128>) -> Result<i128, AllowitError> { x.ok_or(AllowitError::MissingParam) }",
    "fn helper(x: i128) -> i128 { (x)[0] }",
    "fn helper() -> i128 { 1i128 }\nfn other() -> i128 { helper }",
]

# Statements inserted at the start of `execute` or before its final `Ok(())`.
STATEMENTS = [
    "let x = 1i128;",
    "allowit::set_state(ctx, \"k\", AllowitValue::Int(1i128))?;",
    "allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;",
    "allowit::lifetime_spending_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT])?;",
    "allowit::payment_limit(ctx, &ctx.params[allowit::PAYMENT_LIMIT],)?;",
    "allowit::payment_limit(ctx, ctx.params[allowit::PAYMENT_LIMIT])?;",
    "if ctx.params.contains(\"vendor_payment_limit\") { allowit::payment_limit(ctx, &ctx.params[\"vendor_payment_limit\"])?; }",
    "if ctx.request.amount > 5i128 { return Err(AllowitError::PaymentLimitExceeded); }",
    "assert!(ctx.request.amount >= 0i128);",
    "let n = ctx.params[\"n\"].len();",
    "let _ = ctx.request.now;",
    "let _ = ctx.request.expires_at.checked_sub(ctx.request.now);",
]


def gate(source):
    import policy_source
    try:
        prefix = policy_source.standard_prefix(source)
        return {"accepted": True, "prefix": prefix, "writes_state": policy_source.writes_state(source)}
    except Exception as error:  # Any failure of the gate is a rejection.
        return {"accepted": False, "error": type(error).__name__ + ": " + str(error)}


def gate_sources():
    lines = CANONICAL.splitlines(keepends=True)
    rules = [k for k, line in enumerate(lines) if line.strip().startswith("allowit::")]
    sources = [CANONICAL]
    sources += [CANONICAL + "\n" + item + "\n" for item in ITEMS]
    for statement in STATEMENTS:
        sources.append(CANONICAL.replace(BODY, BODY + "\n    " + statement, 1))
        sources.append(CANONICAL.replace("    Ok(())\n}", "    " + statement + "\n    Ok(())\n}", 1))
    # Drop, swap or stop each prefix rule, so the derived prefix changes.
    for k in rules:
        sources.append("".join(lines[:k] + lines[k + 1:]))
        sources.append("".join(lines[:k] + ["    let _ = 0i128;\n"] + lines[k:]))
    for a, b in zip(rules, rules[1:]):
        swapped = list(lines)
        swapped[a], swapped[b] = swapped[b], swapped[a]
        sources.append("".join(swapped))
    # Seeded random combinations: one to three items and statements at random places.
    rng = random.Random(20261031)
    for _ in range(240):
        source = CANONICAL
        for _ in range(rng.randint(1, 3)):
            if rng.random() < 0.6:
                source += "\n" + rng.choice(ITEMS) + "\n"
            else:
                anchor = rng.choice([BODY, "    Ok(())\n}"])
                statement = rng.choice(STATEMENTS)
                source = source.replace(anchor, (anchor + "\n    " + statement) if anchor == BODY
                                        else ("    " + statement + "\n" + anchor), 1)
        sources.append(source)
    return list(dict.fromkeys(sources))


def edit(source):
    """The edit of the ASCII canonical source that gives `source`."""
    head = 0
    while head < min(len(source), len(CANONICAL)) and source[head] == CANONICAL[head]:
        head += 1
    tail = 0
    while tail < min(len(source), len(CANONICAL)) - head and source[-1 - tail] == CANONICAL[-1 - tail]:
        tail += 1
    return {"head": head, "middle": source[head:len(source) - tail], "tail": tail}


def gate_vectors():
    assert CANONICAL.isascii()
    return [dict(edit(source), **gate(source)) for source in gate_sources()]



def dump(vectors):
    """One vector per line preserves readable diffs."""
    return "[\n" + ",\n".join(json.dumps(v, ensure_ascii=False) for v in vectors) + "\n]\n"


def main():
    if sys.argv[1:] not in ([], ["--check"]):
        raise SystemExit("usage: policy-source-vectors.py [--check]")
    text = dump(gate_vectors())
    if sys.argv[1:] == ["--check"]:
        if GATE_FILE.read_text() != text:
            raise SystemExit("Stale source vectors. Run scripts/policy-source-vectors.py")
    else:
        GATE_FILE.write_text(text)


if __name__ == "__main__":
    main()
