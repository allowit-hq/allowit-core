//! The error catalogue: what each typed error code means and what its reader does next.
//!
//! A server, a rail and a client send a stable code with a refusal. Every host shows the
//! catalogue's text for that code and links the help page of the code, and keeps the sender's own
//! text for a details view. A host holds no error text of its own. The TypeScript package exports
//! the same bytes as `errorCatalogue`.
//!
//! The generator (`scripts/generate-presentation.mjs`) checks the source,
//! `typescript/error-catalogue.json`, and writes it here. An entry names the problem, its cause and
//! the reader's next step; it says nothing about request IDs, retries or payment outcomes, which
//! the sender's own details state.
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

/// One typed error code with the text that every host shows for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub code: String,
    pub audience: Audience,
    /// One line that names the problem.
    pub title: String,
    /// Why it happened, in the owner's words.
    pub cause: String,
    /// What the owner does next on reading the error in the app.
    pub next: String,
    /// What an agent that reads the error in a terminal does next, when that differs from `next`.
    /// Every entry for the agent has it.
    pub agent: Option<String>,
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

    /// The rule that the text of `field` breaks, `request`, `payment` or `retry`, or None. An entry
    /// says nothing about request IDs, retries or payment outcomes: the sender's details state
    /// those. A step (`next`, `agent`) names no ID, no payment and no retry, and the agent's step
    /// not "again"; the owner's step can say "again" of a step in the app, such as a sign-in. A
    /// title and a cause name no retry and no payment outcome; they can name an ID, because some
    /// codes, such as `request_id_binds_other_input`, are about one.
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
    fn no_entry_names_a_request_id_a_retry_or_a_payment_outcome() {
        for entry in catalogue() {
            let mut fields = vec![
                ("title", entry.title.as_str()),
                ("cause", entry.cause.as_str()),
                ("next", entry.next.as_str()),
            ];
            fields.extend(entry.agent.as_deref().map(|agent| ("agent", agent)));
            for (field, text) in fields {
                assert_eq!(
                    word_rule(field, text),
                    None,
                    "{} {field}: {text}",
                    entry.code
                );
            }
        }
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
            assert!(plain(&entry.next, 260), "{} next", entry.code);
            // The owner reads `next` in the app; an entry for the agent states the agent's own step.
            if entry.audience == Audience::Agent {
                assert!(entry.agent.is_some(), "{} has no agent step", entry.code);
            }
            if let Some(agent) = &entry.agent {
                assert!(plain(agent, 260), "{} agent", entry.code);
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
            BTreeSet::from(["schemaVersion", "helpPath", "entries"])
        );
        for entry in document["entries"].as_array().unwrap() {
            for (key, value) in entry.as_object().unwrap() {
                assert!(
                    ["code", "for", "title", "cause", "next", "agent"].contains(&key.as_str()),
                    "{key}"
                );
                // Every field that is present is text: `parse` would read an `agent` of null or a
                // number as no agent step, which the generator refuses.
                assert!(value.is_string(), "{key} of {} is no text", entry["code"]);
            }
        }
    }

    #[test]
    fn this_crate_has_an_entry_for_each_code_that_it_defines() {
        #[cfg(feature = "vendor")]
        {
            use crate::vendor::custom_build::code;
            for defined in [
                code::UNKNOWN_POLICY,
                code::CODE_HASH_MISMATCH,
                code::TOOLCHAIN_MISSING,
                code::BUILD_FAILED,
                code::SOURCE_REJECTED,
                code::BUILD_MISSING,
                crate::vendor::owner_session::NETWORK_UNAVAILABLE,
                crate::vendor::runtime::CONTRACT_EXPIRED,
                crate::vendor::request_proof::REQUEST_PROOF_EXPIRED,
            ] {
                assert!(entry(defined).is_some(), "{defined} has no catalogue entry");
            }
        }
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
