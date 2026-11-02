//! Structural source checks for portable authored policies.
//!
//! These checks do not compile, publish or admit a policy. The host build gate must also
//! compile admitted source against the same released portable library.

/// Exact portable sources and terms from the selected dependency release.
pub use allowit::{
    SOURCE_CORE_RS as PORTABLE_CORE_RS, SOURCE_LIB_RS as PORTABLE_LIB_RS,
    SOURCE_TERMS as PORTABLE_TERMS,
};

/// The Python gate and host-compile helper from this package's released inputs.
pub const PYTHON_HELPER: &str = include_str!("../scripts/policy_source.py");
/// The original helper terms. Embedding the helper grants no additional license.
pub const PYTHON_HELPER_TERMS: &str = include_str!("../scripts/LICENSE");

type Result<T> = std::result::Result<T, String>;

fn param_keys() -> [&'static str; allowit::RULES.len()] {
    allowit::RULES.map(allowit::AllowitRule::key)
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// The checked shape of authored source: its standard prefix and whether it writes state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceShape {
    /// Leading canonical prefix statements of `execute`.
    pub prefix: Vec<String>,
    /// The file is only `use allowit::...;` items and an `execute` that is exactly the prefix
    /// followed by `Ok(())`, so the host runs it in full without compiling it.
    pub canonical_only: bool,
    /// The source calls `allowit::set_state`.
    pub writes_state: bool,
}
/// Check portable source and derive its shape: `scripts/policy_source.py` `checked_code`,
/// `standard_prefix` and `writes_state`. The gate accepts exactly the sources that
/// `policy_source.py` accepts: one self-contained file, `allowit` only as the path root, no file
/// or environment macros, no `#[path]`, `cfg_attr` or raw pointers, the macro, attribute and
/// identifier allowlists and one `pub fn execute(ctx: &AllowitContext) -> allowit::Result`.
/// The std-only compile (`policy_source.py` `host_compile`) needs `rustc` in the host build gate.
/// A source that is not canonical-only needs that compiled evaluator for full host
/// evaluation. `tests/policy_source.rs` replays vectors that the Python gate produced.
pub fn source_shape(source: &str) -> Result<SourceShape> {
    let code = gate::checked_code(source)?;
    let (prefix, canonical_only) = gate::standard_prefix(&code)?;
    Ok(SourceShape {
        prefix,
        canonical_only,
        writes_state: gate::writes_state(&code),
    })
}
/// The structural source gate, ported from `scripts/policy_source.py`.
pub(crate) mod gate {
    use super::*;
    use std::collections::{HashMap, HashSet};

    /// `policy_source.py` `MACROS`: the macros authored code may invoke.
    const MACROS: &[&str] = &[
        "assert",
        "assert_eq",
        "assert_ne",
        "panic",
        "matches",
        "unreachable",
    ];
    /// `policy_source.py` `ATTRIBUTES`: the attributes authored code may use.
    const ATTRIBUTES: &[&str] = &[
        "allow", "expect", "warn", "deny", "forbid", "inline", "cold", "must_use",
    ];
    /// `policy_source.py` `RUST_KEYWORDS`, which may precede a `!` operator.
    const RUST_KEYWORDS: &[&str] = &[
        "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
        "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
        "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub",
        "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "try",
        "type", "typeof", "union", "unsafe", "unsized", "use", "virtual", "where", "while",
        "yield",
    ];
    fn reject(what: &str) -> String {
        format!("Policy source must be one self-contained, portable file; it uses {what}")
    }
    /// The identifier that starts at `k` (`[A-Za-z_]\w*` at a word start) and its end.
    fn ident_at(chars: &[char], k: usize) -> Option<(String, usize)> {
        let first = *chars.get(k)?;
        if !(first.is_ascii_alphabetic() || first == '_') || (k > 0 && word(chars[k - 1])) {
            return None;
        }
        let mut end = k;
        while end < chars.len() && word(chars[end]) {
            end += 1;
        }
        Some((chars[k..end].iter().collect(), end))
    }
    /// The macro and attribute allowlists of `policy_source.py` `portable`: `name!` (not `!=`)
    /// must name an allowed macro or a keyword, and `#[name` an allowed attribute. Every other
    /// macro, such as `macro_rules!` or `asm!`, and every other attribute, such as `cfg` or
    /// `doc`, rejects.
    fn macros_and_attributes(chars: &[char]) -> Result<()> {
        for k in 0..chars.len() {
            if let Some((name, end)) = ident_at(chars, k) {
                let bang = skip_ws(chars, end);
                if chars.get(bang) == Some(&'!')
                    && chars.get(bang + 1) != Some(&'=')
                    && !RUST_KEYWORDS.contains(&name.as_str())
                    && !MACROS.contains(&name.as_str())
                {
                    return Err(reject(&format!(
                        "macro `{name}!`; only assert!, assert_eq!, assert_ne!, matches!, panic!, unreachable! are allowed"
                    )));
                }
            }
            if chars[k] == '#' {
                let open = skip_ws(chars, k + 1);
                if chars.get(open) != Some(&'[') {
                    continue;
                }
                let mut p = skip_ws(chars, open + 1);
                if starts(chars, p, "r#") {
                    p += 2;
                }
                let first = chars.get(p).copied().unwrap_or(' ');
                if !(first.is_ascii_alphabetic() || first == '_') {
                    continue;
                }
                let mut end = p;
                while end < chars.len() && word(chars[end]) {
                    end += 1;
                }
                let name: String = chars[p..end].iter().collect();
                if !ATTRIBUTES.contains(&name.as_str()) {
                    return Err(reject(&format!("attribute `{name}`")));
                }
            }
        }
        Ok(())
    }
    fn word(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    fn before_ok(chars: &[char], i: usize) -> bool {
        i == 0 || !(chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '_')
    }
    fn starts(chars: &[char], i: usize, text: &str) -> bool {
        (i..)
            .zip(text.chars())
            .all(|(k, c)| chars.get(k) == Some(&c))
    }
    /// `[bc]?r(#*)"` at `i`: the index after the opening quote and the number of hashes.
    fn raw_string(chars: &[char], i: usize) -> Option<(usize, usize)> {
        if !before_ok(chars, i) {
            return None;
        }
        let mut k = i;
        if matches!(chars.get(k), Some('b' | 'c')) {
            k += 1;
        }
        if chars.get(k) != Some(&'r') {
            return None;
        }
        k += 1;
        let start = k;
        while chars.get(k) == Some(&'#') {
            k += 1;
        }
        (chars.get(k) == Some(&'"')).then_some((k + 1, k - start))
    }
    /// `[bc]?"(?:\\[\s\S]|[^"\\])*"` at `i`: the index after the closing quote.
    fn string(chars: &[char], i: usize) -> Option<usize> {
        if !before_ok(chars, i) {
            return None;
        }
        let mut k = i;
        if matches!(chars.get(k), Some('b' | 'c')) {
            k += 1;
        }
        if chars.get(k) != Some(&'"') {
            return None;
        }
        k += 1;
        while k < chars.len() {
            match chars[k] {
                '\\' if k + 1 < chars.len() => k += 2,
                '\\' => return None,
                '"' => return Some(k + 1),
                _ => k += 1,
            }
        }
        None
    }
    /// A char literal at `i`: the index after its closing quote.
    fn char_literal(chars: &[char], i: usize) -> Option<usize> {
        if !before_ok(chars, i) {
            return None;
        }
        let mut k = i;
        if chars.get(k) == Some(&'b') {
            k += 1;
        }
        if chars.get(k) != Some(&'\'') {
            return None;
        }
        k += 1;
        match chars.get(k)? {
            '\\' => {
                k += 1;
                match chars.get(k)? {
                    'x' if chars
                        .get(k + 1..k + 3)
                        .is_some_and(|h| h.iter().all(char::is_ascii_hexdigit)) =>
                    {
                        k += 3
                    }
                    'u' if chars.get(k + 1) == Some(&'{') => {
                        let mut j = k + 2;
                        while chars
                            .get(j)
                            .is_some_and(|c| c.is_ascii_hexdigit() || *c == '_')
                        {
                            j += 1;
                        }
                        let len = j - (k + 2);
                        if (1..=8).contains(&len) && chars.get(j) == Some(&'}') {
                            k = j + 1;
                        } else {
                            // `\\u` without a valid escape is the plain `[^\n]` branch.
                            k += 1;
                        }
                    }
                    '\n' => return None,
                    _ => k += 1,
                }
            }
            '\'' | '\n' => return None,
            _ => k += 1,
        }
        (chars.get(k) == Some(&'\'')).then_some(k + 1)
    }
    /// `unescape` over a string literal's value: the characters its escapes denote. Only the
    /// braces of a macro message matter, so a surrogate becomes U+FFFD. An escape above U+10FFFF
    /// or without digits rejects, as Python `chr` and `int` raise there.
    fn unescape(content: &[char]) -> Result<String> {
        let mut out = String::new();
        let mut k = 0;
        while k < content.len() {
            if content[k] != '\\' {
                out.push(content[k]);
                k += 1;
                continue;
            }
            let hex = |c: &char| c.is_ascii_hexdigit();
            match content.get(k + 1) {
                Some('x') if content.get(k + 2..k + 4).is_some_and(|h| h.iter().all(hex)) => {
                    let text: String = content[k + 2..k + 4].iter().collect();
                    out.push(char::from(u8::from_str_radix(&text, 16).map_err(err)?));
                    k += 4;
                }
                Some('u')
                    if content.get(k + 2) == Some(&'{') && {
                        let run = content[(k + 3).min(content.len())..]
                            .iter()
                            .take_while(|c| hex(c) || **c == '_')
                            .count();
                        (1..=8).contains(&run) && content.get(k + 3 + run) == Some(&'}')
                    } =>
                {
                    let run = content[k + 3..]
                        .iter()
                        .take_while(|c| hex(c) || **c == '_')
                        .count();
                    let digits: String = content[k + 3..k + 3 + run]
                        .iter()
                        .filter(|c| **c != '_')
                        .collect();
                    let value = u32::from_str_radix(&digits, 16)
                        .map_err(|_| "Policy source has an invalid string escape")?;
                    if value > 0x10FFFF {
                        return Err("Policy source has an invalid string escape".into());
                    }
                    out.push(char::from_u32(value).unwrap_or('\u{FFFD}'));
                    k += 4 + run;
                }
                Some('\n') => {
                    k += 2;
                    while k < content.len() && py_space(content[k]) {
                        k += 1;
                    }
                }
                Some(c) => {
                    out.push(match c {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '0' => '\0',
                        other => *other,
                    });
                    k += 2;
                }
                None => k += 1,
            }
        }
        Ok(out)
    }
    /// Source with comments blanked and literals replaced by `0`: `rust_code`. Also returns the
    /// value of each string literal, keyed by the offset of its `0` in the code.
    pub(crate) fn rust_code(source: &str) -> Result<(String, HashMap<usize, String>)> {
        let chars: Vec<char> = source.chars().collect();
        let n = chars.len();
        let mut out = String::new();
        let mut strings = HashMap::new();
        let mut i = 0;
        while i < n {
            if starts(&chars, i, "//") {
                i = (i..n).find(|k| chars[*k] == '\n').unwrap_or(n);
                out.push(' ');
            } else if starts(&chars, i, "/*") {
                let mut depth = 1;
                i += 2;
                while depth > 0 {
                    if i >= n {
                        return Err("Policy source has an unterminated block comment".into());
                    }
                    if starts(&chars, i, "/*") {
                        depth += 1;
                        i += 2;
                    } else if starts(&chars, i, "*/") {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                out.push(' ');
            } else if let Some((after, hashes)) = raw_string(&chars, i) {
                let close: String = std::iter::once('"')
                    .chain(std::iter::repeat_n('#', hashes))
                    .collect();
                let close: Vec<char> = close.chars().collect();
                let end = (after..=n.saturating_sub(close.len()))
                    .find(|k| chars[*k..*k + close.len()] == close[..])
                    .ok_or("Policy source has an unterminated raw string")?;
                strings.insert(out.chars().count() + 1, chars[after..end].iter().collect());
                i = end + close.len();
                out.push_str(" 0 ");
            } else if let Some(end) = string(&chars, i) {
                let quote = if chars[i] == '"' { i } else { i + 1 };
                strings.insert(
                    out.chars().count() + 1,
                    unescape(&chars[quote + 1..end - 1])?,
                );
                i = end;
                out.push_str(" 0 ");
            } else if let Some(end) = char_literal(&chars, i) {
                i = end;
                out.push_str(" 0 ");
            } else if chars[i] == '"' || starts(&chars, i, "b\"") || starts(&chars, i, "c\"") {
                return Err("Policy source has an unterminated string".into());
            } else {
                out.push(chars[i]);
                i += 1;
            }
        }
        Ok((out, strings))
    }
    /// Each match of the whole word `name` in `code`, as (start, end) char offsets.
    fn words<'a>(
        chars: &'a [char],
        names: &'a [&str],
    ) -> impl Iterator<Item = (usize, usize)> + 'a {
        let mut i = 0;
        std::iter::from_fn(move || {
            while i < chars.len() {
                if word(chars[i]) && (i == 0 || !word(chars[i - 1])) {
                    let start = i;
                    while i < chars.len() && word(chars[i]) {
                        i += 1;
                    }
                    let text: String = chars[start..i].iter().collect();
                    if names.contains(&text.as_str()) {
                        return Some((start, i));
                    }
                } else {
                    i += 1;
                }
            }
            None
        })
    }
    fn trimmed_before(chars: &[char], start: usize) -> String {
        chars[..start]
            .iter()
            .collect::<String>()
            .trim_end_matches(py_space)
            .to_string()
    }
    fn trimmed_after(chars: &[char], end: usize) -> String {
        chars[end..]
            .iter()
            .collect::<String>()
            .trim_start_matches(py_space)
            .to_string()
    }
    /// `&\s*mut\b[^;,{}]*\.$` on the text before an `env` field.
    fn mut_borrow_before(before: &str) -> bool {
        let b: Vec<char> = before.chars().collect();
        if b.last() != Some(&'.') {
            return false;
        }
        (0..b.len()).any(|i| {
            if b[i] != '&' {
                return false;
            }
            let mut k = i + 1;
            while k < b.len() && py_space(b[k]) {
                k += 1;
            }
            if !starts(&b, k, "mut") || b.get(k + 3).is_some_and(|c| word(*c)) {
                return false;
            }
            b[k + 3..b.len() - 1]
                .iter()
                .all(|c| !matches!(c, ';' | ',' | '{' | '}'))
        })
    }
    fn skip_ws(chars: &[char], mut k: usize) -> usize {
        while k < chars.len() && py_space(chars[k]) {
            k += 1;
        }
        k
    }
    /// The structural part of `checked_code`; returns the lexed code.
    pub(crate) fn checked_code(source: &str) -> Result<String> {
        let src: Vec<char> = source.chars().collect();
        // `^\s*(//!|/\*!|#\s*!\s*\[)` in multiline mode.
        for start in
            std::iter::once(0).chain((0..src.len()).filter(|k| src[*k] == '\n').map(|k| k + 1))
        {
            let k = skip_ws(&src, start);
            let inner = starts(&src, k, "//!") || starts(&src, k, "/*!") || {
                src.get(k) == Some(&'#') && {
                    let j = skip_ws(&src, k + 1);
                    src.get(j) == Some(&'!') && src.get(skip_ws(&src, j + 1)) == Some(&'[')
                }
            };
            if inner {
                return Err(
                    "Policy source must not use inner attributes or inner doc comments".into(),
                );
            }
        }
        let (code, strings) = rust_code(source)?;
        let chars: Vec<char> = code.chars().collect();
        for (start, end) in words(&chars, &["allowit"]) {
            if !trimmed_after(&chars, end).starts_with("::")
                || trimmed_before(&chars, start).ends_with("::")
            {
                return Err("Policy source must use `allowit` only as the path root `allowit::`; it must not rebind it".into());
            }
        }
        for (start, end) in words(
            &chars,
            &[
                "include",
                "include_str",
                "include_bytes",
                "env",
                "option_env",
            ],
        ) {
            let before = trimmed_before(&chars, start);
            let after = trimmed_after(&chars, end);
            let field = before.ends_with('.') && !before.ends_with("..");
            if !field
                || after.starts_with(['.', '('])
                || (after.starts_with('!') && !after.starts_with("!="))
            {
                return Err("Policy source must be one self-contained file; it uses file or environment macro".into());
            }
            if (after.starts_with('=') && !after.starts_with("==")) || mut_borrow_before(&before) {
                return Err(reject("a write to the `env` field"));
            }
        }
        // `#\s*\[\s*path\b`
        for k in (0..chars.len()).filter(|k| chars[*k] == '#') {
            let j = skip_ws(&chars, k + 1);
            if chars.get(j) == Some(&'[') {
                let p = skip_ws(&chars, j + 1);
                if starts(&chars, p, "path") && !chars.get(p + 4).is_some_and(|c| word(*c)) {
                    return Err(
                        "Policy source must be one self-contained file; it uses #[path] attribute"
                            .into(),
                    );
                }
            }
        }
        if words(&chars, &["cfg_attr"]).next().is_some() {
            return Err(
                "Policy source must be one self-contained file; it uses cfg_attr attribute".into(),
            );
        }
        // `\*\s*(?:const|mut)\b|\braw\s+(?:const|mut)\b`
        let pointer = (0..chars.len()).any(|k| {
            let after_word = |p: usize, w: &str| {
                starts(&chars, p, w) && !chars.get(p + w.len()).is_some_and(|c| word(*c))
            };
            if chars[k] == '*' {
                let p = skip_ws(&chars, k + 1);
                return after_word(p, "const") || after_word(p, "mut");
            }
            if starts(&chars, k, "raw") && (k == 0 || !word(chars[k - 1])) {
                let p = skip_ws(&chars, k + 3);
                return p > k + 3 && (after_word(p, "const") || after_word(p, "mut"));
            }
            false
        });
        if pointer {
            return Err(
                "Policy source must be one self-contained file; it uses a raw pointer".into(),
            );
        }
        macros_and_attributes(&chars)?;
        portable(&code, &strings)?;
        Ok(code)
    }
    // The rest of `policy_source.py` `portable` and `allowlisted`: the code-level allowlist.
    // Code is ASCII here, so byte offsets equal the character offsets of `rust_code`.

    /// `policy_source.py` `MESSAGE`: the argument index of each macro's optional message.
    const MESSAGE: &[(&str, usize)] = &[
        ("panic", 0),
        ("unreachable", 0),
        ("assert", 1),
        ("assert_eq", 2),
        ("assert_ne", 2),
    ];
    const ALLOWED_KEYWORDS: &[&str] = &[
        "break", "const", "continue", "else", "false", "fn", "for", "if", "in", "let", "loop",
        "match", "mut", "pub", "ref", "return", "true", "use", "while",
    ];
    const INTEGERS: &[&str] = &[
        "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128",
    ];
    const NOT_PORTABLE: &[&str] = &["usize", "isize", "f16", "f32", "f64", "f128"];
    const ERRORS: &[&str] = &[
        "MissingParam",
        "PaymentLimitExceeded",
        "LifetimeSpendingLimitExceeded",
        "DailySpendingLimitExceeded",
        "DailyVolumeLimitExceeded",
        "CallLimitExceeded",
        "DestinationNotAllowed",
        "TokenNotAllowed",
        "Expired",
        "Unpriced",
        "InvalidParam",
        "InvalidState",
        "Overflow",
        "Unsupported",
        "OperationNotAllowed",
        "Panicked",
    ];
    const VALUES: &[&str] = &["Int", "Bool", "Str", "Addr", "List"];
    const SERVER_FUNCTIONS: &[&str] = &[
        "price",
        "fetch",
        "internet_search",
        "marketplace_search",
        "systemone",
        "request_screening",
        "clear_systemone_context",
    ];
    const SERVER_TYPES: &[&str] = &[
        "AllowitPrice",
        "AllowitFetch",
        "AllowitInternetSearch",
        "AllowitMarketplaceSearch",
        "AllowitRequestScreening",
        "AllowitSystemOne",
    ];
    const STRUCT_FIELDS: &[&str] = &[
        "from",
        "to",
        "amount",
        "price",
        "slip",
        "method",
        "url",
        "data",
        "pubkey",
        "query",
        "id",
        "deny_below",
        "allow_above",
    ];
    const PRELUDE_FUNCTIONS: &[&str] =
        &["drop", "size_of", "size_of_val", "align_of", "align_of_val"];
    const FIELDS: &[&str] = &[
        "env",
        "params",
        "request",
        "operation",
        "asset",
        "recipient",
        "amount",
        "token_in",
        "amount_in",
        "token_out",
        "min_out",
        "now",
        "expires_at",
        "spent_units",
        "gas_units",
        "unpriced",
        "args",
    ];
    /// `FIELD_METHODS`: `contains` only on the `params` or `args` field.
    const FIELD_NAMES: &[&str] = &["params", "args"];
    /// `INDEXED_FIELD`: the only indexed receiver, `ctx.params[key]`.
    const INDEXED_FIELD: &str = "params";
    const METHODS: &[&str] = &[
        "clone",
        "eq",
        "ne",
        "lt",
        "le",
        "gt",
        "ge",
        "cmp",
        "partial_cmp",
        "min",
        "max",
        "clamp",
        "is_some",
        "is_none",
        "is_some_and",
        "is_none_or",
        "is_ok",
        "is_err",
        "is_ok_and",
        "is_err_and",
        "ok",
        "err",
        "ok_or",
        "unwrap",
        "expect",
        "unwrap_or",
        "unwrap_or_default",
        "copied",
        "cloned",
        "as_ref",
        "xor",
        "or",
        "and",
        "checked_add",
        "checked_sub",
        "checked_mul",
        "checked_div",
        "checked_rem",
        "checked_neg",
        "checked_abs",
        "checked_pow",
        "saturating_add",
        "saturating_sub",
        "saturating_mul",
        "saturating_pow",
        "pow",
        "abs",
        "abs_diff",
        "unsigned_abs",
        "signum",
        "is_positive",
        "is_negative",
        "div_euclid",
        "rem_euclid",
        "is_lt",
        "is_le",
        "is_gt",
        "is_ge",
        "is_eq",
        "is_ne",
        "reverse",
        "then_some",
        "contains",
    ];
    const OPERAND_END: &[&str] = &[")", "]", "?", "true", "false"];

    /// The name lists that `policy_source.py` builds from the constants above.
    struct Names {
        allowit_types: HashSet<String>,
        types: HashSet<String>,
        constructors: HashSet<String>,
        primitives: HashSet<String>,
        function_paths: HashSet<String>,
        paths: HashSet<String>,
    }
    fn lists() -> &'static Names {
        static NAMES: std::sync::OnceLock<Names> = std::sync::OnceLock::new();
        NAMES.get_or_init(|| {
            let set = |items: Vec<String>| items.into_iter().collect::<HashSet<_>>();
            let functions: Vec<String> = param_keys()
                .iter()
                .chain(["param", "arg", "get_state", "set_state"].iter())
                .chain(SERVER_FUNCTIONS.iter())
                .map(|s| s.to_string())
                .collect();
            let mut allowit_types: Vec<String> = [
                "AllowitContext",
                "AllowitError",
                "AllowitOperation",
                "AllowitRequest",
                "AllowitRule",
                "AllowitValue",
                "Address",
                "Result",
                "String",
                "Vec",
                "DAY",
            ]
            .map(String::from)
            .to_vec();
            allowit_types.extend(param_keys().iter().map(|k| k.to_uppercase()));
            allowit_types.extend(SERVER_TYPES.iter().map(|t| t.to_string()));
            let mut types = allowit_types.clone();
            types.extend(["Option", "Some", "None", "Ok", "Err"].map(String::from));
            let mut constructors: Vec<String> = [
                "Some",
                "Ok",
                "Err",
                "Option::Some",
                "Result::Ok",
                "Result::Err",
            ]
            .map(String::from)
            .to_vec();
            for prefix in ["", "allowit::"] {
                for v in VALUES {
                    constructors.push(format!("{prefix}AllowitValue::{v}"));
                }
                constructors.push(format!("{prefix}AllowitOperation::custom"));
            }
            let mut primitives: Vec<String> = INTEGERS.iter().map(|t| t.to_string()).collect();
            primitives.extend(["bool", "char", "str", "_"].map(String::from));
            let mut function_paths: Vec<String> =
                functions.iter().map(|f| format!("allowit::{f}")).collect();
            for t in INTEGERS {
                for f in ["from", "try_from"] {
                    function_paths.push(format!("{t}::{f}"));
                }
            }
            function_paths.extend(["core::cmp::min", "core::cmp::max"].map(String::from));
            let mut paths = function_paths.clone();
            paths.extend(allowit_types.iter().map(|t| format!("allowit::{t}")));
            paths.push("allowit::prelude".into());
            let rules: Vec<&str> = param_keys().to_vec();
            let enums: [(&str, &[&str]); 4] = [
                (
                    "AllowitOperation",
                    &["transfer", "swap", "deposit", "redeem", "custom"],
                ),
                ("AllowitError", ERRORS),
                ("AllowitRule", &rules),
                ("AllowitValue", VALUES),
            ];
            for (name, variants) in enums {
                for variant in variants {
                    for prefix in ["", "allowit::"] {
                        paths.push(format!("{prefix}{name}::{variant}"));
                    }
                }
            }
            paths.extend(
                ["Option::Some", "Option::None", "Result::Ok", "Result::Err"].map(String::from),
            );
            for t in INTEGERS {
                for c in ["MAX", "MIN"] {
                    paths.push(format!("{t}::{c}"));
                }
            }
            paths.push("core::cmp::Ordering".into());
            for o in ["Less", "Equal", "Greater"] {
                paths.push(format!("core::cmp::Ordering::{o}"));
            }
            Names {
                allowit_types: set(allowit_types),
                types: set(types),
                constructors: set(constructors),
                primitives: set(primitives),
                function_paths: set(function_paths),
                paths: set(paths),
            }
        })
    }
    /// Python `str.isspace` for one character: Unicode whitespace plus the ASCII separators.
    fn py_space(c: char) -> bool {
        c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
    }
    fn ascii_word(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }
    /// `IDENT` fullmatch: `(?:r#)?[A-Za-z_]\w*`.
    fn is_ident(token: &str) -> bool {
        let t = token.strip_prefix("r#").unwrap_or(token).as_bytes();
        t.first()
            .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
            && t.iter().all(|b| ascii_word(*b))
    }
    fn strip_raw(token: &str) -> &str {
        token.strip_prefix("r#").unwrap_or(token)
    }
    fn keyword(token: &str) -> bool {
        RUST_KEYWORDS.contains(&token)
    }
    fn open(token: &str) -> bool {
        matches!(token, "(" | "[" | "{")
    }
    fn close(token: &str) -> bool {
        matches!(token, ")" | "]" | "}")
    }
    /// `NUMBER` fullmatch: an integer literal with an optional fixed-width suffix.
    fn number(token: &str) -> bool {
        let suffix = |rest: &str| rest.is_empty() || INTEGERS.contains(&rest);
        for (prefix, digit) in [
            ("0x", u8::is_ascii_hexdigit as fn(&u8) -> bool),
            ("0o", |b: &u8| (b'0'..=b'7').contains(b)),
            ("0b", |b: &u8| matches!(b, b'0' | b'1')),
        ] {
            if let Some(rest) = token.strip_prefix(prefix) {
                let n = rest.bytes().take_while(|b| digit(b) || *b == b'_').count();
                if n > 0 && suffix(&rest[n..]) {
                    return true;
                }
            }
        }
        let n = token
            .bytes()
            .take_while(|b| b.is_ascii_digit() || *b == b'_')
            .count();
        token.as_bytes().first().is_some_and(u8::is_ascii_digit) && suffix(&token[n..])
    }
    /// `TOKEN.finditer`: each token of ASCII code and its start offset.
    fn tokens(code: &str) -> Vec<(usize, String)> {
        let b = code.as_bytes();
        let word_end = |mut k: usize| {
            while k < b.len() && ascii_word(b[k]) {
                k += 1;
            }
            k
        };
        let mut out = vec![];
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            let end = if code[i..].starts_with("r#")
                && b.get(i + 2)
                    .is_some_and(|x| x.is_ascii_alphabetic() || *x == b'_')
            {
                word_end(i + 2)
            } else if c.is_ascii_alphabetic() || c == b'_' {
                word_end(i + 1)
            } else if ["::", "=>", "->", "||"]
                .iter()
                .any(|t| code[i..].starts_with(t))
            {
                i + 2
            } else if c == b'$'
                || c.is_ascii_digit()
                || (c == b'\'' && b.get(i + 1).is_some_and(|x| ascii_word(*x)))
            {
                word_end(i + 1)
            } else if py_space(c as char) {
                i += 1;
                continue;
            } else {
                i + 1
            };
            out.push((i, code[i..end].to_string()));
            i = end;
        }
        out
    }
    /// `after_generics`: the index after the generic arguments that open at `tokens[j]`.
    fn after_generics(tokens: &[String], j: usize) -> Result<usize> {
        let mut depth = 0i64;
        for (k, token) in tokens.iter().enumerate().skip(j) {
            depth += i64::from(token == "<") - i64::from(token == ">");
            if depth == 0 {
                return Ok(k + 1);
            }
        }
        Err(reject("unterminated generic arguments"))
    }
    /// `after_brackets`: the index after the bracket group that opens at `tokens[j]`.
    fn after_brackets(tokens: &[String], j: usize) -> Result<usize> {
        let mut depth = 0i64;
        for (k, token) in tokens.iter().enumerate().skip(j) {
            depth += i64::from(open(token)) - i64::from(close(token));
            if depth == 0 {
                return Ok(k + 1);
            }
        }
        Err(reject("an unterminated attribute"))
    }
    /// `operand`: the token ends an operand.
    fn operand(token: &str) -> bool {
        OPERAND_END.contains(&token)
            || token.as_bytes().first().is_some_and(u8::is_ascii_digit)
            || (is_ident(token) && !keyword(token))
    }
    /// Python `tokens[k]`, where a negative index counts from the end.
    fn at(tokens: &[String], k: isize) -> &str {
        let k = if k < 0 { tokens.len() as isize + k } else { k };
        if k < 0 {
            return "";
        }
        tokens.get(k as usize).map_or("", String::as_str)
    }
    /// `declared`: the lowercase names the file declares.
    fn declared(tokens: &[String]) -> HashSet<String> {
        let mut names = HashSet::new();
        let n = tokens.len();
        let bind = |names: &mut HashSet<String>, k: usize| {
            let token = tokens[k].as_str();
            if is_ident(token)
                && !keyword(token)
                && !token.starts_with(|c: char| c.is_ascii_uppercase())
                && at(tokens, k as isize - 1) != "::"
                && !matches!(at(tokens, k as isize + 1), "(" | "::" | "{" | "!")
            {
                names.insert(strip_raw(token).to_string());
            }
        };
        let pattern = |names: &mut HashSet<String>, mut k: usize, stops: &[&str]| {
            let mut depth = 0i64;
            while k < n && !(depth == 0 && stops.contains(&tokens[k].as_str())) {
                depth += i64::from(open(&tokens[k])) - i64::from(close(&tokens[k]));
                if depth < 0 {
                    return;
                }
                bind(names, k);
                k += 1;
            }
        };
        for k in 0..n.saturating_sub(1) {
            let token = tokens[k].as_str();
            if token == "fn" || token == "const" {
                names.insert(strip_raw(&tokens[k + 1]).to_string());
            }
            if token == "let" {
                pattern(&mut names, k + 1, &["=", ";", ":"]);
            } else if token == "for" {
                pattern(&mut names, k + 1, &["in"]);
            } else if token == "fn" {
                let Some(start) = (k..n).find(|j| tokens[*j] == "(") else {
                    continue;
                };
                let mut depth = 0i64;
                for (j, t) in tokens.iter().enumerate().take(n).skip(start) {
                    depth += i64::from(open(t)) - i64::from(close(t));
                    if depth == 0 {
                        break;
                    }
                    if depth == 1 && (t == "(" || t == ",") {
                        pattern(&mut names, j + 1, &[":", ",", ")"]);
                    }
                }
            } else if token == "=>" {
                let mut depth = 0i64;
                let mut found = vec![];
                for j in (0..k).rev() {
                    let t = tokens[j].as_str();
                    depth += i64::from(matches!(t, ")" | "]")) - i64::from(matches!(t, "(" | "["));
                    if depth == 0 && matches!(t, "," | "{" | "}") {
                        break;
                    }
                    if depth == 0 && t == "if" {
                        found.clear();
                    }
                    found.push(j);
                }
                for j in found {
                    bind(&mut names, j);
                }
            }
        }
        names
    }
    /// `uses`: check the `use` declaration at `tokens[i]`; its imported names and the next index.
    fn uses(tokens: &[String], i: usize) -> Result<(Vec<String>, usize)> {
        let end = (i..tokens.len())
            .find(|k| tokens[*k] == ";")
            .unwrap_or(tokens.len());
        let tree: String = tokens[(i + 1).min(end)..end].concat();
        if tree == "allowit::prelude::*" {
            return Ok((vec![], end + 1));
        }
        let names: Vec<String> = if let Some(inner) = tree
            .strip_prefix("allowit::{")
            .and_then(|t| t.strip_suffix('}'))
        {
            inner.split(',').map(String::from).collect()
        } else if let Some(name) = tree.strip_prefix("allowit::") {
            vec![name.to_string()]
        } else {
            vec![]
        };
        let names: Vec<String> = names.into_iter().filter(|n| !n.is_empty()).collect();
        if names.is_empty() || names.iter().any(|n| !lists().allowit_types.contains(n)) {
            return Err(reject(&format!(
                "`use {tree}`; only `use allowit::prelude::*` and `allowit` types may be imported"
            )));
        }
        Ok((names, end + 1))
    }
    /// `allowlisted`: every identifier, path, method, field and literal is on the allowlist.
    fn allowlisted(code: &str) -> Result<()> {
        if !code.is_ascii() {
            return Err(reject("a non-ASCII identifier"));
        }
        let lists = lists();
        let tokens: Vec<String> = tokens(code).into_iter().map(|(_, t)| t).collect();
        let n = tokens.len();
        let tok = |k: usize| tokens.get(k).map_or("", String::as_str);
        let local_fns: HashSet<&str> = (0..n.saturating_sub(1))
            .filter(|k| tokens[*k] == "fn")
            .map(|k| strip_raw(&tokens[k + 1]))
            .collect();
        let local_names = declared(&tokens);
        let mut reserved: Vec<&&str> = FIELD_NAMES
            .iter()
            .filter(|f| local_names.contains(**f))
            .collect();
        reserved.sort();
        if let Some(name) = reserved.first() {
            return Err(reject(&format!(
                "the name `{name}` for a binding or function; it is reserved for the context field"
            )));
        }
        let mut defined: HashSet<String> = (0..n.saturating_sub(1))
            .filter(|k| tokens[*k] == "const" && is_ident(&tokens[k + 1]))
            .map(|k| tokens[k + 1].clone())
            .collect();
        let mut i = 0;
        while i < n {
            let token = tokens[i].as_str();
            let before = if i > 0 { tokens[i - 1].as_str() } else { "" };
            let after = tok(i + 1);
            if token == "#" && after == "[" {
                i = after_brackets(&tokens, i + 1)?;
                continue;
            }
            if token == "use" {
                let (imported, next) = uses(&tokens, i)?;
                defined.extend(imported);
                i = next;
                continue;
            }
            if token.as_bytes()[0].is_ascii_digit() {
                if before != "."
                    && (!number(token)
                        || (after == "." && tok(i + 2) != "." && !is_ident(tok(i + 2))))
                {
                    return Err(reject(&format!(
                        "the literal `{token}`; only integer literals with fixed-width suffixes are portable"
                    )));
                }
                i += 1;
                continue;
            }
            if token == "::" {
                return Err(reject("a path that does not start at an allowed name"));
            }
            if token == "|" || token == "||" {
                if !operand(before) {
                    return Err(reject(
                        "a closure, which can become a function pointer whose address differs between the backends",
                    ));
                }
                i += 1;
                continue;
            }
            if !is_ident(token) {
                i += 1;
                continue;
            }
            let name = strip_raw(token);
            if token == name && keyword(name) {
                if !ALLOWED_KEYWORDS.contains(&name) {
                    return Err(reject(&format!("the keyword `{name}`")));
                }
                if name == "fn" && !is_ident(after) {
                    return Err(reject(
                        "a function pointer type, whose address differs between the backends",
                    ));
                }
                i += 1;
                continue;
            }
            if after == "!" && tok(i + 2) != "=" {
                i += 1;
                continue;
            }
            if before == "." && (i < 2 || tokens[i - 2] != ".") {
                if after == "(" || after == "::" {
                    if !METHODS.contains(&name) {
                        return Err(reject(&format!(
                            "the method `{name}`, which is not on the portable allowlist"
                        )));
                    }
                    if name == "contains"
                        && (i < 3
                            || !FIELD_NAMES.contains(&tokens[i - 2].as_str())
                            || tokens[i - 3] != ".")
                    {
                        return Err(reject(
                            "the method `contains` other than on the field `.params` or `.args`",
                        ));
                    }
                } else if !FIELDS.contains(&name) {
                    return Err(reject(&format!(
                        "the field `{name}`, which is not on the portable allowlist"
                    )));
                }
                i += 1;
                continue;
            }
            let mut segments = vec![name.to_string()];
            let mut j = i + 1;
            while tok(j) == "::" && j + 1 < n && is_ident(&tokens[j + 1]) {
                segments.push(strip_raw(&tokens[j + 1]).to_string());
                j += 2;
            }
            let turbofish = tok(j) == "::" && tok(j + 1) == "<";
            if segments.len() > 1 || turbofish {
                let path = segments.join("::");
                if !lists.paths.contains(&path) {
                    return Err(reject(&format!(
                        "the path `{path}`, which is not on the portable allowlist"
                    )));
                }
                let call = if turbofish {
                    after_generics(&tokens, j + 1)?
                } else {
                    j
                };
                if (lists.function_paths.contains(&path) || lists.constructors.contains(&path))
                    && tok(call) != "("
                {
                    return Err(reject(&format!(
                        "the function `{path}` as a value, whose address differs between the backends"
                    )));
                }
                i = if turbofish { j + 1 } else { j };
                continue;
            }
            if STRUCT_FIELDS.contains(&name) && after == ":" && tok(i + 2) != ":" {
                i += 1;
                continue;
            }
            if local_fns.contains(name) && after != "(" && before != "fn" {
                return Err(reject(&format!(
                    "the function `{name}` as a value, whose address differs between the backends"
                )));
            } else if name.starts_with(|c: char| c.is_ascii_uppercase()) {
                if !lists.types.contains(name) && !defined.contains(name) {
                    return Err(reject(&format!(
                        "the name `{name}`, which is not on the portable allowlist"
                    )));
                }
                if lists.constructors.contains(name) && after != "(" {
                    return Err(reject(&format!(
                        "the constructor `{name}` as a value, whose address differs between the backends"
                    )));
                }
            } else if NOT_PORTABLE.contains(&name) {
                return Err(reject(&format!(
                    "the type `{name}`, whose width or representation differs between the backends"
                )));
            } else if PRELUDE_FUNCTIONS.contains(&name) {
                return Err(reject(&format!(
                    "the prelude function `{name}`, which can observe a pointer width or layout"
                )));
            } else if !local_names.contains(name) && !lists.primitives.contains(name) {
                return Err(reject(&format!(
                    "the name `{name}`, which the file does not declare"
                )));
            }
            i += 1;
        }
        Ok(())
    }
    /// `macro_args`: the top-level comma-separated token groups of the macro arguments at `i`.
    fn macro_args(tokens: &[String], i: usize) -> Result<Vec<Vec<String>>> {
        let end = after_brackets(tokens, i)?;
        let mut groups: Vec<Vec<String>> = vec![vec![]];
        let mut depth = 0i64;
        for token in &tokens[i + 1..end - 1] {
            depth += i64::from(open(token)) - i64::from(close(token));
            if depth == 0 && token == "," {
                groups.push(vec![]);
            } else {
                groups.last_mut().unwrap().push(token.clone());
            }
        }
        Ok(groups)
    }
    /// `plain_braces`: a macro message admits only `{}`, `{:?}`, `{{` and `}}`.
    fn plain_braces(value: &str) -> Result<()> {
        let v: Vec<char> = value.chars().collect();
        let mut i = 0;
        while i < v.len() {
            let start = i;
            if (v[i] == '{' && v.get(i + 1) == Some(&'{'))
                || (v[i] == '}' && v.get(i + 1) == Some(&'}'))
            {
                i += 2;
            } else if v[i] == '{' {
                i += 1;
                while i < v.len() && v[i] != '{' && v[i] != '}' {
                    i += 1;
                }
                if v.get(i) == Some(&'}') {
                    i += 1;
                }
            } else if v[i] == '}' {
                i += 1;
            } else {
                i += 1;
                continue;
            }
            let brace: String = v[start..i].iter().collect();
            if !["{{", "}}", "{}", "{:?}"].contains(&brace.as_str()) {
                return Err(reject(&format!(
                    "the format placeholder `{brace}`; only `{{}}` and `{{:?}}` are portable"
                )));
            }
        }
        Ok(())
    }
    /// `[{,]\s*\.\.(?![=.])\s*[^\s}\]),]`: struct update syntax.
    fn struct_update(b: &[u8]) -> bool {
        let skip = |mut k: usize| {
            while k < b.len() && py_space(b[k] as char) {
                k += 1;
            }
            k
        };
        (0..b.len()).any(|p| {
            if !matches!(b[p], b'{' | b',') {
                return false;
            }
            let q = skip(p + 1);
            if b.get(q) != Some(&b'.') || b.get(q + 1) != Some(&b'.') {
                return false;
            }
            if matches!(b.get(q + 2), Some(b'=' | b'.')) {
                return false;
            }
            let r = skip(q + 2);
            b.get(r)
                .is_some_and(|c| !py_space(*c as char) && !matches!(c, b'}' | b']' | b')' | b','))
        })
    }
    /// The ends of one `&\s*(?:'\w+\s*)?(?:mut\s+)?` unit that starts at `p`.
    fn borrow_unit_ends(b: &[u8], p: usize) -> Vec<usize> {
        let mut ends = vec![];
        if b.get(p) != Some(&b'&') {
            return ends;
        }
        let spaces = |k: usize| {
            let mut out = vec![k];
            let mut k = k;
            while k < b.len() && py_space(b[k] as char) {
                k += 1;
                out.push(k);
            }
            out
        };
        for q in spaces(p + 1) {
            let mut lifetimes = vec![q];
            if b.get(q) == Some(&b'\'') {
                let mut w = q + 1;
                while w < b.len() && ascii_word(b[w]) {
                    w += 1;
                    lifetimes.extend(spaces(w));
                }
            }
            for l in lifetimes {
                ends.push(l);
                if b[l.min(b.len())..].starts_with(b"mut") {
                    let after: Vec<usize> = spaces(l + 3).into_iter().skip(1).collect();
                    ends.extend(after);
                }
            }
        }
        ends.sort_unstable();
        ends.dedup();
        ends
    }
    /// `re.sub(r"(?:&\s*(?:'\w+\s*)?(?:mut\s+)?)+$", "", text)`.
    fn strip_borrows(text: &str) -> &str {
        let b = text.as_bytes();
        for s in 0..b.len() {
            let mut frontier = vec![s];
            while !frontier.is_empty() {
                let mut next: Vec<usize> = frontier
                    .iter()
                    .flat_map(|p| borrow_unit_ends(b, *p))
                    .collect();
                if next.contains(&b.len()) {
                    return &text[..s];
                }
                next.sort_unstable();
                next.dedup();
                frontier = next;
            }
        }
        text
    }
    fn py_rstrip(text: &str) -> &str {
        text.trim_end_matches(py_space)
    }
    /// `\bAllowitContext\s*\{` outside a return type: a context struct expression or pattern.
    fn context_struct(code: &str) -> bool {
        let chars: Vec<char> = code.chars().collect();
        words(&chars, &["AllowitContext"]).any(|(start, end)| {
            if chars.get(skip_ws(&chars, end)) != Some(&'{') {
                return false;
            }
            let before = py_rstrip(strip_borrows(py_rstrip(&code[..start])));
            !before.ends_with("->")
        })
    }
    /// `\.\s*(?:params|args)\s*\.\s*\d`: a tuple field of params or args.
    fn map_field(b: &[u8]) -> bool {
        let skip = |mut k: usize| {
            while k < b.len() && py_space(b[k] as char) {
                k += 1;
            }
            k
        };
        (0..b.len()).any(|p| {
            if b[p] != b'.' {
                return false;
            }
            let q = skip(p + 1);
            let Some(len) = ["params", "args"]
                .iter()
                .find(|f| b[q..].starts_with(f.as_bytes()))
                .map(|f| f.len())
            else {
                return false;
            };
            let r = skip(q + len);
            if b.get(r) != Some(&b'.') {
                return false;
            }
            b.get(skip(r + 1)).is_some_and(u8::is_ascii_digit)
        })
    }
    /// `portable` after its macro and attribute checks, then `allowlisted`.
    pub(crate) fn portable(code: &str, strings: &HashMap<usize, String>) -> Result<()> {
        // Every non-ASCII code character rejects in `allowlisted`; checking first keeps the
        // byte and character offsets equal below.
        if !code.is_ascii() {
            return Err(reject("a non-ASCII identifier"));
        }
        let b = code.as_bytes();
        if struct_update(b) {
            return Err(reject("struct update syntax `..`"));
        }
        if context_struct(code) {
            return Err(reject("a struct expression or pattern of `AllowitContext`"));
        }
        if map_field(b) {
            return Err(reject(
                "the map inside params or args; read params as `ctx.params[key]` and args with `allowit::arg`",
            ));
        }
        let spans = tokens(code);
        let toks: Vec<String> = spans.iter().map(|(_, t)| t.clone()).collect();
        for k in 0..toks.len().saturating_sub(2) {
            let name = strip_raw(&toks[k]);
            let Some((_, index)) = MESSAGE.iter().find(|(m, _)| *m == name) else {
                continue;
            };
            if toks[k + 1] != "!" || !open(&toks[k + 2]) {
                continue;
            }
            let args = macro_args(&toks, k + 2)?;
            let message: &[String] = args.get(*index).map_or(&[], Vec::as_slice);
            if !(message.is_empty() || message == ["0"]) {
                return Err(reject(&format!(
                    "a `{name}!` message that is not one string literal"
                )));
            }
            if !message.is_empty() {
                let at = k + 3 + args[..*index].iter().map(|g| g.len() + 1).sum::<usize>();
                plain_braces(strings.get(&spans[at].0).map_or("", String::as_str))?;
            }
        }
        for k in 1..toks.len() {
            let (before, token) = (toks[k - 1].as_str(), toks[k].as_str());
            if token == "["
                && (matches!(before, ")" | "]" | "?")
                    || before.as_bytes()[0].is_ascii_digit()
                    || (is_ident(before) && !keyword(before)))
                && !(before == INDEXED_FIELD && k > 1 && toks[k - 2] == ".")
            {
                return Err(reject(
                    "indexing other than `ctx.params[key]`, whose index is a pointer-width `usize`",
                ));
            }
        }
        allowlisted(code)
    }
    /// Match `\bpub\s+fn\s+execute\s*\(\s*ctx\s*:\s*&\s*AllowitContext\s*\)\s*->\s*allowit\s*::\s*Result(?:\s*<\s*\(\s*\)\s*>)?\s*\{`
    /// at `k`; the index after `{`.
    fn execute_at(chars: &[char], k: usize) -> Option<usize> {
        if !starts(chars, k, "pub") || (k > 0 && word(chars[k - 1])) {
            return None;
        }
        let mut p = k + 3;
        let ws1 = |p: usize| {
            let q = skip_ws(chars, p);
            (q > p).then_some(q)
        };
        let lit = |p: usize, t: &str| starts(chars, p, t).then_some(p + t.chars().count());
        p = ws1(p)?;
        p = lit(p, "fn")?;
        p = ws1(p)?;
        p = lit(p, "execute")?;
        for t in [
            "(",
            "ctx",
            ":",
            "&",
            "AllowitContext",
            ")",
            "->",
            "allowit",
            "::",
            "Result",
        ] {
            p = lit(skip_ws(chars, p), t)?;
        }
        let generic = (|| {
            let mut q = p;
            for t in ["<", "(", ")", ">"] {
                q = lit(skip_ws(chars, q), t)?;
            }
            Some(q)
        })();
        if let Some(q) = generic {
            p = q;
        }
        lit(skip_ws(chars, p), "{")
    }
    /// `allowit::<rule>(ctx,&ctx.params[allowit::<RULE>],?)?` on a whitespace-free statement.
    fn prefix_call(statement: &str) -> Option<String> {
        let rest = statement.strip_prefix("allowit::")?;
        let name_len = rest
            .bytes()
            .take_while(|b| b.is_ascii_lowercase() || *b == b'_')
            .count();
        if name_len == 0 {
            return None;
        }
        let (name, rest) = rest.split_at(name_len);
        let rest = rest.strip_prefix("(ctx,&ctx.params[allowit::")?;
        let key_len = rest
            .bytes()
            .take_while(|b| b.is_ascii_uppercase() || *b == b'_')
            .count();
        if key_len == 0 {
            return None;
        }
        let (key, rest) = rest.split_at(key_len);
        let rest = rest.strip_prefix(']')?;
        let rest = rest.strip_prefix(',').unwrap_or(rest);
        if rest != ")?" || name.to_uppercase() != key || !param_keys().contains(&name) {
            return None;
        }
        Some(name.into())
    }
    /// The standard prefix of lexed code and whether the whole file is canonical-only.
    ///
    /// A file is canonical-only when it is nothing but optional `use allowit::...;` items and
    /// one `execute` whose body is exactly the prefix statements followed by `Ok(())`. Only then
    /// does running the prefix rules on the host equal running the compiled file: any other
    /// item, such as a local `fn Ok` that shadows the prelude, could change what `execute` does.
    pub(crate) fn standard_prefix(code: &str) -> Result<(Vec<String>, bool)> {
        let chars: Vec<char> = code.chars().collect();
        let found: Vec<usize> = (0..chars.len())
            .filter_map(|k| execute_at(&chars, k))
            .collect();
        if found.len() != 1 || words(&chars, &["execute"]).count() != 1 {
            return Err("Policy source must define pub fn execute(ctx: &AllowitContext) -> allowit::Result once".into());
        }
        let body: String = chars[found[0]..].iter().collect();
        let mut rules = vec![];
        let mut consumed = 0;
        for statement in body.split(';') {
            let compact: String = statement.chars().filter(|c| !py_space(*c)).collect();
            match prefix_call(&compact) {
                Some(rule) => {
                    rules.push(rule);
                    consumed += statement.len() + 1;
                }
                None => break,
            }
        }
        let rest: String = body[consumed.min(body.len())..]
            .chars()
            .filter(|c| !py_space(*c))
            .collect();
        let canonical_only = rest == "Ok(())}" && only_allowit_uses_before(&chars, found[0]);
        Ok((rules, canonical_only))
    }
    /// True when the code before `execute` (which ends at `body_start`, after its `{`) is only
    /// `use allowit::...;` items: a use tree of path segments, `::`, `*`, `,` and braces,
    /// without renaming.
    fn only_allowit_uses_before(chars: &[char], body_start: usize) -> bool {
        let Some(signature) = (0..body_start)
            .rev()
            .find(|k| starts(chars, *k, "pub") && execute_at(chars, *k) == Some(body_start))
        else {
            return false;
        };
        let mut k = skip_ws(chars, 0);
        while k < signature {
            if !(starts(chars, k, "use") && !chars.get(k + 3).is_some_and(|c| word(*c))) {
                return false;
            }
            let mut p = skip_ws(chars, k + 3);
            if !(starts(chars, p, "allowit") && !chars.get(p + 7).is_some_and(|c| word(*c))) {
                return false;
            }
            p = skip_ws(chars, p + 7);
            if !starts(chars, p, "::") {
                return false;
            }
            p += 2;
            let tree = p;
            while p < signature && chars[p] != ';' {
                let c = chars[p];
                if !(word(c) || py_space(c) || matches!(c, ':' | '*' | ',' | '{' | '}')) {
                    return false;
                }
                p += 1;
            }
            // A renaming import (`use allowit::x as Ok;`) could shadow a prelude name.
            if words(&chars[tree..p], &["as"]).next().is_some() {
                return false;
            }
            if p >= signature {
                return false;
            }
            k = skip_ws(chars, p + 1);
        }
        k == signature
    }
    /// `\ballowit\s*::\s*(?:r#)?set_state\b` on lexed code.
    pub(crate) fn writes_state(code: &str) -> bool {
        let chars: Vec<char> = code.chars().collect();
        words(&chars, &["allowit"]).any(|(_, end)| {
            let p = skip_ws(&chars, end);
            if !starts(&chars, p, "::") {
                return false;
            }
            let mut q = skip_ws(&chars, p + 2);
            if starts(&chars, q, "r#") {
                q += 2;
            }
            starts(&chars, q, "set_state") && !chars.get(q + 9).is_some_and(|c| word(*c))
        })
    }
}
