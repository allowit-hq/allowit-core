//! The error catalogue: what each typed error code means and what its reader does next.
//!
//! A server, a rail and a client send a stable code with a refusal. Every host shows the
//! catalogue's text for that code and links the help page of the code, and keeps the sender's own
//! text for a details view. A host holds no error text of its own. The TypeScript package exports
//! the same bytes as `errorCatalogue`.
//!
//! The generator (`scripts/generate-presentation.mjs`) composes this catalogue from its source,
//! `typescript/error-catalogue.json`: an entry's own words, and the [`Retry`] kind of an entry whose
//! reader can send its request again. Only the template of a retry kind names a request ID, a
//! payment or a retry, so each step that repeats a request says it in the same reviewed words.
//! Where the senders of a code need different recoveries, the code takes [`Retry::Details`]: the
//! sender's `details.retry` names the recovery.
use alloc::string::String;
use alloc::vec::Vec;
use std::sync::OnceLock;

/// The exact bytes of the catalogue.
pub const ERROR_CATALOGUE_JSON: &str = include_str!("../resources/error-catalogue.json");

/// The path of a help page before its code, on the origin that serves the app.
pub const HELP_PATH: &str = "/help/";

/// Who reads an error and acts on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Audience {
    /// The wallet owner.
    Owner,
    /// The paying agent.
    Agent,
    /// The operator of the server.
    Operator,
}

/// What a reader does with the request that the error answered. Each kind has one reviewed
/// template in the catalogue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Retry {
    /// The request has no final outcome, or the server answers it from its kept outcome: send it
    /// unchanged, with the same request ID.
    Same,
    /// The outcome stands until the owner acts on the policy; then send the request unchanged.
    Owner,
    /// The request can have taken effect: read it first, and send a new one only once it ended
    /// without a payment.
    Final,
    /// The purchase has a payment that settled or is settling: do not pay for it again.
    Settled,
    /// The request is not sent again.
    Never,
    /// The senders of the code need different recoveries: the sender's `details.retry` names
    /// the recovery, and the template follows it.
    Details,
}

impl Retry {
    /// Every kind, in the order of the catalogue's templates.
    pub const ALL: [Retry; 6] = [
        Retry::Same,
        Retry::Owner,
        Retry::Final,
        Retry::Settled,
        Retry::Never,
        Retry::Details,
    ];
    /// The kind's name in the catalogue.
    pub fn name(self) -> &'static str {
        match self {
            Retry::Same => "same",
            Retry::Owner => "owner",
            Retry::Final => "final",
            Retry::Settled => "settled",
            Retry::Never => "none",
            Retry::Details => "details",
        }
    }
    fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

/// One typed error code with the text that every host shows for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub code: String,
    pub audience: Audience,
    /// One line that names the problem.
    pub title: String,
    /// Why it happened, in the owner's words.
    pub cause: String,
    /// What the owner does next on reading the error in the app. An entry for the agent ends it
    /// with the owner's sentence of its retry kind.
    pub next: String,
    /// What an agent that reads the error in a terminal does next, when that differs from `next`,
    /// with the agent's sentences of the retry kind. Every entry for the agent has it.
    pub agent: Option<String>,
    /// The retry kind, when the reader can send the request again or must not.
    pub retry: Option<Retry>,
}

impl Entry {
    /// The next step as an agent reads it in a terminal.
    pub fn agent_next(&self) -> &str {
        self.agent.as_deref().unwrap_or(&self.next)
    }
}

/// True for text that has the form of a code: 3 to 64 lowercase letters, digits and underscores,
/// starting with a letter. A code of this form is safe in a URL path and on a terminal.
pub fn is_code(text: &str) -> bool {
    let bytes = text.as_bytes();
    (3..=64).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
}

fn parse() -> Vec<Entry> {
    let document: serde_json::Value =
        serde_json::from_str(ERROR_CATALOGUE_JSON).expect("the error catalogue is JSON");
    let text = |entry: &serde_json::Value, key: &str| -> Option<String> {
        entry.get(key).and_then(|v| v.as_str()).map(String::from)
    };
    document["entries"]
        .as_array()
        .expect("the error catalogue has entries")
        .iter()
        .map(|entry| Entry {
            code: text(entry, "code").expect("an entry has a code"),
            audience: match entry["for"].as_str() {
                Some("owner") => Audience::Owner,
                Some("agent") => Audience::Agent,
                Some("operator") => Audience::Operator,
                other => panic!("unknown audience {other:?}"),
            },
            title: text(entry, "title").expect("an entry has a title"),
            cause: text(entry, "cause").expect("an entry has a cause"),
            next: text(entry, "next").expect("an entry has a next step"),
            agent: text(entry, "agent"),
            retry: text(entry, "retry")
                .map(|name| Retry::parse(&name).expect("a known retry kind")),
        })
        .collect()
}

/// Every entry, in the order of the catalogue.
pub fn catalogue() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(parse)
}

/// The entry of a typed error code. A newer server can send a code that this release does not
/// know: a host then shows the sender's text and still links the code's help page (`help_url`,
/// with the `client` feature).
pub fn entry(code: &str) -> Option<&'static Entry> {
    catalogue().iter().find(|entry| entry.code == code)
}

/// The origin that `text` names, when `text` is an HTTP or HTTPS origin written as a URL parser
/// writes it: `https://host` or `https://host:port`, with an optional final slash, and nothing
/// else. Text that a URL parser reads as another origin than the one it shows, such as a host in
/// capitals, a numeric host in another notation or a line break inside the host, gives none.
#[cfg(feature = "client")]
pub fn canonical_origin(text: &str) -> Option<String> {
    let url = url::Url::parse(text).ok()?;
    let plain = matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none();
    let origin = url.origin().ascii_serialization();
    (plain && text.strip_suffix('/').unwrap_or(text) == origin).then_some(origin)
}

/// The address of the help page of `code` on the app at `origin`, such as
/// `https://app.example/help/policy_denied`. Text that is no code gives the help index. An `origin`
/// that [`canonical_origin`] refuses gives no address.
#[cfg(feature = "client")]
pub fn help_url(origin: &str, code: &str) -> Option<String> {
    let origin = canonical_origin(origin)?;
    Some(if is_code(code) {
        alloc::format!("{origin}{HELP_PATH}{code}")
    } else {
        alloc::format!("{origin}{}", HELP_PATH.trim_end_matches('/'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Catalogue text is printable ASCII, trimmed, of 1 to `limit` characters: the generator
    /// (`scripts/error-catalogue.mjs`) applies the same rule.
    fn plain(text: &str, limit: usize) -> bool {
        !text.is_empty()
            && text.len() <= limit
            && text == text.trim()
            && text.bytes().all(|b| (0x20..=0x7e).contains(&b))
    }

    /// True for text that names its reader: an owner and an agent read the same title and cause.
    /// A word is a run of ASCII letters and digits, as in the generator's rule.
    fn names_reader(text: &str) -> bool {
        text.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| {
                ["you", "your", "yours", "yourself"].contains(&word.to_ascii_lowercase().as_str())
            })
    }

    #[test]
    fn hidden_characters_and_a_named_reader_are_refused() {
        for bad in [
            "a\u{85}b",
            "a\u{2028}b",
            "a\u{202e}b",
            "a\u{61c}b",
            "a\u{feff}b",
            "you\u{345}",
            "caf\u{e9}",
            "a\nb",
            " a",
        ] {
            assert!(!plain(bad, 70), "{bad:?}");
        }
        assert!(plain("The wallet did not sign", 70));
        assert!(names_reader("Your session ended"));
        assert!(names_reader("A later answer of yours stands"));
        // An underscore ends a word, as in the generator's rule.
        assert!(names_reader("The key that _you_ supplied did not match"));
        assert!(!names_reader("The owner's young account"));
    }

    /// The words of a text: runs of ASCII letters and digits, in lowercase, as the generator
    /// splits them.
    fn words(text: &str) -> Vec<String> {
        text.split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(str::to_ascii_lowercase)
            .collect()
    }

    /// The rule that an entry's own text in `field` breaks, `request`, `payment` or `retry`, or
    /// None, as the generator (`scripts/error-catalogue.mjs`) applies it: only the template of a
    /// retry kind names a request ID, a retry or a payment. A step (`next`, `agent`) names no ID, no
    /// payment and no retry, and the agent's step not "again"; the owner's step can say "again" of
    /// a step in the app, such as a sign-in. A title and a cause name no retry and no payment
    /// outcome; they can name an ID, because some codes, such as `request_id_binds_other_input`,
    /// are about one.
    fn word_rule(field: &str, text: &str) -> Option<&'static str> {
        const IDS: [&str; 2] = ["id", "ids"];
        const OUTCOME: [&str; 14] = [
            "paid",
            "unpaid",
            "settle",
            "settles",
            "settled",
            "settling",
            "settlement",
            "settlements",
            "unsettled",
            "charged",
            "refund",
            "refunds",
            "refunding",
            "refunded",
        ];
        const PAYMENT: [&str; 9] = [
            "pay", "pays", "paying", "repay", "repays", "repaying", "repaid", "payment", "payments",
        ];
        const RETRY: [&str; 8] = [
            "retry",
            "retries",
            "retried",
            "retrying",
            "resend",
            "resends",
            "resending",
            "resent",
        ];
        let list = words(text);
        let has = |set: &[&str]| list.iter().any(|word| set.contains(&word.as_str()));
        let step = field == "next" || field == "agent";
        if step && has(&IDS) {
            Some("request")
        } else if has(&OUTCOME) || (step && has(&PAYMENT)) {
            Some("payment")
        } else if has(&RETRY) || (field == "agent" && has(&["again"])) {
            Some("retry")
        } else {
            None
        }
    }

    #[test]
    fn the_word_rule_finds_each_kind_of_word() {
        for (field, text, rule) in [
            (
                "agent",
                "Send it with the same request ID.",
                Some("request"),
            ),
            ("next", "Open the ids list.", Some("request")),
            ("cause", "The request ID was used before.", None),
            ("title", "This payment is already settled", Some("payment")),
            ("cause", "Nothing was paid.", Some("payment")),
            (
                "cause",
                "The policy compares the price of this payment.",
                None,
            ),
            ("agent", "Do not pay twice.", Some("payment")),
            ("next", "Check the payment.", Some("payment")),
            ("agent", "Retry in a moment.", Some("retry")),
            ("next", "Resend the request.", Some("retry")),
            ("cause", "The server retried it.", Some("retry")),
            ("agent", "Send it again.", Some("retry")),
            ("next", "Sign in again.", None),
            // A word is a run of letters and digits: an underscore or a hyphen ends it.
            ("agent", "Use the request_id field.", Some("request")),
            ("agent", "Read the request's status before you act.", None),
        ] {
            assert_eq!(word_rule(field, text), rule, "{field}: {text}");
        }
    }

    #[test]
    fn the_word_rules_agree_with_the_generator() {
        // The generator's test reads the same cases, so both rule sets agree on each.
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/catalogue-text-vectors.json"
        ))
        .expect("the vectors are JSON");
        let cases = vectors["cases"].as_array().expect("cases");
        assert!(cases.len() > 20);
        for case in cases {
            let (field, text) = (
                case["field"].as_str().unwrap(),
                case["text"].as_str().unwrap(),
            );
            assert_eq!(
                word_rule(field, text),
                case["rule"].as_str(),
                "{field}: {text}"
            );
        }
    }

    /// The part of `composed` between `first` and `then`, or None when it is not so composed.
    fn between<'a>(composed: &'a str, first: &str, then: &str) -> Option<&'a str> {
        let middle = composed.strip_prefix(first)?.strip_suffix(then)?;
        Some(middle.trim())
    }

    #[test]
    fn each_step_is_its_own_words_and_the_template_of_its_retry_kind() {
        let document: serde_json::Value = serde_json::from_str(ERROR_CATALOGUE_JSON).unwrap();
        let templates = document["retry"].as_object().expect("the retry templates");
        // serde_json reads the keys of an object in sorted order; the generator checks their order.
        let kinds: BTreeSet<&str> = templates.keys().map(String::as_str).collect();
        assert_eq!(kinds, BTreeSet::from(Retry::ALL.map(Retry::name)));
        for (kind, template) in templates {
            let fields: BTreeSet<&str> = template
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(fields, BTreeSet::from(["first", "then", "owner"]), "{kind}");
        }
        let mut composed = 0;
        for (raw, entry) in document["entries"]
            .as_array()
            .unwrap()
            .iter()
            .zip(catalogue())
        {
            let kind = raw["retry"].as_str();
            assert_eq!(kind, entry.retry.map(Retry::name), "{}", entry.code);
            for (field, text) in [("title", &entry.title), ("cause", &entry.cause)] {
                assert_eq!(word_rule(field, text), None, "{} {field}", entry.code);
            }
            // The entry's own words of each step: without a retry kind the whole step, with one
            // what stands between the template's sentences.
            let (next, agent) = match kind {
                None => (entry.next.as_str(), entry.agent.as_deref().unwrap_or("")),
                Some(kind) => {
                    composed += 1;
                    let template = &templates[kind];
                    let text = |key: &str| template[key].as_str().unwrap();
                    let owner = if entry.audience == Audience::Agent {
                        text("owner")
                    } else {
                        ""
                    };
                    let agent = entry.agent.as_deref().expect("a composed agent step");
                    (
                        between(&entry.next, "", owner).unwrap_or_else(|| {
                            panic!("{} next ends with its template", entry.code)
                        }),
                        between(agent, text("first"), text("then"))
                            .unwrap_or_else(|| panic!("{} agent holds its template", entry.code)),
                    )
                }
            };
            assert!(plain(next, 260), "{} next", entry.code);
            assert_eq!(word_rule("next", next), None, "{} next: {next}", entry.code);
            assert!(
                agent.is_empty() || plain(agent, 260),
                "{} agent",
                entry.code
            );
            assert_eq!(
                word_rule("agent", agent),
                None,
                "{} agent: {agent}",
                entry.code
            );
        }
        assert!(composed > 40, "{composed} entries have a retry kind");
    }

    #[test]
    fn titles_and_causes_name_no_reader() {
        for entry in catalogue() {
            assert!(!names_reader(&entry.title), "{} title", entry.code);
            assert!(!names_reader(&entry.cause), "{} cause", entry.code);
        }
    }

    #[test]
    fn every_entry_has_one_code_and_plain_bounded_text() {
        let entries = catalogue();
        assert!(entries.len() > 100);
        let mut seen = BTreeSet::new();
        for entry in entries {
            assert!(is_code(&entry.code), "{}", entry.code);
            assert!(seen.insert(entry.code.as_str()), "{} twice", entry.code);
            assert!(plain(&entry.title, 70), "{} title", entry.code);
            assert!(!entry.title.ends_with(['.', '!']), "{} title", entry.code);
            assert!(plain(&entry.cause, 260), "{} cause", entry.code);
            // A composed step holds the entry's own words and the sentences of its retry kind.
            assert!(plain(&entry.next, 600), "{} next", entry.code);
            // The owner reads `next` in the app; an entry for the agent states the agent's own step.
            if entry.audience == Audience::Agent {
                assert!(entry.agent.is_some(), "{} has no agent step", entry.code);
            }
            if let Some(agent) = &entry.agent {
                assert!(plain(agent, 600), "{} agent", entry.code);
            }
            assert_eq!(
                entry.agent_next(),
                entry.agent.as_deref().unwrap_or(&entry.next)
            );
        }
    }

    #[test]
    fn the_catalogue_states_the_help_path_and_only_known_fields() {
        let document: serde_json::Value = serde_json::from_str(ERROR_CATALOGUE_JSON).unwrap();
        assert_eq!(document["schemaVersion"], 1);
        assert_eq!(document["helpPath"], HELP_PATH);
        let keys: BTreeSet<&str> = document
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            BTreeSet::from(["schemaVersion", "helpPath", "retry", "entries"])
        );
        for entry in document["entries"].as_array().unwrap() {
            for (key, value) in entry.as_object().unwrap() {
                assert!(
                    ["code", "for", "title", "cause", "next", "agent", "retry"]
                        .contains(&key.as_str()),
                    "{key}"
                );
                // Every field that is present is text: `parse` would read an `agent` of null or a
                // number as no agent step, which the generator refuses.
                assert!(value.is_string(), "{key} of {} is no text", entry["code"]);
            }
        }
    }

    /// The files whose string constants are policy parameter and state names, not error codes.
    const POLICY_NAME_FILES: [&str; 2] = ["portable/core.rs", "policy_source/derive.rs"];
    /// The other string constants of code form that are no error code: JSON keys and names of
    /// the policy language, by file and constant.
    const NOT_CODES: [(&str, &str); 4] = [
        ("policy_source.rs", "SERVER_TEXT"),
        ("policy_source.rs", "INDEXED_FIELD"),
        ("vendor/execution.rs", "KEY"),
        ("vendor/execution.rs", "EXPLORER_KEY"),
    ];

    /// Each string constant of code form in `text`, as (name, value): a line such as
    /// `pub const NAME: &str = "value";`.
    fn code_constants(text: &str) -> Vec<(&str, &str)> {
        text.lines()
            .filter_map(|line| {
                let line = line.trim_start();
                let line = ["pub(crate) ", "pub(super) ", "pub "]
                    .iter()
                    .find_map(|prefix| line.strip_prefix(prefix))
                    .unwrap_or(line);
                let (name, rest) = line.strip_prefix("const ")?.split_once(':')?;
                let rest = rest.trim_start();
                let rest = ["&str", "&'static str"]
                    .iter()
                    .find_map(|ty| rest.strip_prefix(ty))?;
                let value = rest.trim().strip_prefix("= \"")?.strip_suffix("\";")?;
                is_code(value).then_some((name.trim(), value))
            })
            .collect()
    }

    #[test]
    fn this_crate_has_an_entry_for_each_code_that_it_defines() {
        // Every string constant of code form in this crate's source is an error code, apart from
        // the listed names: a new code constant without an entry fails here.
        fn files(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
            for item in std::fs::read_dir(dir).expect("the source folder") {
                let path = item.expect("a source file").path();
                if path.is_dir() {
                    files(&path, found);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    found.push(path);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut sources = Vec::new();
        files(&root, &mut sources);
        let mut codes = BTreeSet::new();
        for path in sources {
            let file = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if POLICY_NAME_FILES.contains(&file.as_str()) {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for (name, value) in code_constants(&text) {
                if NOT_CODES.contains(&(file.as_str(), name)) {
                    continue;
                }
                assert!(
                    entry(value).is_some(),
                    "{file} {name} = {value} has no entry"
                );
                codes.insert(value.to_owned());
            }
        }
        // The custom build's six codes, the owner session's, the runtime's and the proof's.
        assert!(codes.len() >= 9, "{codes:?}");
        assert!(codes.contains("request_proof_expired"), "{codes:?}");
        assert_eq!(
            code_constants(
                "pub const A: &str = \"abc_def\";\n    const B: &'static str = \"xyz\";\nconst C: &str = \"/x/\";\nlet d = \"abc\";"
            ),
            [("A", "abc_def"), ("B", "xyz")]
        );
        assert!(entry("no_such_code").is_none());
        let denied = entry("policy_denied").unwrap();
        assert_eq!(denied.audience, Audience::Agent);
        assert_ne!(denied.agent_next(), denied.next);
    }

    #[cfg(feature = "client")]
    #[test]
    fn a_help_address_carries_only_a_code() {
        assert_eq!(
            help_url("https://app.example/", "policy_denied").as_deref(),
            Some("https://app.example/help/policy_denied")
        );
        // A newer server's code has no entry yet and still has an address.
        assert_eq!(
            help_url("http://127.0.0.1:5173", "new_code_1").as_deref(),
            Some("http://127.0.0.1:5173/help/new_code_1")
        );
        for unsafe_text in [
            "",
            "ab",
            "../x",
            "a b",
            "A_CODE",
            "x\n",
            "1abc",
            "a/b",
            "\u{e9}_code",
        ] {
            assert_eq!(
                help_url("https://app.example", unsafe_text).as_deref(),
                Some("https://app.example/help"),
                "{unsafe_text:?}"
            );
        }
        assert!(!is_code(&"a".repeat(65)));
    }

    #[cfg(feature = "client")]
    #[test]
    fn a_help_address_needs_an_origin_as_a_url_parser_writes_it() {
        for origin in [
            "https://app.example",
            "https://app.example/",
            "http://localhost:8080",
            "https://[::1]:8443",
            "https://xn--bcher-kva.example",
            "https://app.example.",
            "http://192.168.1.10:8080",
            "https://[2001:db8::1]",
        ] {
            assert!(canonical_origin(origin).is_some(), "{origin}");
            assert!(help_url(origin, "policy_denied").is_some(), "{origin}");
        }
        for origin in [
            "",
            "app.example",
            "ftp://app.example",
            "https://app.example/?mode=preview",
            "https://app.example/base",
            "https://app.example#x",
            "https://user@app.example",
            "https://app.example:65536",
            "https://app.example:443",
            "https://APP.example",
            "https://app\n.example",
            "https://app.example\n",
            "https://app .example",
            "https://[::1",
            "https://[]",
            "https://[:::]",
            "https://[zz::1]",
            "https://999.999.999.999",
            "https://1.2.3.256",
            "https://0x7f000001",
            "https://app.0x1",
            "https://b\u{fc}cher.example",
            "https://",
        ] {
            assert_eq!(canonical_origin(origin), None, "{origin:?}");
            assert_eq!(help_url(origin, "policy_denied"), None, "{origin:?}");
        }
    }
}
