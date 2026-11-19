//! The error catalogue: what each typed error code means and what its reader does next.
//!
//! A server, a rail and a client send a stable code with a refusal. Every host shows the
//! catalogue's text for that code and links the help page of the code, and keeps the sender's own
//! text for a details view. A host holds no error text of its own. The TypeScript package exports
//! the same bytes as `errorCatalogue`.
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

    /// The first place of `phrase` in `text` between word boundaries, where a word character is
    /// an ASCII letter, a digit or an underscore: the boundary of the generator's `\b`.
    fn find_word(text: &str, phrase: &str) -> Option<usize> {
        let word = |b: Option<&u8>| b.is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_');
        let bytes = text.as_bytes();
        text.match_indices(phrase).map(|(at, _)| at).find(|&at| {
            !word(at.checked_sub(1).and_then(|i| bytes.get(i)))
                && !word(bytes.get(at + phrase.len()))
        })
    }

    /// The rule that a text of an entry breaks, `outcome` or `original`, as the generator
    /// (`scripts/error-catalogue.mjs`) applies it: an entry states no outcome, which only the
    /// sender knows, and a step that sends a new request ID reads the original request before it.
    fn unsafe_text(text: &str) -> Option<&'static str> {
        let lower = text.to_ascii_lowercase();
        // An outcome of a step; "Nothing was lost", the owner's words for the text that stays in
        // the form, is none.
        let step = [
            "paid",
            "sent",
            "signed",
            "saved",
            "changed",
            "charged",
            "prepared",
            "imported",
            "evaluated",
        ];
        let claims_outcome = ["nothing was ", "nothing is ", "nothing were "]
            .iter()
            .any(|start| {
                step.iter()
                    .any(|outcome| find_word(&lower, &format!("{start}{outcome}")).is_some())
            });
        if claims_outcome
            || find_word(&lower, "no draft changed").is_some()
            || find_word(&lower, "was not made").is_some()
        {
            return Some("outcome");
        }
        if let Some(id) = find_word(text, "new request ID") {
            let check = [
                find_word(text, "allowit status"),
                find_word(text, "checks the original request"),
            ]
            .into_iter()
            .flatten()
            .min();
            if check.is_none_or(|check| check > id) {
                return Some("original");
            }
        }
        None
    }

    #[test]
    fn no_entry_states_an_outcome_or_retries_without_the_original() {
        // The generator's test reads the same cases, so both rule sets agree on each.
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/catalogue-text-vectors.json"
        ))
        .expect("the vectors are JSON");
        let cases = vectors["cases"].as_array().expect("cases");
        assert!(cases.len() > 10);
        for case in cases {
            let text = case["text"].as_str().expect("text");
            assert_eq!(unsafe_text(text), case["rule"].as_str(), "{text}");
        }
        for entry in catalogue() {
            for text in [&entry.title, &entry.cause, &entry.next]
                .into_iter()
                .chain(entry.agent.as_ref())
            {
                assert_eq!(unsafe_text(text), None, "{}: {text}", entry.code);
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
                code::BUILD_MISSING,
                crate::vendor::owner_session::NETWORK_UNAVAILABLE,
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
