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
    /// What the reader does next. In the app the reader is the owner.
    pub next: String,
    /// What an agent that reads the error in a terminal does next, when that differs from `next`.
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
/// know: a host then shows the sender's text and still links [`help_url`].
pub fn entry(code: &str) -> Option<&'static Entry> {
    catalogue().iter().find(|entry| entry.code == code)
}

/// True for an HTTP or HTTPS origin and nothing else: `https://host` or `https://host:port`, with an
/// optional final slash. The host is a DNS name, an IPv4 address or a bracketed IPv6 address. A path,
/// a query, a fragment, user information, a space or a control character makes it no origin.
pub fn is_origin(text: &str) -> bool {
    let text = text.strip_suffix('/').unwrap_or(text);
    let Some(authority) = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"))
    else {
        return false;
    };
    let (host, port) = match authority.strip_prefix('[') {
        Some(bracketed) => {
            let Some((address, after)) = bracketed.split_once(']') else {
                return false;
            };
            let ipv6 = !address.is_empty()
                && address.contains(':')
                && address
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.');
            if !ipv6 {
                return false;
            }
            match after {
                "" => (address, None),
                _ => match after.strip_prefix(':') {
                    Some(port) => (address, Some(port)),
                    None => return false,
                },
            }
        }
        None => {
            let (host, port) = match authority.split_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (authority, None),
            };
            let name = !host.is_empty()
                && host.len() <= 253
                && host.split('.').all(|label| {
                    !label.is_empty()
                        && label.len() <= 63
                        && !label.starts_with('-')
                        && !label.ends_with('-')
                        && label
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                });
            if !name {
                return false;
            }
            (host, port)
        }
    };
    let port = port.is_none_or(|port| {
        (1..=5).contains(&port.len())
            && port.bytes().all(|b| b.is_ascii_digit())
            && port.parse::<u16>().is_ok_and(|n| n > 0)
    });
    !host.is_empty() && port
}

/// The address of the help page of `code` on the app at `origin`, such as
/// `https://app.example/help/policy_denied`. Text that is no code gives the help index. An `origin`
/// that [`is_origin`] refuses gives no address.
pub fn help_url(origin: &str, code: &str) -> Option<String> {
    if !is_origin(origin) {
        return None;
    }
    let origin = origin.trim_end_matches('/');
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

    /// A character that does not belong in one line of text: a control (C0, DEL, C1), a line or
    /// paragraph separator, a direction mark or override, or the byte order mark. The generator
    /// (`scripts/error-catalogue.mjs`) refuses the same characters.
    fn hidden(c: char) -> bool {
        c.is_control()
            || matches!(c, '\u{200e}' | '\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{feff}')
    }

    fn plain(text: &str, limit: usize) -> bool {
        !text.is_empty()
            && text.chars().count() <= limit
            && text == text.trim()
            && !text.chars().any(hidden)
    }

    /// True for text that names its reader: an owner and an agent read the same title and cause.
    fn names_reader(text: &str) -> bool {
        text.split(|c: char| !c.is_alphanumeric()).any(|word| {
            ["you", "your", "yours", "yourself"].contains(&word.to_lowercase().as_str())
        })
    }

    #[test]
    fn hidden_characters_and_a_named_reader_are_refused() {
        for bad in [
            "a\u{85}b",
            "a\u{2028}b",
            "a\u{202e}b",
            "a\u{feff}b",
            "a\nb",
            " a",
        ] {
            assert!(!plain(bad, 70), "{bad:?}");
        }
        assert!(plain("The wallet did not sign", 70));
        assert!(names_reader("Your session ended"));
        assert!(names_reader("A later answer of yours stands"));
        assert!(!names_reader("The owner's young account"));
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
            for key in entry.as_object().unwrap().keys() {
                assert!(
                    ["code", "for", "title", "cause", "next", "agent"].contains(&key.as_str()),
                    "{key}"
                );
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
            ] {
                assert!(entry(defined).is_some(), "{defined} has no catalogue entry");
            }
        }
        assert!(entry("no_such_code").is_none());
        let denied = entry("policy_denied").unwrap();
        assert_eq!(denied.audience, Audience::Agent);
        assert_ne!(denied.agent_next(), denied.next);
    }

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
            "", "ab", "../x", "a b", "A_CODE", "x\n", "1abc", "a/b", "é_code",
        ] {
            assert_eq!(
                help_url("https://app.example", unsafe_text).as_deref(),
                Some("https://app.example/help"),
                "{unsafe_text:?}"
            );
        }
        assert!(!is_code(&"a".repeat(65)));
    }

    #[test]
    fn a_help_address_needs_an_origin() {
        for origin in [
            "https://app.example",
            "https://app.example/",
            "http://localhost:8080",
            "https://[::1]:8443",
            "https://xn--bcher-kva.example",
        ] {
            assert!(is_origin(origin), "{origin}");
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
            "https://app.example:0",
            "https://app.example:65536",
            "https://app.example:",
            "https://app\n.example",
            "https://app.example\n",
            "https://app .example",
            "https://-app.example",
            "https://[::1",
            "https://[]",
            "https://[zz::1]",
            "https://",
        ] {
            assert!(!is_origin(origin), "{origin:?}");
            assert_eq!(help_url(origin, "policy_denied"), None, "{origin:?}");
        }
    }
}
