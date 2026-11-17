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
//!    which only the server can read, or reads stored policy state with `allowit::get_state`.
//!    A server call is a call of a server function, of a registered package function, or of a
//!    local function that makes one;
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
//! A contract wallet stores no policy state of its own: it keeps only the cumulated figures
//! that its kept canonical rules read ([`Derived::figures`]). So a statement that reads stored
//! state leaves (rule 1), and the server reads the same figures when it evaluates the whole
//! source. [`derive`] refuses a source that writes policy state (`allowit::set_state`)
//! anywhere: neither the chain nor the server stores such a write.
//!
//! [`derive`] also refuses a source with a server part that the rule cannot follow by its
//! names: one that takes a mutable reference (`&mut`, `ref mut`), which lets a statement
//! assign a name that it does not mention; one that declares two local functions or
//! constants with one name; and one that declares a local `Ok` or `Err`.
use super::{Result, gate, param_keys, source_shape};
use std::collections::HashSet;

/// Why a statement runs only on the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The statement makes a server call.
    ServerCall,
    /// The statement reads a value that a server call gave.
    ServerValue,
    /// The statement reads stored policy state, which only the server reads.
    StoredState,
    /// The statement is in a branch whose condition depends on a server function or on
    /// stored state.
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
    /// The cumulated figures that the kept checks read, in a fixed order and without
    /// repetition. A contract wallet keeps these figures and no other state.
    pub figures: Vec<Figure>,
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

/// The figures that the checks in `kept` read.
fn figures(kept: &[String]) -> Vec<Figure> {
    FIGURES
        .iter()
        .filter(|(_, rules)| rules.iter().any(|rule| kept.iter().any(|k| k == rule)))
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
        } else if ["::", "=>", "->", "==", "!=", "&&", "||"]
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
                            let stop = self.find(body, close, &[","]).unwrap_or(close);
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
                let open = self.block_after(p + 1, end).filter(|open| *open > p + 1)?;
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
                let free = !matches!(name, "mut" | "ref" | "_")
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
            } else if matches!(self.text(item), "use" | "const") {
                let last = plain(self);
                let id = self.node(Shape::Item, first, last, parent);
                if self.text(item) == "const" {
                    self.nodes[id].name = self.word(item + 1).map(str::to_owned);
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
                let branch = self
                    .branch(p, close)
                    .filter(|branch| !self.continues(branch.end + 1));
                match branch {
                    None => {
                        let last = plain(self);
                        self.node(Shape::Whole, first, last, parent)
                    }
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
            return gate::SERVER_FUNCTIONS.contains(&function) || function == "get_state";
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

    /// Whether token `i` reads stored policy state: `allowit::get_state`.
    fn state_at(&self, i: usize) -> bool {
        self.allowit_path(i) == Some("get_state")
    }

    fn server(&self, from: usize, to: usize, helpers: &HashSet<String>) -> bool {
        (from..to).any(|i| self.server_at(i) || self.server_text_at(i) || self.names(i, helpers))
    }

    /// Whether the tokens make a server call other than a read of stored state. `callers` are
    /// the local functions that make one.
    fn calls(&self, from: usize, to: usize, callers: &HashSet<String>) -> bool {
        (from..to).any(|i| {
            (self.server_at(i) && !self.state_at(i))
                || self.server_text_at(i)
                || self.names(i, callers)
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
        let (mut pending, mut depth, mut pending_depth) = (false, 0i64, 0i64);
        for i in self.steps(from, to) {
            match self.text(i) {
                "loop" | "while" | "for" if self.tokens[i].kind == Kind::Word => {
                    pending = true;
                    pending_depth = depth;
                }
                "(" | "[" => depth += 1,
                ")" | "]" => depth -= 1,
                "{" => {
                    loops.push(pending && depth == pending_depth);
                    pending = false;
                }
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
            if matches!(text, "?" | "!" | "=" | "{" | "}") || self.names(i, functions) {
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

    // Local functions and constants that make a server call, and local functions that write
    // state. A function is one statement here, whether it stands in the file or in a block.
    let named: Vec<usize> = all
        .iter()
        .copied()
        .filter(|id| *id != execute && p.nodes[*id].name.is_some())
        .collect();
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
    let helpers = closure(&|from, to, found| p.server(from, to, found));
    // The helpers that make a server call other than a read of stored state. The others only
    // read stored state.
    let callers = closure(&|from, to, found| p.calls(from, to, found));
    let readers: HashSet<String> = helpers.difference(&callers).cloned().collect();

    // Neither a contract wallet nor the server stores a policy's own state: the wallet keeps
    // only the figures of its canonical rules.
    if let Some(write) = (0..count).find(|i| p.allowit_path(*i) == Some("set_state")) {
        return Err(format!(
            "A policy for a contract wallet cannot write policy state (`allowit::set_state`, line {}). The wallet keeps only the cumulated figures that its canonical rules read, and no policy state of its own. State the limit with a canonical rule.",
            p.line(p.tokens[write].start)
        ));
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
    let scope = Scope {
        body: &body,
        mutable: &mutable,
    };

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

    let mut cause: Vec<Option<Cause>> = vec![None; p.nodes.len()];
    let mut tainted: HashSet<String> = HashSet::new();
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

    // Items of the file that make a server call leave.
    let mut gone: Vec<bool> = cause.iter().map(Option::is_some).collect();
    for id in &named {
        if helpers.contains(p.nodes[*id].name.as_deref().unwrap_or_default()) {
            gone[*id] = true;
        }
    }
    let changed = gone.iter().any(|left| *left);

    // What only leaving code read leaves too: a `let` that only reads, a local function, a
    // constant.
    let functions: HashSet<String> = named
        .iter()
        .filter(|id| p.nodes[**id].is_fn)
        .filter_map(|id| p.nodes[*id].name.clone())
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
            let read = |i: &usize| p.names(*i, &wanted);
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

    // The derivation reads `return Err(...)` as a denial and ends a derived `execute` with
    // `Ok(())`. A local item with one of these names would change both.
    let shadowed = (0..count).any(|i| {
        matches!(p.word(i), Some("fn" | "const")) && matches!(p.word(i + 1), Some("Ok" | "Err"))
    });
    if changed && shadowed {
        return Err("A policy that depends on a server function cannot define a local `Ok` or `Err`. Rename that function or constant.".into());
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
    // statement calls.
    let mut seen: HashSet<&str> = HashSet::new();
    if let Some(twice) = named
        .iter()
        .filter_map(|id| p.nodes[*id].name.as_deref())
        .find(|name| !seen.insert(name))
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
        return finish(source, source.to_owned(), vec![], vec![]);
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
            } else if any(&|from, to| (from..to).any(|i| p.state_at(i) || p.names(i, &readers)))
                || (!item && any(&|from, to| p.mentions(from, to, &stored)))
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
                    } else if (i..end).any(|k| p.state_at(k) || p.names(k, &readers))
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
    finish(source, derived, left, server_only)
}

/// Check the derived policy and list the checks that it keeps.
fn finish(
    original: &str,
    source: String,
    left: Vec<(u32, u32)>,
    server_only: Vec<ServerOnly>,
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
    Ok(Derived {
        source,
        left,
        server_only,
        figures: figures(&kept),
        kept,
    })
}
