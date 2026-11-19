//! The on-chain part of a policy source.
//!
//! The owner writes and signs one policy source. The server evaluates the whole source. A
//! contract that compiles its policy cannot call a server function, so it compiles the part
//! that [`derive`] keeps: the statements that the chain can check by itself. The statements
//! that depend on a server function leave. Nobody writes the derived policy: it is a pure
//! function of the source bytes, so the same source gives the same derived policy, byte for
//! byte, wherever it is derived.
//!
//! The derived policy never denies a request that the whole source allows: every statement
//! that stays runs in the derived policy exactly when it runs in an allowed evaluation of the
//! whole source, on the same values.
//!
//! A statement of `execute` leaves when:
//!
//! 1. it makes a server call, names a server request type, reads owner text with `as_str()`,
//!    which only the server can read, or uses policy state that the server keeps (see
//!    [Policy state](#policy-state)). A server call is a call of a server function, of a
//!    registered package function, or of a local function that makes one;
//! 2. it reads a name that a leaving statement binds, or a name that a leaving statement can
//!    assign: a `mut` name, or a name that its `let` declares without a value;
//! 3. it is an `if`, `match`, `while` or `for` whose condition, matched value, pattern guard
//!    or iterated value does 1 or 2: the whole statement leaves, with every branch;
//! 4. it is a `let` that only reads an owner param, a request field or a constant, and only
//!    leaving statements read its name;
//! 5. it comes after a leaving statement that holds a `return` other than `return Err(...)`:
//!    every later statement of `execute` leaves, and the derived `execute` ends with `Ok(())`.
//!    A leaving statement that holds such a `return`, a `break` or a `continue` takes its
//!    enclosing loops with it.
//!
//! A local function that makes a server call leaves. A local function or constant that only
//! leaving code named leaves. The server request types leave from the `use` items. A statement
//! that stays keeps its bytes; a leaving statement takes its own lines and the comment lines
//! directly above it.
//!
//! A branch statement whose value can be used is not split: an `if` with an `else`, a `match`
//! or a block that ends with `;`, or that is the last expression of `execute`, stays whole or
//! leaves whole.
//!
//! # Policy state
//!
//! A policy may keep its own state: `allowit::get_state(ctx, "key")` reads the value that the
//! last admitted operation wrote, and `allowit::set_state(ctx, "key", value)` writes when the
//! operation is admitted. Each key has one keeper, so a statement reads the same value on the
//! chain and on the server:
//!
//! - the chain keeps a key when a statement that uses it stays and no statement that writes it
//!   leaves. The contract stores the key, the kept statements read and write it there, and
//!   the server reads it from the contract when it evaluates the whole source, also in a
//!   statement that left;
//! - the server keeps a key when a statement that writes it leaves. Then every statement that
//!   reads or writes the key leaves too (rule 1), and the contract never stores it. A key
//!   with no write that only leaving statements read is the server's as well.
//!
//! A call of a local function that writes a key counts as a write of that key, also through
//! another local function. A function that stands inside another one writes only where it is
//! called. A key whose name
//! starts with `day_` lives one daily window; any other key lives until a later write replaces
//! it ([`StateEntry::daily`]). [`Derived::state`] lists every key with its keeper.
//!
//! A read of a standard figure, such as `allowit::get_state(ctx, "day_spent")`, stays: the
//! contract keeps each figure that a kept statement reads ([`Derived::figures`]). No policy
//! writes a standard figure. No policy reads the stored day, `allowit::get_state(ctx, "day")`:
//! a contract wallet stores no day, and it keeps its daily figures in a window of its own, so
//! the day has no value that the chain and the server share. A policy reads the daily figures
//! instead.
//!
//! A value without a stated type can take its type from the code that uses it, and Rust gives a
//! number `i32` when no code does. A statement that leaves can be that code, so the on-chain
//! part would compute with another type. The rule names the forms of a value whose type is
//! fixed: a literal with a fixed type (a text, a bool, a number with a suffix), a field of the
//! request, an owner param read whole, a call of a library function with a fixed return type
//! or of an integer type's `from` or `try_from` (with at most `?` after it), a range with a
//! side of a fixed type, and a name of a fixed type (with at most `.clone()` or `.as_str()`).
//! A name bound without a stated type (by `let`, `for`, `if let`, `while let` or a `match`
//! arm) from any other form is followed like a value of the server: when a removed statement
//! mentions it, or a name that one statement uses together with it, a kept statement that
//! reads those names leaves too. A form that the rule does not know counts as open, so it
//! moves more to the server, never less. A stated type (`let n: u64 = ...`) keeps the
//! statements on chain.
//!
//! The derivation reads tokens, not a syntax tree, and ends each statement where the Rust
//! parser ends it: a block-like statement (a branch, a loop, a block, a macro with braces, a
//! `const` block) ends at its last `}` unless `.` or `?` follows, and any other statement ends
//! at its `;`. A match arm whose body is block-like or a `const` block ends at its last `}` in
//! the same way. So each
//! statement, each item and each arm of a block is a step of its own. The on-chain part states
//! `warn` where the source states the lint level `deny` or `forbid`: it leaves code out, and a
//! lint level changes nothing that the policy does.
//!
//! The contract must know each key before it runs, and Rust source cannot pass a state function
//! as a value. So [`derive`] refuses a state call whose key is not a string literal of 1 to 32
//! ASCII letters, digits or `_`, a write of a standard figure and a use of the stored day.
//!
//! [`derive`] also refuses a source that the rule cannot follow by its names, when a statement
//! or a local item of it leaves: one that takes a mutable reference (`&mut`, `ref mut`), which
//! lets a statement assign a name that it does not mention; one that declares two local
//! functions or constants with one name; one that declares a local `Ok` or `Err`; and one in
//! which the parser cannot tell where a statement ends, which it shows by a `let` or an item
//! right after a `}` of that statement. A source of which only an unused server import leaves
//! has no such statement, so it is not refused.
use super::{Result, gate, param_keys, source_shape};
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap, HashSet},
};

/// Why a statement runs only on the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The statement makes a server call.
    ServerCall,
    /// The statement reads a value that a server call gave.
    ServerValue,
    /// The statement reads or writes policy state that the server keeps.
    StoredState,
    /// The statement is in a branch whose condition depends on a server function or on
    /// policy state that the server keeps.
    ServerBranch,
    /// The statement comes after an early exit that depends on a server function.
    AfterServerExit,
    /// The statement is in a branch statement whose value can be used, which leaves whole.
    NotSplit,
}

impl Cause {
    /// The stable code of this cause.
    pub fn code(self) -> &'static str {
        match self {
            Self::ServerCall => "server_call",
            Self::ServerValue => "server_value",
            Self::StoredState => "stored_state",
            Self::ServerBranch => "server_branch",
            Self::AfterServerExit => "after_server_exit",
            Self::NotSplit => "not_split",
        }
    }

    /// Whether a check with this cause leaves because of where it stands, not because of
    /// what it reads.
    pub fn movable(self) -> bool {
        matches!(
            self,
            Self::ServerBranch | Self::AfterServerExit | Self::NotSplit
        )
    }
}

/// One deterministic check of the source that only the server enforces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerOnly {
    /// The line of the check in the owner's source, from 1.
    pub line: u32,
    /// The canonical rule, such as `payment_limit`; `assert` for an assertion macro; or `deny`
    /// for a `return Err(...)`, a `panic!` or an `unreachable!`.
    pub check: String,
    /// Why it left.
    pub cause: Cause,
}

impl ServerOnly {
    /// Whether the owner can move this check so that the chain enforces it too. A rule or an
    /// assertion that leaves because of where it stands can move. A plain denial is the verdict
    /// of its branch, so it stays where it is.
    pub fn movable(&self) -> bool {
        self.cause.movable() && self.check != "deny"
    }
}

/// The on-chain part of one policy source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derived {
    /// The derived policy source. It equals the owner's source when the source names no server
    /// function and no server request type.
    pub source: String,
    /// The line ranges of the owner's source that left, each inclusive and from 1.
    pub left: Vec<(u32, u32)>,
    /// The deterministic checks that left, in source order.
    pub server_only: Vec<ServerOnly>,
    /// The deterministic checks that stay, in source order and without repetition. An empty
    /// list means that the contract enforces nothing of the policy beyond its signatures.
    pub kept: Vec<String>,
    /// The cumulated figures that the kept statements read, in a fixed order and without
    /// repetition: those of the kept canonical rules, and those that a kept statement reads
    /// with `allowit::get_state`. A contract wallet keeps these figures, and the server
    /// evaluates the whole source on the wallet's values of them. The state map holds every
    /// standard figure, as [`AllowitStandardState::write`](crate::AllowitStandardState::write)
    /// writes it: a daily figure of the wallet's current window, and zero (false for the flag)
    /// when that window has ended. A key of the policy's own state reads as absent instead
    /// ([`StateEntry::daily`]).
    pub figures: Vec<Figure>,
    /// The policy's own state keys, sorted by key and without repetition, each with its
    /// keeper. An empty list means that the policy keeps no state of its own.
    pub state: Vec<StateEntry>,
}

/// Who keeps one state key of a policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keeper {
    /// The contract stores the key. A statement that uses it stays in the on-chain part, and
    /// so does every statement that writes it.
    Chain,
    /// The server stores the key. Every statement that reads or writes it runs only on the
    /// server: a statement that writes it left, or the key has no write and only leaving
    /// statements read it.
    Server,
}

impl Keeper {
    /// The stable code of this keeper.
    pub fn code(self) -> &'static str {
        match self {
            Self::Chain => "chain",
            Self::Server => "server",
        }
    }
}

/// One state key of a policy: a key of its `allowit::get_state` and `allowit::set_state`
/// calls that is no standard figure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateEntry {
    /// The key, as the source names it.
    pub key: String,
    /// Whether the value lives one daily window: the key starts with [`DAILY_PREFIX`]. The
    /// window opens with the first write after the last window ended, and the value reads as
    /// absent when it has ended. Any other value lives until a later write replaces it.
    ///
    /// The keeper enforces the window. The evaluator of this crate reads the state map as it
    /// gets it and carries every key forward: the contract and the server each leave an
    /// ended value out of the state that they give to an evaluation.
    pub daily: bool,
    /// Who stores the value.
    pub keeper: Keeper,
}

/// The prefix of a state key whose value lives one daily window.
pub const DAILY_PREFIX: &str = "day_";
/// The most bytes of a state key.
pub const MAX_STATE_KEY_BYTES: usize = 32;

/// The standard state keys: the figures, and `day`, the day of the last admitted operation. A
/// contract wallet stores no day, so [`derive`] refuses a use of it.
const STANDARD: &[&str] = &[
    allowit::CALLS,
    allowit::SPENT,
    allowit::UNPRICED,
    allowit::DAY_SPENT,
    allowit::DAY_TOTAL,
    allowit::DAY_UNPRICED,
    allowit::DAY_KEY,
];

/// Whether `key` is a standard state key, and no key of the policy's own.
fn standard(key: &str) -> bool {
    STANDARD.contains(&key)
}

/// One cumulated figure that a kept canonical rule reads: a key of the standard state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Figure {
    /// The standard state key, such as `day_spent`.
    pub key: &'static str,
    /// Whether the figure counts one UTC day. A daily figure starts again at zero each day;
    /// a total counts the wallet's whole life.
    pub daily: bool,
}

/// Every figure, in the fixed order of [`Derived::figures`], with the canonical rules that
/// read it.
const FIGURES: &[(Figure, &[&str])] = &[
    (
        Figure {
            key: allowit::CALLS,
            daily: false,
        },
        &["call_limit"],
    ),
    (
        Figure {
            key: allowit::SPENT,
            daily: false,
        },
        &["lifetime_spending_limit"],
    ),
    (
        Figure {
            key: allowit::UNPRICED,
            daily: false,
        },
        &["lifetime_spending_limit"],
    ),
    (
        Figure {
            key: allowit::DAY_SPENT,
            daily: true,
        },
        &["daily_spending_limit"],
    ),
    (
        Figure {
            key: allowit::DAY_TOTAL,
            daily: true,
        },
        &["daily_volume_limit"],
    ),
    (
        Figure {
            key: allowit::DAY_UNPRICED,
            daily: true,
        },
        &["daily_spending_limit", "daily_volume_limit"],
    ),
];

/// The figures that the checks in `kept` read, with the figures in `read`: the standard keys
/// that a kept statement reads with `allowit::get_state`.
fn figures(kept: &[String], read: &BTreeSet<&str>) -> Vec<Figure> {
    FIGURES
        .iter()
        .filter(|(figure, rules)| {
            read.contains(figure.key) || rules.iter().any(|rule| kept.iter().any(|k| k == rule))
        })
        .map(|(figure, _)| *figure)
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Word,
    Literal,
    Lifetime,
    Punct,
}

#[derive(Clone, Copy, Debug)]
struct Token {
    kind: Kind,
    start: usize,
    end: usize,
}

/// The tokens of `source` with their byte ranges. Comments are no tokens.
fn lex(source: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = source.chars().collect();
    let mut at = Vec::with_capacity(chars.len() + 1);
    let mut offset = 0;
    for c in &chars {
        at.push(offset);
        offset += c.len_utf8();
    }
    at.push(offset);
    let n = chars.len();
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let word_end = |mut k: usize| {
        while k < n && word(chars[k]) {
            k += 1;
        }
        k
    };
    let mut tokens = vec![];
    let mut push = |kind, start: usize, end: usize| {
        tokens.push(Token {
            kind,
            start: at[start],
            end: at[end],
        })
    };
    let mut i = 0;
    while i < n {
        let c = chars[i];
        if gate::starts(&chars, i, "//") {
            i = (i..n).find(|k| chars[*k] == '\n').unwrap_or(n);
        } else if gate::starts(&chars, i, "/*") {
            let mut depth = 1;
            i += 2;
            while depth > 0 {
                if i >= n {
                    return Err("Policy source has an unterminated block comment".into());
                }
                if gate::starts(&chars, i, "/*") {
                    depth += 1;
                    i += 2;
                } else if gate::starts(&chars, i, "*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if let Some((after, hashes)) = gate::raw_string(&chars, i) {
            let close: Vec<char> = std::iter::once('"')
                .chain(std::iter::repeat_n('#', hashes))
                .collect();
            let end = (after..=n.saturating_sub(close.len()))
                .find(|k| chars[*k..*k + close.len()] == close[..])
                .ok_or("Policy source has an unterminated raw string")?;
            push(Kind::Literal, i, end + close.len());
            i = end + close.len();
        } else if let Some(end) = gate::string(&chars, i) {
            push(Kind::Literal, i, end);
            i = end;
        } else if let Some(end) = gate::char_literal(&chars, i) {
            push(Kind::Literal, i, end);
            i = end;
        } else if c == '"' {
            return Err("Policy source has an unterminated string".into());
        } else if gate::py_space(c) {
            i += 1;
        } else if c == 'r'
            && chars.get(i + 1) == Some(&'#')
            && chars
                .get(i + 2)
                .is_some_and(|x| x.is_ascii_alphabetic() || *x == '_')
        {
            let end = word_end(i + 2);
            push(Kind::Word, i, end);
            i = end;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let end = word_end(i + 1);
            push(Kind::Word, i, end);
            i = end;
        } else if c.is_ascii_digit() {
            let end = word_end(i + 1);
            push(Kind::Literal, i, end);
            i = end;
        } else if c == '\'' && chars.get(i + 1).is_some_and(|x| word(*x)) {
            let end = word_end(i + 1);
            push(Kind::Lifetime, i, end);
            i = end;
        } else if gate::starts(&chars, i, "..=") {
            push(Kind::Punct, i, i + 3);
            i += 3;
        } else if ["::", "=>", "->", "==", "!=", "&&", "||", ".."]
            .iter()
            .any(|t| gate::starts(&chars, i, t))
        {
            push(Kind::Punct, i, i + 2);
            i += 2;
        } else {
            push(Kind::Punct, i, i + 1);
            i += 1;
        }
    }
    Ok(tokens)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    /// A `use`, `const` or `fn` item.
    Item,
    Let,
    /// A statement that is not split: an expression, a macro call, a `return`, or a branch
    /// statement whose value can be used.
    Whole,
    /// A branch statement whose blocks hold statements of their own.
    Split,
    /// The expression of one `match` arm of a split `match`.
    Arm,
}

#[derive(Clone, Debug)]
struct Node {
    shape: Shape,
    /// First and last token, inclusive. The last token is the `;` when the statement has one.
    first: usize,
    last: usize,
    /// The name of a `fn` or `const` item.
    name: Option<String>,
    is_fn: bool,
    /// The token of the statement's own keyword or first word, after its attributes, its
    /// label and `pub`.
    key: usize,
    /// The brackets of each block of a split statement.
    blocks: Vec<(usize, usize)>,
    /// The names that a `let` binds.
    names: Vec<String>,
    /// The tokens of a `let` initializer, as a half-open range.
    init: Option<(usize, usize)>,
    /// The conditions, matched values, patterns and guards of a split statement.
    headers: Vec<(usize, usize)>,
    /// A statement that a `break` can end: a `while`, `for` or `loop` statement, or a
    /// labeled block.
    is_loop: bool,
    /// A whole statement that holds a block: a branch statement that is not split.
    braced: bool,
    parent: Option<usize>,
    /// The position among the statements of its block. An arm has none.
    place: Option<(usize, usize)>,
    children: Vec<Vec<usize>>,
    arms: Vec<usize>,
}

struct Parsed<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    nodes: Vec<Node>,
    /// The state keys that the server keeps. [`derive`] finds them step by step.
    server_keys: RefCell<HashSet<String>>,
    /// The tokens where each local function or constant is seen: the block that declares it,
    /// or the whole file.
    item_scopes: RefCell<HashMap<String, (usize, usize)>>,
}

/// One call of `allowit::get_state` or `allowit::set_state`.
struct StateUse {
    /// The token of `allowit`.
    at: usize,
    writes: bool,
    key: String,
}

/// The extent of one branch statement.
struct Branch {
    end: usize,
    headers: Vec<(usize, usize)>,
    blocks: Vec<(usize, usize)>,
    arms: Vec<(usize, usize)>,
    is_loop: bool,
    /// An `if` without a final `else`, or a loop: its blocks have no value.
    unit: bool,
}

impl<'a> Parsed<'a> {
    fn text(&self, i: usize) -> &'a str {
        self.tokens
            .get(i)
            .map_or("", |token| &self.source[token.start..token.end])
    }

    /// Token `i` as a name: a word, without its raw prefix.
    fn word(&self, i: usize) -> Option<&'a str> {
        let token = self.tokens.get(i)?;
        (token.kind == Kind::Word).then(|| {
            let text = &self.source[token.start..token.end];
            text.strip_prefix("r#").unwrap_or(text)
        })
    }

    /// The token that closes the bracket at `open`.
    fn close(&self, open: usize) -> Option<usize> {
        let mut depth = 0i64;
        for i in open..self.tokens.len() {
            match self.text(i) {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                return Some(i);
            }
        }
        None
    }

    /// The first token in `from..end` outside every bracket whose text is one of `stops`.
    fn find(&self, from: usize, end: usize, stops: &[&str]) -> Option<usize> {
        let mut depth = 0i64;
        for i in from..end {
            let text = self.text(i);
            if depth == 0 && stops.contains(&text) {
                return Some(i);
            }
            match text {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                return None;
            }
        }
        None
    }

    /// The `{` that opens the block after the condition or iterated value that starts at
    /// `from`. A `let` in the condition has a pattern, which can hold braces.
    fn block_after(&self, from: usize, end: usize) -> Option<usize> {
        let mut depth = 0i64;
        let mut i = from;
        while i < end {
            match self.text(i) {
                "let" if depth == 0 => i = self.find(i + 1, end, &["="])?,
                "(" | "[" => depth += 1,
                ")" | "]" => depth -= 1,
                "{" if depth == 0 => return Some(i),
                ";" if depth == 0 => return None,
                _ => {}
            }
            i += 1;
        }
        None
    }

    /// Whether token `i`, the one after a closed block, continues an expression. Then the
    /// block was no body of a statement: it was a value, such as the condition of
    /// `while { true } { ... }`. Such a statement is not split.
    fn continues(&self, i: usize) -> bool {
        matches!(
            self.text(i),
            "{" | "."
                | ".."
                | "..="
                | "?"
                | "&&"
                | "||"
                | "=="
                | "!="
                | "+"
                | "-"
                | "*"
                | "/"
                | "%"
                | "<"
                | ">"
                | "="
                | "&"
                | "|"
                | "^"
                | "as"
        )
    }

    /// The `{` that opens the body after the header that starts at `from`. A block can also
    /// stand as an operand of the header, as in `if { a } { ... }` or `if x == { 1 } { ... }`:
    /// a `{` opens an operand at the start of the header and after an operator, and the body
    /// after a name, a literal or a closing bracket. After `..` it opens the body, as the Rust
    /// parser reads a header: `for i in 0.. { ... }`.
    fn body_after(&self, from: usize, end: usize) -> Option<usize> {
        let mut depth = 0i64;
        let mut i = from;
        // The token after the generic arguments of a path: a `{` there opens the body.
        let mut after_generics = None;
        while i < end {
            // A branch or a loop inside the header is a whole value with its own blocks, as
            // in `if ok && loop { break true; } { ... }`.
            let nested = self.tokens[i].kind == Kind::Word
                && matches!(self.text(i), "if" | "match" | "while" | "for" | "loop");
            if depth == 0 && nested {
                i = self.block_like_end(i, end)? + 1;
                continue;
            }
            match self.text(i) {
                "let" if depth == 0 => i = self.find(i + 1, end, &["="])?,
                // The generic arguments of a path, as in `u64::MAX::<>`: their `>` is no
                // operator.
                "::" if self.text(i + 1) == "<" => {
                    let next = self.after_generics(i + 1)?;
                    after_generics = Some(next);
                    i = next - 1;
                }
                "(" | "[" => depth += 1,
                ")" | "]" => depth -= 1,
                "{" if depth == 0 => {
                    let operand = after_generics != Some(i)
                        && (i == from
                            || matches!(
                                self.text(i - 1),
                                "if" | "while"
                                    | "match"
                                    | "in"
                                    | "return"
                                    | "const"
                                    | "=="
                                    | "!="
                                    | "&&"
                                    | "||"
                                    | "<"
                                    | ">"
                                    | "="
                                    | "+"
                                    | "-"
                                    | "*"
                                    | "/"
                                    | "%"
                                    | "!"
                                    | "&"
                                    | "|"
                                    | "^"
                                    | ","
                                    | "=>"
                                    | ":"
                            ));
                    if !operand {
                        return Some(i);
                    }
                    i = self.close(i)?;
                }
                ";" if depth == 0 => return None,
                _ => {}
            }
            i += 1;
        }
        None
    }

    /// The last token of the statement that token `p` starts, when it is a macro with braces,
    /// such as `assert! { ok }`, or a `const { ... }` block. Such a statement ends at its `}`
    /// with no `;`, unless a method call or `?` continues it: that is the rule of the Rust
    /// parser. Only the name of a macro counts, plain or raw (`r#assert`):
    /// `if ! { ok } { ... }` negates a block.
    fn braced_end(&self, p: usize, end: usize) -> Option<usize> {
        let macro_name = self
            .word(p)
            .is_some_and(|name| gate::MACROS.contains(&name));
        let open = match (self.text(p), self.text(p + 1), self.text(p + 2)) {
            ("const", "{", _) => p + 1,
            (_, "!", "{") if macro_name => p + 2,
            _ => return None,
        };
        self.close(open)
            .filter(|last| *last < end && !matches!(self.text(last + 1), "." | "?"))
    }

    /// The `{` that opens the body of the `for` at token `p`. Its pattern can hold braces, as
    /// in `for S { a, .. } in xs { ... }`, so the body comes after the value that follows `in`.
    fn for_body(&self, p: usize, end: usize) -> Option<usize> {
        let at = self.find(p + 1, end, &["in"])?;
        self.body_after(at + 1, end)
    }

    /// The last token of the branch statement whose keyword or `{` is token `p`, also when its
    /// header holds a block, which [`Parsed::branch`] does not split.
    fn block_like_end(&self, p: usize, end: usize) -> Option<usize> {
        match self.text(p) {
            "if" => {
                let mut q = p;
                loop {
                    let close = self.close(self.body_after(q + 1, end)?)?;
                    if self.text(close + 1) != "else" {
                        return Some(close);
                    }
                    match self.text(close + 2) {
                        "if" => q = close + 2,
                        "{" => return self.close(close + 2),
                        _ => return None,
                    }
                }
            }
            "match" | "while" => self.close(self.body_after(p + 1, end)?),
            "for" => self.close(self.for_body(p, end)?),
            "loop" if self.text(p + 1) == "{" => self.close(p + 1),
            "{" => self.close(p),
            _ => None,
        }
    }

    /// The branch statement whose keyword or `{` is token `p`. A condition, a matched value
    /// or an iterated value holds at least one token before its block.
    fn branch(&self, p: usize, end: usize) -> Option<Branch> {
        let mut branch = Branch {
            end: p,
            headers: vec![],
            blocks: vec![],
            arms: vec![],
            is_loop: false,
            unit: false,
        };
        match self.text(p) {
            "if" => {
                let mut q = p;
                loop {
                    let open = self.block_after(q + 1, end).filter(|open| *open > q + 1)?;
                    let close = self.close(open)?;
                    branch.headers.push((q + 1, open));
                    branch.blocks.push((open, close));
                    branch.end = close;
                    if self.text(close + 1) != "else" {
                        branch.unit = true;
                        break;
                    }
                    if self.text(close + 2) == "if" {
                        q = close + 2;
                    } else if self.text(close + 2) == "{" {
                        let last = self.close(close + 2)?;
                        branch.blocks.push((close + 2, last));
                        branch.end = last;
                        break;
                    } else {
                        return None;
                    }
                }
            }
            "match" => {
                let open = self.block_after(p + 1, end).filter(|open| *open > p + 1)?;
                let close = self.close(open)?;
                branch.headers.push((p + 1, open));
                branch.end = close;
                let mut a = open + 1;
                while a < close {
                    let arrow = self.find(a, close, &["=>"])?;
                    branch.headers.push((a, arrow));
                    let body = arrow + 1;
                    let block_end = (self.text(body) == "{")
                        .then(|| self.close(body))
                        .flatten()
                        .filter(|c| !self.continues(c + 1));
                    a = match block_end {
                        Some(c) => {
                            branch.blocks.push((body, c));
                            c + 1
                        }
                        None => {
                            // A block-like body, such as `if ok { .. } else { .. }`, needs no
                            // comma: it ends at its last `}` unless `.` or `?` continues it,
                            // as in Rust.
                            let labeled = self.tokens[body].kind == Kind::Lifetime
                                && self.text(body + 1) == ":";
                            let start = if labeled { body + 2 } else { body };
                            let block_like = matches!(
                                self.text(start),
                                "if" | "match" | "loop" | "while" | "for" | "{"
                            );
                            // So does a `const` block. A macro with braces needs a comma there.
                            let braced = (self.text(start) == "const"
                                && self.text(start + 1) == "{")
                                .then(|| self.close(start + 1))
                                .flatten();
                            let exact = block_like
                                .then(|| self.block_like_end(start, close))
                                .flatten()
                                .or(braced)
                                .filter(|last| {
                                    *last < close && !matches!(self.text(last + 1), "." | "?")
                                });
                            let stop = match exact {
                                Some(last) => last + 1,
                                // A `,` between generic arguments ends no arm.
                                None => self.argument_end(body, close).unwrap_or(close),
                            };
                            if stop == body {
                                return None;
                            }
                            branch.arms.push((body, stop));
                            stop
                        }
                    };
                    if self.text(a) == "," {
                        a += 1;
                    }
                }
            }
            "while" | "for" => {
                // The pattern of a `for` can hold braces: its value starts after `in`.
                let from = match self.text(p) {
                    "for" => self.find(p + 1, end, &["in"])? + 1,
                    _ => p + 1,
                };
                let open = self.block_after(from, end).filter(|open| *open > from)?;
                let close = self.close(open)?;
                branch.headers.push((p + 1, open));
                branch.blocks.push((open, close));
                branch.end = close;
                branch.is_loop = true;
                branch.unit = true;
            }
            "loop" | "{" => {
                let open = if self.text(p) == "loop" { p + 1 } else { p };
                if self.text(open) != "{" {
                    return None;
                }
                let close = self.close(open)?;
                branch.blocks.push((open, close));
                branch.end = close;
                branch.is_loop = self.text(p) == "loop";
                branch.unit = branch.is_loop;
            }
            _ => return None,
        }
        (branch.end < end).then_some(branch)
    }

    fn node(&mut self, shape: Shape, first: usize, last: usize, parent: Option<usize>) -> usize {
        self.nodes.push(Node {
            shape,
            first,
            last,
            name: None,
            is_fn: false,
            key: first,
            blocks: vec![],
            names: vec![],
            init: None,
            headers: vec![],
            is_loop: false,
            braced: false,
            parent,
            place: None,
            children: vec![],
            arms: vec![],
        });
        self.nodes.len() - 1
    }

    /// The names that the pattern in `from..to` binds: every name that is no path segment,
    /// call, struct name or macro. A name that starts with an uppercase letter counts too: a
    /// binding may carry the name of a type, and a unit variant read as a name only makes more
    /// statements leave.
    fn bound(&self, from: usize, to: usize) -> Vec<String> {
        (from..to)
            .filter_map(|i| {
                let name = self.word(i)?;
                // The keywords of a pattern, as written: `r#mut` is a name.
                let free = !matches!(self.text(i), "mut" | "ref" | "_")
                    && self.text(i.wrapping_sub(1)) != "::"
                    && !matches!(self.text(i + 1), "(" | "::" | "{" | "!");
                free.then(|| name.to_owned())
            })
            .collect()
    }

    /// Parse the statements from token `from` to the closing bracket `close`. `unit` says that
    /// the block has no value, so each of its statements can leave by itself. `parent` and
    /// `slot` name the split statement and the block that hold these statements.
    fn statements(
        &mut self,
        from: usize,
        close: usize,
        unit: bool,
        parent: Option<usize>,
        slot: usize,
    ) -> Vec<usize> {
        let mut found = vec![];
        let mut p = from;
        while p < close {
            let first = p;
            while self.text(p) == "#" && self.text(p + 1) == "[" {
                match self.close(p + 1) {
                    Some(end) if end + 1 < close => p = end + 1,
                    _ => break,
                }
            }
            let labeled =
                self.tokens[p].kind == Kind::Lifetime && self.text(p + 1) == ":" && p + 2 < close;
            if labeled {
                p += 2;
            }
            let item = if self.text(p) == "pub" { p + 1 } else { p };
            let is_fn = self.text(item) == "fn"
                || (self.text(item) == "const" && self.text(item + 1) == "fn");
            // The end of a statement that is not block-like: its `;`, or the end of the block.
            let plain = |parsed: &Self| match parsed.find(p, close, &[";"]) {
                Some(semi) => semi,
                None => close - 1,
            };
            let id = if is_fn {
                let name = if self.text(item) == "fn" {
                    item + 1
                } else {
                    item + 2
                };
                let body = self
                    .find(name, close, &["{", ";"])
                    .filter(|i| self.text(*i) == "{")
                    .and_then(|open| self.close(open))
                    .filter(|end| *end < close);
                let last = body.unwrap_or_else(|| plain(self));
                let id = self.node(Shape::Item, first, last, parent);
                self.nodes[id].name = self.word(name).map(str::to_owned);
                self.nodes[id].is_fn = true;
                id
            } else if let Some(end) = self.braced_end(p, close) {
                // A macro with braces or a `const` block is a statement that needs no `;`.
                let semi = self.text(end + 1) == ";" && end + 1 < close;
                let last = if semi { end + 1 } else { end };
                self.node(Shape::Whole, first, last, parent)
            } else if self.text(item) == "use"
                || (self.text(item) == "const" && self.word(item + 1).is_some())
            {
                // A `const` block that a method call continues is no item: it is a statement.
                let last = plain(self);
                let id = self.node(Shape::Item, first, last, parent);
                // `const _` binds no name.
                if self.text(item) == "const" {
                    self.nodes[id].name = self
                        .word(item + 1)
                        .filter(|name| *name != "_")
                        .map(str::to_owned);
                }
                id
            } else if self.text(p) == "let" {
                let last = plain(self);
                let id = self.node(Shape::Let, first, last, parent);
                let pattern_end = self.find(p + 1, last, &["=", ":"]).unwrap_or(last);
                self.nodes[id].names = self.bound(p + 1, pattern_end);
                if let Some(equals) = self.find(p + 1, last, &["="]) {
                    self.nodes[id].init = Some((equals + 1, last));
                }
                id
            } else {
                // A block-like statement ends at its last `}`, as the Rust parser ends it: only
                // `.` or `?` after that `}` continues the expression, so a `{`, an operator or
                // a declaration after it starts the next statement. `block_like_end` finds the
                // body after a header that holds a block, as in `while { ok } { ... }`.
                let exact = self.block_like_end(p, close).filter(|last| *last < close);
                let continued = |last: usize| matches!(self.text(last + 1), "." | "?");
                // `branch` takes the first block of a header as the body. It splits the
                // statement only when that is the statement's own last block.
                let branch = self
                    .branch(p, close)
                    .filter(|branch| Some(branch.end) == exact && !continued(branch.end));
                let unsplit = || exact.filter(|last| !continued(*last));
                match branch {
                    None => match unsplit() {
                        Some(end) => {
                            let semi = self.text(end + 1) == ";" && end + 1 < close;
                            let last = if semi { end + 1 } else { end };
                            let id = self.node(Shape::Whole, first, last, parent);
                            self.nodes[id].is_loop =
                                labeled || matches!(self.text(p), "while" | "for" | "loop");
                            self.nodes[id].braced = true;
                            id
                        }
                        None => {
                            let last = plain(self);
                            self.node(Shape::Whole, first, last, parent)
                        }
                    },
                    Some(branch) => {
                        let semi = self.text(branch.end + 1) == ";" && branch.end + 1 < close;
                        let last = if semi { branch.end + 1 } else { branch.end };
                        let tail = last + 1 == close && !semi;
                        // A branch statement with no value: one that cannot have a value, one
                        // that stands before other statements without `;`, or the last
                        // statement of a block that has no value.
                        let split = branch.unit || (!semi && (!tail || unit));
                        if split {
                            let id = self.node(Shape::Split, first, last, parent);
                            self.nodes[id].headers = branch.headers;
                            self.nodes[id].is_loop = branch.is_loop || labeled;
                            for (slot, (open, close)) in branch.blocks.into_iter().enumerate() {
                                let children =
                                    self.statements(open + 1, close, true, Some(id), slot);
                                self.nodes[id].children.push(children);
                                self.nodes[id].blocks.push((open, close));
                            }
                            for (from, to) in branch.arms {
                                let arm = self.node(Shape::Arm, from, to - 1, Some(id));
                                self.nodes[id].arms.push(arm);
                            }
                            id
                        } else {
                            let id = self.node(Shape::Whole, first, last, parent);
                            self.nodes[id].is_loop = branch.is_loop;
                            self.nodes[id].braced = true;
                            id
                        }
                    }
                }
            };
            self.nodes[id].place = Some((slot, found.len()));
            self.nodes[id].key = item;
            p = self.nodes[id].last + 1;
            found.push(id);
        }
        found
    }

    /// Whether token `i` names a local value, function or constant: a word that is no field,
    /// method or path segment.
    fn names(&self, i: usize, wanted: &HashSet<String>) -> bool {
        self.word(i).is_some_and(|name| {
            wanted.contains(name)
                && !matches!(self.text(i.wrapping_sub(1)), "." | "::")
                && self.text(i + 1) != "::"
        })
    }

    /// Whether token `i` names one of the local functions or constants in `wanted` where that
    /// item is seen: outside the block that declares it, the name is another item, such as the
    /// `Some` of the language.
    fn names_item(&self, i: usize, wanted: &HashSet<String>) -> bool {
        self.names(i, wanted)
            && self.word(i).is_some_and(|name| {
                self.item_scopes
                    .borrow()
                    .get(name)
                    .is_none_or(|(from, to)| (*from..=*to).contains(&i))
            })
    }

    /// The text of the token before token `at`, past the attributes (`#[...]`) that stand
    /// right before it, or "" at the start of the file.
    fn before_attributes(&self, at: usize) -> &'a str {
        let mut i = at;
        while i > 0 && self.text(i - 1) == "]" {
            let mut depth = 0i64;
            let mut j = i - 1;
            loop {
                match self.text(j) {
                    "]" => depth += 1,
                    "[" => depth -= 1,
                    _ => {}
                }
                if depth == 0 || j == 0 {
                    break;
                }
                j -= 1;
            }
            if depth != 0 || j == 0 || self.text(j - 1) != "#" {
                break;
            }
            i = j - 1;
        }
        if i == 0 { "" } else { self.text(i - 1) }
    }

    fn mentions(&self, from: usize, to: usize, wanted: &HashSet<String>) -> bool {
        !wanted.is_empty() && (from..to).any(|i| self.names(i, wanted))
    }

    /// Whether token `i` starts the path `allowit::<name>`.
    fn allowit_path(&self, i: usize) -> Option<&'a str> {
        (self.word(i) == Some("allowit")
            && self.text(i + 1) == "::"
            && self.text(i.wrapping_sub(1)) != "::")
            .then(|| self.word(i + 2))
            .flatten()
    }

    /// Whether token `i` starts a server call or names a server request type: a server
    /// function, a server request type, or a call of a two-segment path that the portable
    /// profile does not hold, which is a registered package function.
    fn server_at(&self, i: usize) -> bool {
        let Some(name) = self.word(i) else {
            return false;
        };
        if gate::SERVER_TYPES.contains(&name) {
            return true;
        }
        if let Some(function) = self.allowit_path(i) {
            return gate::SERVER_FUNCTIONS.contains(&function) || self.state_at(i);
        }
        let package = self.text(i + 1) == "::"
            && self.text(i + 3) == "("
            && self.text(i.wrapping_sub(1)) != "::"
            && name.starts_with(|c: char| c.is_ascii_lowercase());
        package
            && self
                .word(i + 2)
                .is_some_and(|operation| !gate::portable_path(&format!("{name}::{operation}")))
    }

    /// Whether token `i` reads the text of a portable `String`: `text.as_str()`. Only server
    /// policy source has this method, so a contract cannot compile a statement that holds it.
    fn server_text_at(&self, i: usize) -> bool {
        self.word(i) == Some("as_str")
            && self.text(i.wrapping_sub(1)) == "."
            && self.text(i + 1) == "("
    }

    /// The token after the generic arguments that the `<` at token `open` starts, as in
    /// `get_state::<Option<u64>>`.
    fn after_generics(&self, open: usize) -> Option<usize> {
        let mut depth = 0i64;
        let mut i = open;
        loop {
            match self.text(i) {
                "<" => depth += 1,
                ">" => depth -= 1,
                // A type can hold brackets: a tuple, an array with its length.
                "(" | "[" | "{" => i = self.close(i)?,
                "" | ")" | "]" | "}" | ";" => return None,
                _ => {}
            }
            i += 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }

    /// The `,` that ends the call argument that starts at token `from`, before the closing
    /// bracket `end`. A `,` in brackets or between the generic arguments of a path, as in
    /// `Ok::<_, AllowitError>(ctx)`, ends no argument.
    fn argument_end(&self, from: usize, end: usize) -> Option<usize> {
        let mut i = from;
        while i < end {
            match self.text(i) {
                "," => return Some(i),
                "(" | "[" | "{" => i = self.close(i)?,
                "::" if self.text(i + 1) == "<" => i = self.after_generics(i + 1)? - 1,
                _ => {}
            }
            i += 1;
        }
        None
    }

    /// The state call that token `i` starts, `allowit::get_state(ctx, "key")` or
    /// `allowit::set_state(ctx, "key", value)`: whether it writes, and its key. `None` for
    /// another token and for a call whose key is no string literal.
    fn state_use(&self, i: usize) -> Option<(bool, &'a str)> {
        let writes = match self.allowit_path(i)? {
            "get_state" => false,
            "set_state" => true,
            _ => return None,
        };
        // An explicit type: `get_state::<u64>(...)`.
        let mut open = i + 3;
        if self.text(open) == "::" && self.text(open + 1) == "<" {
            open = self.after_generics(open + 1)?;
        }
        if self.text(open) != "(" {
            return None;
        }
        let close = self.close(open)?;
        // The key is the second argument, and the literal alone.
        let key = self.argument_end(open + 1, close)? + 1;
        let token = self
            .tokens
            .get(key)
            .filter(|token| token.kind == Kind::Literal)?;
        let ends = key + 1 == close || (self.text(key + 1) == "," && key + 1 < close);
        // A plain string, or a raw string such as `r"key"` or `r#"key"#`.
        let text = &self.source[token.start..token.end];
        let raw = text.strip_prefix('r').map(|text| text.trim_matches('#'));
        raw.unwrap_or(text)
            .strip_prefix('"')
            .and_then(|text| text.strip_suffix('"'))
            .filter(|_| ends)
            .map(|key| (writes, key))
    }

    /// Every state call of the source. A use of `allowit::get_state` or `allowit::set_state`
    /// that is no such call, a key that is no valid literal, a write of a standard figure and a
    /// use of the stored day are refused.
    fn state_uses(&self) -> Result<Vec<StateUse>> {
        let mut uses = vec![];
        for at in 0..self.tokens.len() {
            // Only the library's own functions: a local item may carry one of the two names.
            let Some(name) = self
                .allowit_path(at)
                .filter(|name| matches!(*name, "get_state" | "set_state"))
            else {
                continue;
            };
            let line = self.line(self.tokens[at].start);
            let Some((writes, key)) = self.state_use(at) else {
                return Err(format!(
                    "A policy for a contract wallet names each state key as a string literal: `allowit::{name}(ctx, \"key\"{})` (line {line}). The wallet must know each key before it runs.",
                    if name == "set_state" { ", value" } else { "" }
                ));
            };
            let valid = !key.is_empty()
                && key.len() <= MAX_STATE_KEY_BYTES
                && key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_');
            if !valid {
                return Err(format!(
                    "The state key {key:?} (line {line}) is not 1 to {MAX_STATE_KEY_BYTES} ASCII letters, digits or `_`."
                ));
            }
            if key == allowit::DAY_KEY {
                return Err(format!(
                    "A policy cannot use the stored day \"day\" (`allowit::{name}`, line {line}): a contract wallet stores no day and keeps its daily figures in a window of its own. Read the daily figures instead, such as \"day_spent\"."
                ));
            }
            if writes && standard(key) {
                return Err(format!(
                    "A policy cannot write the standard figure {key:?} (`allowit::set_state`, line {line}): the wallet keeps it. Use another key."
                ));
            }
            uses.push(StateUse {
                at,
                writes,
                key: key.to_owned(),
            });
        }
        Ok(uses)
    }

    /// Whether token `i` starts a call that reads or writes policy state that the server
    /// keeps.
    fn state_at(&self, i: usize) -> bool {
        self.state_use(i)
            .is_some_and(|(_, key)| self.server_keys.borrow().contains(key))
    }

    fn server(&self, from: usize, to: usize, helpers: &HashSet<String>) -> bool {
        (from..to)
            .any(|i| self.server_at(i) || self.server_text_at(i) || self.names_item(i, helpers))
    }

    /// Whether the tokens make a server call other than a use of state that the server keeps.
    /// `callers` are the local functions that make one.
    fn calls(&self, from: usize, to: usize, callers: &HashSet<String>) -> bool {
        (from..to).any(|i| {
            (self.server_at(i) && !self.state_at(i))
                || self.server_text_at(i)
                || self.names_item(i, callers)
        })
    }

    /// The deterministic check that token `i` starts, if any.
    fn check_at(&self, i: usize) -> Option<String> {
        if let Some(function) = self.allowit_path(i) {
            return param_keys()
                .contains(&function)
                .then(|| function.to_owned());
        }
        let name = self.word(i)?;
        if self.text(i + 1) == "!" && self.text(i + 2) != "=" {
            return match name {
                "assert" | "assert_eq" | "assert_ne" => Some("assert".into()),
                "panic" | "unreachable" => Some("deny".into()),
                _ => None,
            };
        }
        self.denies_at(i).then(|| "deny".into())
    }

    /// Whether token `i` starts a plain denial: `return Err(...)` that ends its statement or
    /// its arm. `return Err(...).or(Ok(()))` is no denial.
    fn denies_at(&self, i: usize) -> bool {
        self.word(i) == Some("return")
            && self.word(i + 1) == Some("Err")
            && self.text(i + 2) == "("
            && self
                .close(i + 2)
                .is_some_and(|close| matches!(self.text(close + 1), ";" | "," | "}" | ""))
    }

    /// Whether the tokens of a leaving statement hold a `return` that can end `execute` with
    /// an approval.
    fn returns(&self, from: usize, to: usize) -> bool {
        self.steps(from, to)
            .into_iter()
            .any(|i| self.word(i) == Some("return") && !self.denies_at(i))
    }

    /// The tokens in `from..to` that are steps of the statement itself: those outside the
    /// body of every function that the statement declares. A `return` or a `break` in such a
    /// body ends that function, not `execute`.
    fn steps(&self, from: usize, to: usize) -> Vec<usize> {
        let mut steps = vec![];
        let mut i = from;
        while i < to {
            steps.push(i);
            // The keyword itself: a raw identifier `r#fn` is a name.
            let body = (self.text(i) == "fn")
                .then(|| self.find(i, to, &["{", ";"]))
                .flatten()
                .filter(|open| self.text(*open) == "{")
                .and_then(|open| self.close(open))
                .filter(|close| *close < to);
            i = body.map_or(i + 1, |close| close + 1);
        }
        steps
    }

    /// Whether the tokens hold a `break` or `continue` of a loop that encloses them.
    fn breaks(&self, from: usize, to: usize) -> bool {
        let mut loops: Vec<bool> = vec![];
        // The `{` of each loop body. A block in the header of a loop, such as the iterated
        // value of `for _ in { ... } {}`, runs outside the loop: an exit there ends an
        // enclosing loop.
        let mut bodies: HashSet<usize> = HashSet::new();
        for i in self.steps(from, to) {
            match self.text(i) {
                "loop" | "while" | "for" if self.tokens[i].kind == Kind::Word => {
                    let body = match self.text(i) {
                        "loop" => Some(i + 1).filter(|open| self.text(*open) == "{"),
                        "for" => self.for_body(i, to),
                        _ => self.body_after(i + 1, to),
                    };
                    bodies.extend(body);
                }
                "{" => loops.push(bodies.contains(&i)),
                "}" => {
                    loops.pop();
                }
                "break" | "continue" if self.tokens[i].kind == Kind::Word => {
                    let labeled =
                        self.tokens.get(i + 1).map(|token| token.kind) == Some(Kind::Lifetime);
                    if labeled || !loops.contains(&true) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Whether the initializer in `from..to` only reads: an owner param, a request field, a
    /// constant or a literal. It checks nothing that an owner states as a rule.
    fn reads_only(&self, from: usize, to: usize, functions: &HashSet<String>) -> bool {
        (from..to).all(|i| {
            let text = self.text(i);
            if matches!(text, "?" | "!" | "=" | "{" | "}") || self.names_item(i, functions) {
                return false;
            }
            let Some(name) = self.word(i) else {
                return true;
            };
            if matches!(
                name,
                "if" | "match"
                    | "loop"
                    | "while"
                    | "for"
                    | "return"
                    | "break"
                    | "continue"
                    | "else"
            ) {
                return false;
            }
            if self.text(i.wrapping_sub(1)) == "." && self.text(i + 1) == "(" {
                return matches!(name, "ok" | "clone");
            }
            match self.allowit_path(i) {
                Some(path) => path == "param" || path.starts_with(|c: char| c.is_ascii_uppercase()),
                None => true,
            }
        })
    }

    fn line(&self, byte: usize) -> u32 {
        1 + self.source[..byte].bytes().filter(|b| *b == b'\n').count() as u32
    }
}

/// What the statements of `execute` share.
struct Scope<'a> {
    /// The statements of the body of `execute`.
    body: &'a [usize],
    /// Every name of `execute` that a later statement can assign.
    mutable: &'a HashSet<String>,
    /// Each name of `execute` whose type a number without a type can leave open, with every
    /// name whose type the same statements can tie to it.
    inferred: &'a HashMap<String, BTreeSet<String>>,
}

/// One change of the source text.
struct Edit {
    start: usize,
    end: usize,
    text: String,
}

/// The lines of the statement in `start..end`, with the comment lines directly above it, when
/// the statement has those lines to itself.
fn own_lines(source: &str, start: usize, end: usize) -> (usize, usize) {
    let line_start = |at: usize| source[..at].rfind('\n').map_or(0, |n| n + 1);
    let blank = |text: &str| text.chars().all(gate::py_space);
    let mut from = start;
    let begin = line_start(start);
    let leads = blank(&source[begin..start]);
    if leads {
        from = begin;
        while from > 0 {
            let above = line_start(from - 1);
            if !source[above..from].trim_start().starts_with("//") {
                break;
            }
            from = above;
        }
    }
    let mut to = end;
    let line_end = source[end..]
        .find('\n')
        .map_or(source.len(), |n| end + n + 1);
    let rest = source[end..line_end].trim();
    if leads && (rest.is_empty() || rest.starts_with("//")) {
        to = line_end;
    }
    (from, to)
}

/// The canonical rules and the host calls that a policy source calls, each in source order and
/// without repetition, as [`derive`] reads the source.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceCalls {
    /// The canonical rules that the source calls as `allowit::<rule>(...)`.
    pub rules: Vec<String>,
    /// The calls of a two-segment path `<namespace>::<function>(...)` that the portable profile
    /// does not hold: the functions of the policy packages and rails that a host registers.
    pub host_calls: Vec<String>,
}

/// The calls of `source` ([`SourceCalls`]). Comments and literals hold no call, a raw identifier
/// names the same function as its plain spelling, and a path that is not called or that is the
/// tail of a longer path is no call. A host takes the rules of its placements from here, so that
/// every reader of a source sees the calls that the derivation sees.
pub fn source_calls(source: &str) -> Result<SourceCalls> {
    let parsed = Parsed {
        source,
        tokens: lex(source)?,
        nodes: vec![],
        server_keys: RefCell::default(),
        item_scopes: RefCell::default(),
    };
    let rules = param_keys();
    let mut calls = SourceCalls::default();
    for i in 0..parsed.tokens.len() {
        if parsed.text(i.wrapping_sub(1)) == "::"
            || parsed.text(i + 1) != "::"
            || parsed.text(i + 3) != "("
        {
            continue;
        }
        let (Some(namespace), Some(function)) = (parsed.word(i), parsed.word(i + 2)) else {
            continue;
        };
        if namespace == "allowit" {
            if rules.contains(&function) && !calls.rules.iter().any(|rule| rule == function) {
                calls.rules.push(function.to_owned());
            }
            continue;
        }
        let path = format!("{namespace}::{function}");
        if namespace.starts_with(|c: char| c.is_ascii_lowercase())
            && !gate::portable_path(&path)
            && !calls.host_calls.contains(&path)
        {
            calls.host_calls.push(path);
        }
    }
    Ok(calls)
}

/// Derive the on-chain part of `source`. See the module text for the rule.
///
/// `source` is server policy source that passed
/// [`source_shape_with_host_calls`](super::source_shape_with_host_calls). The derived policy
/// passes [`source_shape`], the portable profile, and names no server function. The contract
/// build is the last check: a derived policy that does not compile is refused there.
pub fn derive(source: &str) -> Result<Derived> {
    let mut parsed = Parsed {
        source,
        tokens: lex(source)?,
        nodes: vec![],
        server_keys: RefCell::default(),
        item_scopes: RefCell::default(),
    };
    let count = parsed.tokens.len();
    // The file is a list of items. `execute` is the one function that is split.
    let items = parsed.statements(0, count, false, None, 0);
    let execute: Vec<usize> = items
        .iter()
        .copied()
        .filter(|id| parsed.nodes[*id].name.as_deref() == Some("execute"))
        .collect();
    let missing =
        "Policy source must define pub fn execute(ctx: &AllowitContext) -> allowit::Result once";
    let [execute] = execute[..] else {
        return Err(missing.into());
    };
    let (first, last) = (parsed.nodes[execute].first, parsed.nodes[execute].last);
    let open = parsed.find(first, last, &["{"]).ok_or(missing)?;
    if parsed.close(open) != Some(last) {
        return Err(missing.into());
    }
    let body = parsed.statements(open + 1, last, false, None, 0);
    let p = parsed;
    let range = |id: usize| (p.nodes[id].first, p.nodes[id].last + 1);
    let all: Vec<usize> = (0..p.nodes.len()).collect();
    let in_execute = |id: usize| p.nodes[id].first > open && p.nodes[id].last < last;

    // Local functions and constants. A function is one statement here, whether it stands in the
    // file or in a block.
    let named: Vec<usize> = all
        .iter()
        .copied()
        .filter(|id| *id != execute && p.nodes[*id].name.is_some())
        .collect();
    // Where each of them is seen: the block that declares it, or the whole file.
    {
        let mut inside: Vec<Option<usize>> = vec![None; count];
        let mut blocks: Vec<usize> = vec![];
        for (i, block) in inside.iter_mut().enumerate() {
            *block = blocks.last().copied();
            match p.text(i) {
                "{" => blocks.push(i),
                "}" => {
                    blocks.pop();
                }
                _ => {}
            }
        }
        let mut scopes = p.item_scopes.borrow_mut();
        for id in &named {
            let block = inside[p.nodes[*id].first];
            let scope = block.map_or((0, count), |open| (open, p.close(open).unwrap_or(count)));
            if let Some(name) = &p.nodes[*id].name {
                scopes.entry(name.clone()).or_insert(scope);
            }
        }
    }
    let closure = |start: &dyn Fn(usize, usize, &HashSet<String>) -> bool| {
        let mut found: HashSet<String> = HashSet::new();
        loop {
            let before = found.len();
            for id in &named {
                let (from, to) = range(*id);
                let name = p.nodes[*id].name.clone().unwrap_or_default();
                let mut others = found.clone();
                others.remove(&name);
                if start(from, to, &others) {
                    found.insert(name);
                }
            }
            if found.len() == before {
                return found;
            }
        }
    };

    // The state calls of the source, and the keys that each local function can write, itself
    // or through a local function that it calls.
    let uses = p.state_uses()?;
    // A call of a local function: its name, before `(`, that is no field, method, path segment
    // or declaration. The gate admits no function value, so a function is only ever called.
    let call_at = |i: usize| {
        p.word(i).filter(|_| {
            p.text(i + 1) == "(" && !matches!(p.text(i.wrapping_sub(1)), "." | "::" | "fn")
        })
    };
    // Every local function with its tokens, read from each `fn` of the file: a function can
    // stand inside a statement that is not split, where it is no statement of its own.
    // `execute` itself is no local function. A function is seen in the block that holds its
    // declaration, or in the whole file: `scope` is that range of tokens.
    struct LocalFn<'a> {
        name: &'a str,
        tokens: (usize, usize),
        scope: (usize, usize),
    }
    let mut local_fns: Vec<LocalFn> = vec![];
    let mut blocks: Vec<usize> = vec![];
    for i in 0..count {
        match p.text(i) {
            "{" => blocks.push(i),
            "}" => {
                blocks.pop();
            }
            "fn" if !(first..open).contains(&i) => {
                let body = p
                    .find(i + 1, count, &["{", ";"])
                    .filter(|at| p.text(*at) == "{")
                    .and_then(|at| p.close(at));
                let scope = match blocks.last() {
                    Some(block) => (*block, p.close(*block).unwrap_or(count)),
                    None => (0, count),
                };
                if let (Some(end), Some(name)) = (body, p.word(i + 1)) {
                    local_fns.push(LocalFn {
                        name,
                        tokens: (i, end + 1),
                        scope,
                    });
                }
            }
            _ => {}
        }
    }
    // The local function that the call at token `i` names: one that is seen there. Another
    // item can carry the same name outside that block, such as `Some`.
    let called_at = |i: usize| {
        call_at(i).filter(|name| {
            local_fns
                .iter()
                .any(|f| f.name == *name && (f.scope.0..f.scope.1).contains(&i))
        })
    };
    let in_local_fn = |i: usize| {
        local_fns
            .iter()
            .any(|f| (f.tokens.0..f.tokens.1).contains(&i))
    };
    let mut writers: HashMap<String, BTreeSet<String>> = HashMap::new();
    // The local functions that each function calls, read once.
    let mut callers_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for function in &local_fns {
        let (from, to) = function.tokens;
        // A function that stands inside this one runs only when it is called: its tokens
        // are its own.
        let inner: Vec<(usize, usize)> = local_fns
            .iter()
            .map(|other| other.tokens)
            .filter(|(start, end)| *start > from && *end <= to)
            .collect();
        let own = |i: usize| {
            (from..to).contains(&i) && !inner.iter().any(|(start, end)| (*start..*end).contains(&i))
        };
        let direct = uses
            .iter()
            .filter(|state| state.writes && own(state.at))
            .map(|state| state.key.clone());
        writers
            .entry(function.name.to_owned())
            .or_default()
            .extend(direct);
        for callee in (from..to).filter(|i| own(*i)).filter_map(called_at) {
            callers_of.entry(callee).or_default().push(function.name);
        }
    }
    // Each written key goes from a function to its callers until no set grows.
    let mut pending: Vec<&str> = local_fns.iter().map(|f| f.name).collect();
    while let Some(callee) = pending.pop() {
        let written = writers.get(callee).cloned().unwrap_or_default();
        for caller in callers_of.get(callee).into_iter().flatten() {
            let keys = writers.entry((*caller).to_owned()).or_default();
            let before = keys.len();
            keys.extend(written.iter().cloned());
            if keys.len() > before {
                pending.push(caller);
            }
        }
    }

    let mut mutable: HashSet<String> = (open..last)
        .filter(|i| p.word(*i) == Some("mut"))
        .filter_map(|i| p.word(i + 1).map(str::to_owned))
        .collect();
    // A name that a `let` declares without a value gets its value from a later statement,
    // which can leave: `let accepted: bool;`.
    for i in (open..last).filter(|i| p.word(*i) == Some("let")) {
        if let Some(end) = p
            .find(i + 1, last, &["=", ";"])
            .filter(|end| p.text(*end) == ";")
        {
            let pattern = p.find(i + 1, end, &[":"]).unwrap_or(end);
            mutable.extend(p.bound(i + 1, pattern));
        }
    }
    // A value without a stated type can take its type from the code that uses it, and a
    // statement that leaves can be that code. The names that can hold such a value are found
    // here: those bound with no stated type from a value of a form that `fixed` does not
    // know. Names that one statement uses together can share a type, so they form one group,
    // and a removed statement that mentions one marks the group.
    let inferred = {
        const SUFFIXES: &[&str] = &[
            "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
        ];
        // Whether the tokens `from..to` are one value of a form whose type is fixed, whatever
        // code uses it. Every other form counts as open: the rule names the fixed forms, so a
        // form that it does not know moves more to the server, never less.
        let fixed = |from: usize, to: usize, known: &HashSet<String>| -> bool {
            let mut i = from;
            // `&value` and `&mut value` have the type of the value.
            while i < to && p.text(i) == "&" {
                i += 1;
            }
            if i < to && p.text(i) == "mut" {
                i += 1;
            }
            if i >= to {
                return false;
            }
            let one = |at: usize| {
                let text = p.text(at);
                match p.tokens[at].kind {
                    Kind::Literal => {
                        !text.starts_with(|c: char| c.is_ascii_digit())
                            || SUFFIXES.iter().any(|suffix| text.ends_with(suffix))
                    }
                    // The constructors of the language are no bindings, also where a pattern
                    // names one: `None` has an open type.
                    Kind::Word => {
                        matches!(text, "true" | "false")
                            || p.word(at).is_some_and(|name| {
                                known.contains(name)
                                    && !matches!(name, "None" | "Some" | "Ok" | "Err")
                            })
                    }
                    _ => false,
                }
            };
            // A value read whole, with at most `.clone()` or `.as_str()` after it.
            let plain_end = |mut at: usize| {
                while at + 3 < to + 1
                    && p.text(at) == "."
                    && matches!(p.text(at + 1), "clone" | "as_str")
                    && p.text(at + 2) == "("
                    && p.text(at + 3) == ")"
                {
                    at += 4;
                }
                at == to
            };
            // A range takes the type of a side whose type is fixed. A side is one token: a
            // number, or a value that `one` takes.
            if let Some(dots) = p.find(i, to, &["..", "..="]) {
                let number = |at: usize| {
                    p.tokens[at].kind == Kind::Literal
                        && p.text(at).starts_with(|c: char| c.is_ascii_digit())
                };
                let side = |(a, b): (usize, usize)| b == a + 1 && (number(a) || one(a));
                let typed = |(a, b): (usize, usize)| b == a + 1 && one(a);
                let (left, right) = ((i, dots), (dots + 1, to));
                return side(left) && side(right) && (typed(left) || typed(right));
            }
            if one(i) && plain_end(i + 1) {
                return true;
            }
            // A field of the request.
            if p.text(i) == "ctx" && p.text(i + 1) == "." && p.text(i + 2) == "request" {
                let mut at = i + 3;
                while p.text(at) == "." && p.word(at + 1).is_some() && p.text(at + 2) != "(" {
                    at += 2;
                }
                return plain_end(at);
            }
            // An owner param read whole: an `AllowitValue`.
            if p.text(i) == "ctx" && p.text(i + 1) == "." && p.text(i + 2) == "params" {
                return p.text(i + 3) == "[" && p.close(i + 3).is_some_and(|close| close + 1 == to);
            }
            // A call of a library function with a fixed return type, or of `from` or
            // `try_from` of an integer type, with at most `?` after it.
            let call = if let Some(function) = p.allowit_path(i) {
                let generic = matches!(function, "get_state" | "param" | "arg");
                let open = if p.text(i + 3) == "::" && p.text(i + 4) == "<" {
                    p.after_generics(i + 4)
                        .filter(|end| !(i + 4..*end).any(|at| p.text(at) == "_"))
                } else {
                    (!generic).then_some(i + 3)
                };
                open.filter(|at| p.text(*at) == "(")
            } else {
                (SUFFIXES.contains(&p.text(i))
                    && p.text(i + 1) == "::"
                    && matches!(p.text(i + 2), "from" | "try_from"))
                .then_some(i + 3)
                .filter(|at| p.text(*at) == "(")
            };
            call.and_then(|open| p.close(open)).is_some_and(|close| {
                close + 1 == to || (close + 2 == to && p.text(close + 1) == "?")
            })
        };
        // The bindings of `execute`: each pattern with the value that it takes.
        type Tokens = (usize, usize);
        let mut bindings: Vec<(Tokens, Option<Tokens>)> = vec![];
        for id in all.iter().copied().filter(|id| in_execute(*id)) {
            let node = &p.nodes[id];
            let key = node.key;
            match node.shape {
                Shape::Let => {
                    let end = p.find(key + 1, node.last, &["=", ":"]).unwrap_or(node.last);
                    // A stated type with no `_` in it fixes the type.
                    let typed = p.text(end) == ":"
                        && !(end..node.init.map_or(node.last, |(from, _)| from))
                            .any(|i| p.text(i) == "_");
                    if !typed {
                        bindings.push(((key + 1, end), node.init));
                    }
                }
                Shape::Split => {
                    let headers = &node.headers;
                    if p.text(key) == "match" {
                        for pattern in headers.iter().skip(1) {
                            bindings.push((*pattern, headers.first().copied()));
                        }
                    } else if p.text(key) == "for"
                        && let Some((from, to)) = headers.first().copied()
                    {
                        let at = p.find(from, to, &["in"]).unwrap_or(to);
                        bindings.push(((from, at), Some((at + 1, to))));
                    }
                    for (from, to) in headers {
                        for at in (*from..*to).filter(|i| p.text(*i) == "let") {
                            let equals = p.find(at + 1, *to, &["="]).unwrap_or(*to);
                            let end = p.find(equals + 1, *to, &["&&", "||"]).unwrap_or(*to);
                            bindings.push(((at + 1, equals), Some((equals + 1, end))));
                        }
                    }
                }
                _ => {}
            }
        }
        // Start from every name as fixed, and take out each name that a binding gives an open
        // value, until no name moves: a name bound twice is open when one binding is.
        let mut known: HashSet<String> = named
            .iter()
            .filter_map(|id| p.nodes[*id].name.clone())
            .chain(["ctx".to_owned()])
            .chain(
                bindings
                    .iter()
                    .flat_map(|((from, to), _)| p.bound(*from, *to)),
            )
            .chain(
                all.iter()
                    .filter(|id| in_execute(**id) && p.nodes[**id].shape == Shape::Let)
                    .flat_map(|id| p.nodes[*id].names.iter().cloned()),
            )
            .collect();
        let mut names: HashSet<String> = HashSet::new();
        loop {
            let before = names.len();
            for ((from, to), value) in &bindings {
                if value.is_none_or(|(start, end)| !fixed(start, end, &known)) {
                    let bound = p.bound(*from, *to);
                    for name in &bound {
                        known.remove(name);
                    }
                    names.extend(bound);
                }
            }
            if names.len() == before {
                break;
            }
        }
        // The groups: the names that one statement or one binding uses together.
        let mut groups: HashMap<String, BTreeSet<String>> = names
            .iter()
            .map(|name| (name.clone(), BTreeSet::from([name.clone()])))
            .collect();
        let ranges: Vec<(usize, usize)> = all
            .iter()
            .copied()
            .filter(|id| in_execute(*id) || *id == execute)
            .flat_map(|id| match p.nodes[id].shape {
                Shape::Split => p.nodes[id].headers.clone(),
                _ if id == execute => vec![],
                _ => vec![range(id)],
            })
            .collect();
        loop {
            let mut grew = false;
            for (from, to) in &ranges {
                let together: BTreeSet<String> = (*from..*to)
                    .filter(|i| p.names(*i, &names))
                    .filter_map(|i| groups.get(p.word(i)?).cloned())
                    .flatten()
                    .collect();
                for name in &together {
                    let group = groups.get_mut(name).expect("a name of a group");
                    if group.len() < together.len() {
                        *group = together.clone();
                        grew = true;
                    }
                }
            }
            if !grew {
                break groups;
            }
        }
    };
    let scope = Scope {
        body: &body,
        mutable: &mutable,
        inferred: &inferred,
    };
    let inferred_names: HashSet<String> = inferred.keys().cloned().collect();

    // The statements of `execute` in source order, with the statements of their blocks.
    fn walk(nodes: &[Node], ids: &[usize], out: &mut Vec<usize>) {
        for id in ids {
            out.push(*id);
            for block in &nodes[*id].children {
                walk(nodes, block, out);
            }
            out.extend(&nodes[*id].arms);
        }
    }
    let mut order = vec![];
    walk(&p.nodes, &body, &mut order);

    // The tokens that decide whether a statement leaves: all of a statement that is not split,
    // and the conditions of a split one.
    let own = |id: usize| match p.nodes[id].shape {
        Shape::Split => p.nodes[id].headers.clone(),
        _ => vec![range(id)],
    };

    fn leave(
        p: &Parsed,
        scope: &Scope,
        cause: &mut Vec<Option<Cause>>,
        tainted: &mut HashSet<String>,
        id: usize,
        why: Cause,
    ) {
        if cause[id].is_some() {
            return;
        }
        // The statement leaves with everything inside it.
        fn mark(
            p: &Parsed,
            cause: &mut Vec<Option<Cause>>,
            tainted: &mut HashSet<String>,
            id: usize,
            why: Cause,
        ) {
            if cause[id].is_some() {
                return;
            }
            cause[id] = Some(why);
            tainted.extend(p.nodes[id].names.iter().cloned());
            let inside = match why {
                Cause::ServerCall | Cause::ServerValue => Cause::ServerBranch,
                other => other,
            };
            let children: Vec<usize> = p.nodes[id]
                .children
                .iter()
                .flatten()
                .chain(&p.nodes[id].arms)
                .copied()
                .collect();
            for child in children {
                mark(p, cause, tainted, child, inside);
            }
        }
        mark(p, cause, tainted, id, why);
        let (from, to) = (p.nodes[id].first, p.nodes[id].last + 1);
        for i in from..to {
            if p.names(i, scope.mutable) {
                tainted.extend(p.word(i).map(str::to_owned));
            }
            // A leaving statement can give the type of a number without one: then every name
            // that shares that type can hold another value on chain.
            if let Some(group) = p.word(i).and_then(|name| scope.inferred.get(name))
                && p.names(i, &group.iter().cloned().collect())
            {
                tainted.extend(group.iter().cloned());
            }
        }
        let returns = p.returns(from, to);
        if !returns && !p.breaks(from, to) {
            return;
        }
        // An exit that the chain cannot follow: the enclosing loops leave, and after a
        // `return` every later statement leaves too.
        let mut current = id;
        loop {
            let parent = p.nodes[current].parent;
            if returns {
                let later: Vec<usize> = match (parent, p.nodes[current].place) {
                    (Some(parent), Some((slot, at))) => {
                        p.nodes[parent].children[slot][at + 1..].to_vec()
                    }
                    (None, Some((_, at))) => scope.body[at + 1..].to_vec(),
                    _ => vec![],
                };
                // An item is no step of the evaluation: it leaves only when nothing names it.
                for next in later {
                    if p.nodes[next].shape == Shape::Item {
                        continue;
                    }
                    leave(p, scope, cause, tainted, next, Cause::AfterServerExit);
                }
            }
            let Some(parent) = parent else {
                return;
            };
            if p.nodes[parent].is_loop {
                leave(p, scope, cause, tainted, parent, Cause::AfterServerExit);
            }
            current = parent;
        }
    }

    let functions: HashSet<String> = named
        .iter()
        .filter(|id| p.nodes[**id].is_fn)
        .filter_map(|id| p.nodes[*id].name.clone())
        .chain(local_fns.iter().map(|f| f.name.to_owned()))
        .collect();
    let covered = |gone: &[bool]| {
        let mut left = vec![false; count];
        for id in &all {
            if gone[*id] {
                let (from, to) = range(*id);
                left[from..to].fill(true);
            }
        }
        left
    };

    // A key whose write leaves is the server's, and then every statement that uses the key
    // leaves. That can make another write leave, so the steps repeat until no key moves.
    // The names whose type a removed statement could have given, beyond the leaving steps:
    // a `let` that only leaving code read, for one. The steps run again with them.
    let mut pinned: HashSet<String> = HashSet::new();
    let (callers, readers, cause, gone, changed) = loop {
        // Local functions and constants that make a server call or use state that the server
        // keeps.
        let helpers = closure(&|from, to, found| p.server(from, to, found));
        // The helpers that make a server call. The others only use state that the server keeps.
        let callers = closure(&|from, to, found| p.calls(from, to, found));
        let readers: HashSet<String> = helpers.difference(&callers).cloned().collect();
        let mut cause: Vec<Option<Cause>> = vec![None; p.nodes.len()];
        let mut tainted: HashSet<String> = pinned.clone();
        loop {
            let before = cause.iter().flatten().count() + tainted.len();
            for id in &order {
                if cause[*id].is_some() {
                    continue;
                }
                // A `use` item is no step: it keeps the names that the kept code reads.
                if p.nodes[*id].shape == Shape::Item && p.text(p.nodes[*id].key) == "use" {
                    continue;
                }
                let ranges = own(*id);
                let calls = ranges
                    .iter()
                    .any(|(from, to)| p.server(*from, *to, &helpers));
                // An item cannot read a local name: a function or constant that has the name of a
                // server value is not that value.
                let reads = p.nodes[*id].shape != Shape::Item
                    && ranges
                        .iter()
                        .any(|(from, to)| p.mentions(*from, *to, &tainted));
                let why = match (calls, reads) {
                    (false, false) => continue,
                    _ if p.nodes[*id].braced => Cause::NotSplit,
                    (true, _) => Cause::ServerCall,
                    (false, true) => Cause::ServerValue,
                };
                leave(&p, &scope, &mut cause, &mut tainted, *id, why);
            }
            if cause.iter().flatten().count() + tainted.len() == before {
                break;
            }
        }

        // Items of the file that make a server call leave. A `const _` has no name that
        // another statement could read, so it leaves by its own tokens.
        let mut gone: Vec<bool> = cause.iter().map(Option::is_some).collect();
        for id in &named {
            if helpers.contains(p.nodes[*id].name.as_deref().unwrap_or_default()) {
                gone[*id] = true;
            }
        }
        for id in &all {
            let node = &p.nodes[*id];
            let anonymous = matches!(node.shape, Shape::Item | Shape::Whole)
                && node.name.is_none()
                && p.text(node.key) == "const";
            if anonymous && !gone[*id] {
                let (from, to) = range(*id);
                gone[*id] = p.server(from, to, &helpers);
            }
        }
        let changed = gone.iter().any(|left| *left);

        // What only leaving code read leaves too: a `let` that only reads, a local function,
        // a constant.
        loop {
            let left = covered(&gone);
            let mut more = vec![];
            for id in &all {
                let node = &p.nodes[*id];
                if gone[*id] || *id == execute {
                    continue;
                }
                let wanted: HashSet<String> = match node.shape {
                    Shape::Let if in_execute(*id) => match node.init {
                        Some((from, to)) if p.reads_only(from, to, &functions) => {
                            node.names.iter().cloned().collect()
                        }
                        _ => continue,
                    },
                    Shape::Item => node.name.iter().cloned().collect(),
                    _ => continue,
                };
                if wanted.is_empty() {
                    continue;
                }
                let (from, to) = range(*id);
                let outside = |i: &usize| *i < from || *i >= to;
                let read = |i: &usize| match node.shape {
                    Shape::Item => p.names_item(*i, &wanted),
                    _ => p.names(*i, &wanted),
                };
                let kept = (0..count).filter(outside).any(|i| !left[i] && read(&i));
                let dropped = (0..count).filter(outside).any(|i| left[i] && read(&i));
                if !kept && dropped {
                    more.push(*id);
                }
            }
            if more.is_empty() {
                break;
            }
            for id in more {
                gone[id] = true;
            }
        }

        // The keys that a leaving step of `execute` writes, itself or through a local
        // function that it calls. A function that leaves writes nothing by that: its calls
        // write, and they are steps of `execute` or of a function that such a step calls.
        let left = covered(&gone);
        let step = |i: usize| left[i] && (open..last).contains(&i) && !in_local_fn(i);
        let mut moved: BTreeSet<String> = uses
            .iter()
            .filter(|state| state.writes && step(state.at))
            .map(|state| state.key.clone())
            .collect();
        for written in (0..count)
            .filter(|i| step(*i))
            .filter_map(called_at)
            .filter_map(|name| writers.get(name))
        {
            moved.extend(written.iter().cloned());
        }
        moved.retain(|key| !p.server_keys.borrow().contains(key));
        // Every statement that leaves can give the type of a name that it mentions, also one
        // that left only because leaving code read it.
        let typed_by_removed: BTreeSet<String> = (0..count)
            .filter(|i| left[*i] && (open..last).contains(i))
            .filter(|i| p.names(*i, &inferred_names))
            .filter_map(|i| inferred.get(p.word(i)?))
            .flatten()
            .filter(|name| !tainted.contains(*name))
            .cloned()
            .collect();
        if moved.is_empty() && typed_by_removed.is_empty() {
            break (callers, readers, cause, gone, changed);
        }
        pinned.extend(typed_by_removed);
        p.server_keys.borrow_mut().extend(moved);
    };

    // The derivation reads `return Err(...)` as a denial and ends a derived `execute` with
    // `Ok(())`. A local item with one of these names would change both.
    let shadowed = (0..count).any(|i| {
        matches!(p.word(i), Some("fn" | "const")) && matches!(p.word(i + 1), Some("Ok" | "Err"))
    });
    if changed && shadowed {
        return Err("A policy that depends on a server function cannot define a local `Ok` or `Err`. Rename that function or constant.".into());
    }
    // The parser ends each statement where Rust ends it. A statement that it still read with
    // the next one, as after a missing `;`, shows a `let` or an item at its own level, where
    // no expression holds one: only a condition holds a `let`, after `if`, `while`, `&&` or
    // `||`. The name that such a declaration binds would stay seen by later statements while
    // it left with the group, so the derivation refuses the source.
    let grouped = all
        .iter()
        .filter(|id| matches!(p.nodes[**id].shape, Shape::Whole | Shape::Let | Shape::Item))
        .find_map(|id| {
            // From the statement's own first token, past its attributes.
            let (from, to) = (p.nodes[*id].key, p.nodes[*id].last + 1);
            let mut i = from;
            while i < to {
                if matches!(p.text(i), "(" | "[" | "{") {
                    i = p.close(i)?;
                } else if i > from {
                    // Attributes can stand before the declaration.
                    let mut at = i;
                    while p.text(at) == "#" && p.text(at + 1) == "[" {
                        at = p.close(at + 1)? + 1;
                    }
                    let declaration = match p.text(at) {
                        "let" => !matches!(p.text(at - 1), "if" | "while" | "&&" | "||"),
                        // `const fn` is one declaration.
                        "fn" => p.text(at - 1) != "const",
                        "use" => true,
                        // A `const` parameter of a generic function is no item.
                        "const" => {
                            p.text(at + 1) != "{" && !matches!(p.before_attributes(at), "<" | ",")
                        }
                        _ => false,
                    };
                    if declaration && p.tokens[at].kind == Kind::Word {
                        return Some(p.line(p.tokens[at].start));
                    }
                    if at > i {
                        i = at;
                        continue;
                    }
                }
                i += 1;
            }
            None
        });
    if let Some(line) = grouped.filter(|_| changed) {
        return Err(format!(
            "A policy for a contract wallet that depends on a server function must end the statement before line {line} with `;`: the derivation cannot tell where it ends."
        ));
    }
    // A mutable reference lets a statement assign a name that it does not mention. The rule
    // follows names, not references, so it derives no part of such a policy.
    let aliasing = (0..count).any(|i| {
        p.word(i + 1) == Some("mut") && (matches!(p.text(i), "&" | "&&") || p.text(i) == "ref")
    });
    if changed && aliasing {
        return Err("A policy for a contract wallet that depends on a server function cannot take a mutable reference (`&mut` or `ref mut`). Assign the name itself.".into());
    }
    // Two local functions or constants with one name: the rule cannot tell which one a
    // statement calls. A function or a constant can also stand inside another function or a
    // statement that is not split, where it is no statement of its own, so every `fn` and
    // every `const` of the file counts.
    let mut seen: HashSet<&str> = HashSet::new();
    let mut declared: HashSet<&str> = HashSet::new();
    if let Some(twice) = named
        .iter()
        .filter_map(|id| p.nodes[*id].name.as_deref())
        .find(|name| !seen.insert(name))
        .or_else(|| {
            (0..count)
                .filter(|i| matches!(p.text(*i), "fn" | "const") && p.text(i + 1) != "fn")
                // A `const` parameter of a generic function is no item.
                .filter(|i| !matches!(p.before_attributes(*i), "<" | ","))
                .filter_map(|i| p.word(i + 1))
                .filter(|name| *name != "_")
                .find(|name| !declared.insert(name))
        })
        .filter(|_| changed)
    {
        return Err(format!(
            "A policy for a contract wallet that depends on a server function cannot declare `{twice}` twice. Rename one of the two."
        ));
    }

    // The text: each topmost leaving statement leaves once.
    let bytes = |id: usize| {
        (
            p.tokens[p.nodes[id].first].start,
            p.tokens[p.nodes[id].last].end,
        )
    };
    let last_step = body
        .iter()
        .rev()
        .copied()
        .find(|id| p.nodes[*id].shape != Shape::Item);
    let mut edits: Vec<Edit> = vec![];
    for id in &all {
        let node = &p.nodes[*id];
        let top = gone[*id] && !node.parent.is_some_and(|parent| gone[parent]);
        let (start, end) = bytes(*id);
        if top {
            if last_step == Some(*id) {
                // The last step of `execute` leaves: the derived `execute` approves there.
                // Items can follow the last step, so it is then a `return`.
                let approval = if body.last() == Some(id) {
                    "Ok(())"
                } else {
                    "return Ok(());"
                };
                if &source[start..end] != approval {
                    edits.push(Edit {
                        start,
                        end,
                        text: approval.into(),
                    });
                }
            } else if node.shape == Shape::Arm {
                edits.push(Edit {
                    start,
                    end,
                    text: "{}".into(),
                });
            } else {
                let (start, end) = own_lines(source, start, end);
                edits.push(Edit {
                    start,
                    end,
                    text: String::new(),
                });
            }
            continue;
        }
        // A block whose statements all leave is an empty block.
        if node.shape == Shape::Split && !gone[*id] {
            for (block, (open, close)) in node.children.iter().zip(&node.blocks) {
                if !block.is_empty() && block.iter().all(|child| gone[*child]) {
                    edits.push(Edit {
                        start: p.tokens[*open].end,
                        end: p.tokens[*close].start,
                        text: String::new(),
                    });
                }
            }
        }
        // A `use` item keeps its other names.
        let item = node.key;
        if node.shape != Shape::Item || p.text(item) != "use" || gone[*id] {
            continue;
        }
        let names: Vec<&str> = (item + 3..node.last).filter_map(|i| p.word(i)).collect();
        if p.word(item + 1) != Some("allowit") || p.word(item + 3) == Some("prelude") {
            continue;
        }
        // A server request type leaves. So does a type that only leaving code named.
        let left = covered(&gone);
        let (from, to) = range(*id);
        let named_by = |name: &str, leaving: bool| {
            (0..count).any(|i| {
                (i < from || i >= to)
                    && left[i] == leaving
                    && p.word(i) == Some(name)
                    && p.text(i.wrapping_sub(1)) != "::"
            })
        };
        let stay: Vec<&str> = names
            .iter()
            .copied()
            .filter(|name| {
                !gate::SERVER_TYPES.contains(name)
                    && (named_by(name, false) || !named_by(name, true))
            })
            .collect();
        if stay.len() == names.len() {
            continue;
        }
        if stay.is_empty() {
            let (start, end) = own_lines(source, start, end);
            edits.push(Edit {
                start,
                end,
                text: String::new(),
            });
        } else {
            let head = &source[start..p.tokens[item + 3].start];
            let text = match stay[..] {
                [one] => format!("{head}{one};"),
                _ => format!("{head}{{{}}};", stay.join(", ")),
            };
            edits.push(Edit { start, end, text });
        }
    }
    if edits.is_empty() && !changed {
        return finish(
            source,
            source.to_owned(),
            vec![],
            vec![],
            &uses,
            &HashSet::new(),
        );
    }

    edits.sort_by_key(|edit| edit.start);
    // The line ranges that left, with adjacent ranges as one.
    let mut left: Vec<(u32, u32)> = vec![];
    for edit in &edits {
        let (from, to) = (p.line(edit.start), p.line(edit.end.max(edit.start + 1) - 1));
        match left.last_mut() {
            Some(last) if from <= last.1 + 1 => last.1 = last.1.max(to),
            _ => left.push((from, to)),
        }
    }
    // The on-chain part leaves code out, so a lint that turns unused code into an error could
    // stop it from compiling. It states `warn` where the source states `deny` or `forbid`: a
    // lint level changes nothing that the policy does.
    for i in 0..count {
        let attribute = p.text(i.wrapping_sub(1)) == "["
            && (p.text(i.wrapping_sub(2)) == "#"
                || (p.text(i.wrapping_sub(2)) == "!" && p.text(i.wrapping_sub(3)) == "#"));
        if attribute && matches!(p.word(i), Some("deny" | "forbid")) {
            edits.push(Edit {
                start: p.tokens[i].start,
                end: p.tokens[i].end,
                text: "warn".into(),
            });
        }
    }
    edits.sort_by_key(|edit| edit.start);
    let mut derived = String::with_capacity(source.len());
    let mut at = 0;
    for edit in &edits {
        if edit.start < at {
            continue;
        }
        derived.push_str(&source[at..edit.start]);
        at = edit.end;
        // Removed lines between two blank lines, or between a blank line and a closing brace,
        // leave one blank line less. Removed lines after an opening brace take the blank line
        // after them.
        let line = source[at..]
            .split_inclusive('\n')
            .next()
            .unwrap_or_default();
        let after = line.trim();
        if edit.text.is_empty() && edit.end > edit.start {
            if derived.ends_with("\n\n") && (after.is_empty() || after.starts_with('}')) {
                derived.pop();
            } else if after.is_empty()
                && line.ends_with('\n')
                && derived.trim_end_matches([' ', '\t']).ends_with("{\n")
            {
                at += line.len();
            }
        }
        derived.push_str(&edit.text);
    }
    derived.push_str(&source[at..]);

    // Each leaving statement reports its own checks: a split statement those of its
    // conditions, and its statements theirs. A leaving local function is not split.
    //
    // A check that itself calls a server function, reads a server value or reads stored state
    // cannot run on chain wherever it stands. So its cause is its own, also inside a branch
    // that left. A server call or value comes before stored state.
    let mut served: HashSet<String> = HashSet::new();
    let mut stored: HashSet<String> = HashSet::new();
    let mut own_cause: Vec<Option<Cause>> = vec![None; p.nodes.len()];
    loop {
        let before = served.len() + stored.len() + own_cause.iter().flatten().count();
        for id in order.iter().copied().filter(|id| cause[*id].is_some()) {
            let ranges = own(id);
            let item = p.nodes[id].shape == Shape::Item;
            let any = |test: &dyn Fn(usize, usize) -> bool| {
                ranges.iter().any(|(from, to)| test(*from, *to))
            };
            let why = if any(&|from, to| p.calls(from, to, &callers)) {
                Cause::ServerCall
            } else if !item && any(&|from, to| p.mentions(from, to, &served)) {
                Cause::ServerValue
            } else if any(&|from, to| {
                (from..to).any(|i| p.state_at(i) || p.names_item(i, &readers))
            }) || (!item && any(&|from, to| p.mentions(from, to, &stored)))
            {
                Cause::StoredState
            } else {
                continue;
            };
            if own_cause[id].is_some_and(|known| known != Cause::StoredState) {
                continue;
            }
            own_cause[id] = Some(why);
            let names = if why == Cause::StoredState {
                &mut stored
            } else {
                &mut served
            };
            names.extend(p.nodes[id].names.iter().cloned());
            let (from, to) = range(id);
            for i in from..to {
                if p.names(i, &mutable) {
                    names.extend(p.word(i).map(str::to_owned));
                }
            }
        }
        if served.len() + stored.len() + own_cause.iter().flatten().count() == before {
            break;
        }
    }
    let mut server_only = vec![];
    // A statement that is not split reports each check with the cause of what the check
    // itself reads. Its own `let` names count as server values when the statement depends on a
    // server function, and as stored state when it reads stored state.
    let unsplit = |id: usize| -> Option<(HashSet<String>, Cause)> {
        let own = own_cause[id]?;
        let (from, to) = range(id);
        let mut local = HashSet::new();
        for k in (from..to).filter(|k| p.word(*k) == Some("let")) {
            let end = p.find(k + 1, to, &["=", ":", ";"]).unwrap_or(to);
            local.extend(p.bound(k + 1, end));
        }
        let local_cause = match own {
            Cause::StoredState => Cause::StoredState,
            _ => Cause::ServerValue,
        };
        Some((local, local_cause))
    };
    let reported = order
        .iter()
        .copied()
        .filter(|id| gone[*id])
        .map(|id| {
            let structural = cause[id].unwrap_or(Cause::ServerValue);
            let braced = p.nodes[id].braced;
            let why = match own_cause[id] {
                Some(own) if !braced => own,
                _ => structural,
            };
            (own(id), why, braced.then(|| unsplit(id)).flatten())
        })
        .chain(
            named
                .iter()
                .copied()
                .filter(|id| gone[*id] && !in_execute(*id))
                .map(|id| (vec![range(id)], Cause::NotSplit, None)),
        );
    for (ranges, why, inside) in reported {
        for i in ranges.into_iter().flat_map(|(from, to)| from..to) {
            let Some(check) = p.check_at(i) else {
                continue;
            };
            let cause = match &inside {
                // The check's own tokens: the keyword or path, then its bracketed arguments.
                Some((local, local_cause)) => {
                    let end = (i..(i + 4).min(count))
                        .find(|k| matches!(p.text(*k), "(" | "[" | "{"))
                        .and_then(|open| p.close(open))
                        .map_or(i + 1, |close| close + 1);
                    if p.calls(i, end, &callers) {
                        Cause::ServerCall
                    } else if p.mentions(i, end, &served) {
                        Cause::ServerValue
                    } else if p.mentions(i, end, local) {
                        *local_cause
                    } else if (i..end).any(|k| p.state_at(k) || p.names_item(k, &readers))
                        || p.mentions(i, end, &stored)
                    {
                        Cause::StoredState
                    } else {
                        why
                    }
                }
                None => why,
            };
            server_only.push(ServerOnly {
                line: p.line(p.tokens[i].start),
                check,
                cause,
            });
        }
    }
    server_only.sort_by_key(|check| check.line);
    let server_keys = p.server_keys.borrow().clone();
    finish(source, derived, left, server_only, &uses, &server_keys)
}

/// Check the derived policy, list the checks that it keeps and name the keeper of each state
/// key. `uses` are the state calls of the owner's source.
fn finish(
    original: &str,
    source: String,
    left: Vec<(u32, u32)>,
    server_only: Vec<ServerOnly>,
    uses: &[StateUse],
    server_keys: &HashSet<String>,
) -> Result<Derived> {
    let defect = |what: &str| {
        format!(
            "The on-chain part of this policy could not be derived: {what}. Report this policy source."
        )
    };
    let parsed = Parsed {
        source: &source,
        tokens: lex(&source)?,
        nodes: vec![],
        server_keys: RefCell::default(),
        item_scopes: RefCell::default(),
    };
    let count = parsed.tokens.len();
    if source != original {
        if (0..count).any(|i| parsed.server_at(i)) {
            return Err(defect("it still names a server function"));
        }
        source_shape(&source).map_err(|error| defect(&error))?;
    }
    let mut kept: Vec<String> = vec![];
    for check in (0..count).filter_map(|i| parsed.check_at(i)) {
        if !kept.contains(&check) {
            kept.push(check);
        }
    }
    // The state calls that stay: the contract keeps their keys, and the figures that they read.
    let stays = parsed.state_uses().map_err(|error| defect(&error))?;
    let read: BTreeSet<&str> = stays
        .iter()
        .map(|state| state.key.as_str())
        .filter(|key| standard(key))
        .collect();
    let chain: BTreeSet<&str> = stays.iter().map(|state| state.key.as_str()).collect();
    // A key that the server keeps is used by no statement that stays.
    if let Some(key) = chain.iter().find(|key| server_keys.contains(**key)) {
        return Err(defect(&format!(
            "the derived part still uses the state key {key:?}, which the server keeps"
        )));
    }
    let state = uses
        .iter()
        .map(|state| state.key.as_str())
        .filter(|key| !standard(key))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|key| StateEntry {
            key: key.to_owned(),
            daily: key.starts_with(DAILY_PREFIX),
            keeper: if chain.contains(key) {
                Keeper::Chain
            } else {
                Keeper::Server
            },
        })
        .collect();
    Ok(Derived {
        source,
        left,
        server_only,
        figures: figures(&kept, &read),
        kept,
        state,
    })
}
