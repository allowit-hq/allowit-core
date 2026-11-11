//! Owner HTTP session coordination. Native adapters verify and sign the exact challenge.
use crate::transport::{
    http::{HttpClient, Method, Request, Response},
    rpc::Options,
};
use crate::vendor::{
    config::parse_origin,
    error::{Error, Result},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::{Duration, Instant};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use zeroize::Zeroizing;

/// Bounded HTTP session workflow shared by owner adapters.
pub struct Client {
    http: HttpClient,
    backend: String,
    origin: String,
    owner: String,
    session: Option<OwnerSession>,
}
struct OwnerSession {
    cookie: Zeroizing<String>,
    expires: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
/// Exact server challenge passed to the selected native signing capability.
pub struct Challenge {
    pub id: String,
    pub message: String,
    pub address: String,
    pub origin: String,
    pub expires_at: String,
}
#[derive(Deserialize)]
struct Authenticated {
    session: SessionInfo,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SessionInfo {
    pub address: String,
    network: String,
    pub expires_at: String,
}
pub fn timestamp(value: &str) -> Result<u64> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| Error::config("Invalid owner authentication timestamp"))?;
    if parsed.format(&Rfc3339).ok().as_deref() != Some(value) {
        return Err(Error::config("Noncanonical owner authentication timestamp"));
    }
    u64::try_from(parsed.unix_timestamp())
        .map_err(|_| Error::config("Invalid owner authentication timestamp"))
}
fn cookie(response: &Response, backend: &str) -> Result<Zeroizing<String>> {
    let headers: Vec<_> = response.header_values("set-cookie").collect();
    if headers.len() != 1 {
        return Err(Error::config(
            "Owner authentication requires one unambiguous session cookie",
        ));
    }
    let raw = std::str::from_utf8(headers[0])
        .map_err(|_| Error::config("Invalid owner session cookie"))?;
    if !raw.bytes().all(|b| (32..127).contains(&b)) {
        return Err(Error::config("Invalid owner session cookie"));
    }
    let mut fields = raw.split(';').map(str::trim);
    let pair = fields
        .next()
        .ok_or_else(|| Error::config("Missing owner session cookie"))?;
    let (name, value) = pair
        .split_once('=')
        .ok_or_else(|| Error::config("Invalid owner session cookie"))?;
    if !matches!(name, "allowit_session" | "__Host-allowit")
        || value.len() != 32
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err(Error::config("Invalid owner session cookie"));
    }
    let attributes: Vec<_> = fields.map(str::to_ascii_lowercase).collect();
    let mut seen = std::collections::BTreeSet::new();
    if attributes
        .iter()
        .any(|a| !seen.insert(a.split('=').next().unwrap_or("")))
    {
        return Err(Error::config("Ambiguous owner session cookie attributes"));
    }
    if !attributes.iter().any(|a| a == "path=/")
        || !attributes.iter().any(|a| a == "httponly")
        || !attributes.iter().any(|a| a == "samesite=lax")
        || attributes.iter().any(|a| a.starts_with("domain="))
        || (name == "__Host-allowit"
            && (!backend.starts_with("https://") || !attributes.iter().any(|a| a == "secure")))
    {
        return Err(Error::config("Unsafe owner session cookie scope"));
    }
    Ok(Zeroizing::new(pair.to_owned()))
}
impl Client {
    pub fn backend(&self) -> &str {
        &self.backend
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn new(backend: &str, origin: &str, owner: &str) -> Result<Self> {
        if origin.is_empty() {
            return Err(Error::config("Set the explicit owner sign-in Origin"));
        }
        Ok(Self {
            http: HttpClient::new(Options {
                timeout: Duration::from_secs(180),
                max_response_bytes: 1_048_576,
                no_proxy: false,
                connect_timeout: None,
            })
            .map_err(|_| Error::config("Cannot initialize owner HTTP transport"))?,
            backend: parse_origin(backend)?,
            origin: parse_origin(origin)?,
            owner: owner.into(),
            session: None,
        })
    }
    fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
        session: Option<&str>,
        uncertain: bool,
    ) -> Result<Response> {
        let mut headers = vec![("Origin".into(), self.origin.clone())];
        if let Some(cookie) = session {
            headers.push(("Cookie".into(), cookie.into()));
        }
        let bytes = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|_| Error::config("Cannot encode owner request"))?;
        if bytes.is_some() {
            headers.push(("Content-Type".into(), "application/json".into()));
        }
        let response=self.http.send(Request{method,url:&format!("{}{path}",self.backend),headers:&headers,body:bytes.as_deref()},if uncertain {None} else {Some(Instant::now()+Duration::from_secs(30))})
            .map_err(|_|if uncertain {Error::uncertain("Policy creation is uncertain; inspect the owner workspace before any new generation")} else {Error::config("Owner server request is unavailable")})?;
        if !(200..300).contains(&response.status) {
            return Err(refusal(response.status, &response.body, uncertain));
        }
        Ok(response)
    }
    fn decode<T: serde::de::DeserializeOwned>(response: &Response, uncertain: bool) -> Result<T> {
        serde_json::from_slice(&response.body).map_err(|_|if uncertain {Error::uncertain("Invalid generation response; inspect the original owner workspace before retrying")} else {Error::config("Invalid owner server response")})
    }
    pub fn authenticate(
        &mut self,
        network: &str,
        now: u64,
        sign: impl FnOnce(&Challenge) -> Result<String>,
    ) -> Result<()> {
        self.session = None;
        let response = self.send(
            Method::Post,
            "/api/auth/challenge",
            Some(&json!({"address":self.owner})),
            None,
            false,
        )?;
        let guest = cookie(&response, &self.backend)?;
        let challenge: Challenge = Self::decode(&response, false)?;
        let expiry = timestamp(&challenge.expires_at)?;
        if challenge.id.len() != 64
            || !challenge
                .id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || challenge.address != self.owner
            || challenge.origin != self.origin
            || expiry <= now
        {
            return Err(Error::config(
                "Owner sign-in challenge is invalid or expired",
            ));
        }
        let signature = sign(&challenge)?;
        let response = self.send(Method::Post, "/api/auth/verify", Some(&json!({"id":challenge.id,"address":self.owner,"message":challenge.message,"signature":signature})), Some(&guest), false)?;
        let authenticated: Authenticated = Self::decode(&response, false)?;
        let rotated = cookie(&response, &self.backend)?;
        let expires = timestamp(&authenticated.session.expires_at)?;
        if rotated.as_str() == guest.as_str()
            || authenticated.session.address != self.owner
            || authenticated.session.network != network
            || expires <= now
            || expires > now.saturating_add(86_490)
        {
            return Err(Error::config(
                "Owner session was not safely rotated for this account",
            ));
        }
        self.session = Some(OwnerSession {
            cookie: rotated,
            expires,
        });
        Ok(())
    }
    pub fn owner_request<T: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
        now: u64,
        uncertain: bool,
    ) -> Result<T> {
        let session = self
            .session
            .as_ref()
            .filter(|s| s.expires > now)
            .ok_or_else(|| Error::config("Authenticate the selected owner first"))?;
        Self::decode(
            &self.send(method, path, body, Some(&session.cookie), uncertain)?,
            uncertain,
        )
    }
}

/// The server code of a request that the server did not send to the network.
pub const NETWORK_UNAVAILABLE: &str = "network_unavailable";

/// The error for a non-2xx owner reply. A typed body `{error, code, details}` keeps its `code` in
/// [`Error::api_code`] and its `error` text in the message. An untyped server error of a
/// state-changing request leaves the outcome unknown. A typed `network_unavailable` with
/// `details.retry = "same_request_id"` states that nothing was sent, so it is not uncertain.
fn refusal(status: u16, body: &[u8], uncertain: bool) -> Error {
    let reply: Option<Value> = serde_json::from_slice(body).ok();
    let text = |value: &Value, limit: usize| {
        value
            .as_str()
            .filter(|v| !v.is_empty() && v.len() <= limit && !v.chars().any(char::is_control))
            .map(String::from)
    };
    let code = reply
        .as_ref()
        .and_then(|r| text(&r["code"], 64))
        .filter(|c| {
            c.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        });
    let message = reply
        .as_ref()
        .and_then(|r| text(&r["error"], 300))
        .unwrap_or_else(|| "Owner request was refused".into());
    if uncertain && status >= 500 {
        let unsent = code.as_deref() == Some(NETWORK_UNAVAILABLE)
            && reply
                .as_ref()
                .is_some_and(|r| r["details"]["retry"] == "same_request_id");
        if !unsent {
            return Error::uncertain(
                "Policy creation returned a server error; inspect the owner workspace before any new generation",
            );
        }
    }
    match code {
        Some(code) => Error::refused(status, code, message),
        None => Error::api(status, "Owner request was refused"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_refusals_keep_the_server_code_and_only_unknown_outcomes_are_uncertain() {
        let body = |code: &str| {
            serde_json::to_vec(
                &json!({"error":"The owner transaction expired","code":code,"details":{}}),
            )
            .unwrap()
        };
        for code in [
            "owner_transaction_expired",
            "owner_transaction_pending",
            "owner_transaction_settled",
        ] {
            let error = refusal(409, &body(code), true);
            assert_eq!(error.api_code.as_deref(), Some(code));
            assert_eq!((error.code, error.status), (4, Some(409)));
            assert!(
                error.message.ends_with("The owner transaction expired"),
                "{error}"
            );
        }
        // An untyped server error of a state-changing request is uncertain; a read is not.
        assert_eq!(refusal(502, b"bad gateway", true).code, 5);
        assert_eq!(refusal(503, &body("other"), true).code, 5);
        assert_eq!(refusal(502, b"bad gateway", false).status, Some(502));
        // The server states that nothing was sent.
        let unsent = json!({"error":"Network unavailable","code":NETWORK_UNAVAILABLE,"details":{"retry":"same_request_id"}});
        let error = refusal(503, &serde_json::to_vec(&unsent).unwrap(), true);
        assert_eq!(error.api_code.as_deref(), Some(NETWORK_UNAVAILABLE));
        assert_eq!(refusal(503, &body(NETWORK_UNAVAILABLE), true).code, 5);
        // Server text reaches the caller only as a bounded code and message.
        for hostile in [
            json!({"code":"Bad Code","error":"x"}),
            json!({"code":7}),
            json!({"code":"a".repeat(65)}),
        ] {
            let error = refusal(409, &serde_json::to_vec(&hostile).unwrap(), true);
            assert_eq!(error.api_code, None);
            assert!(error.message.ends_with("Owner request was refused"));
        }
        let control = json!({"code":"denied","error":"line\u{1b}[31m"});
        let error = refusal(409, &serde_json::to_vec(&control).unwrap(), true);
        assert!(error.message.ends_with("Owner request was refused"));
        assert_eq!(error.api_code.as_deref(), Some("denied"));
    }
}
