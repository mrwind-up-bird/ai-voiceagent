//! Shared HTTP client for nyxCore integrations.
//!
//! Carries the M6 fix forward inline (connect/timeout) ahead of the
//! global agents-wide rollout — we'd rather not ship Persona/Axiom
//! integration with the same hang-forever default as the legacy
//! agents. A single OnceCell client is shared by both persona.rs and
//! axiom.rs to amortise the TLS pool.

use once_cell::sync::OnceCell;
use reqwest::Client;
use std::time::Duration;

static CLIENT: OnceCell<Client> = OnceCell::new();

/// Connect timeout for the initial TCP+TLS handshake.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Total per-request timeout (includes body read).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub fn get_client() -> &'static Client {
    CLIENT.get_or_init(|| {
        Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

/// Default nyxCore base URL. Overridden at runtime via the optional
/// keychain slot `nyxcore_base_url`. Local-dev default points to the
/// Next.js server in the sibling `nyxcore-systems` project.
pub const DEFAULT_NYXCORE_BASE_URL: &str = "http://localhost:3000";

/// Resolve the effective nyxCore base URL: prefer a user-configured
/// value from the keychain, fall back to the local-dev default.
pub fn base_url() -> String {
    crate::secrets::get_key_or_error("nyxcore_base_url")
        .unwrap_or_else(|_| DEFAULT_NYXCORE_BASE_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_url_has_no_trailing_slash() {
        assert!(!DEFAULT_NYXCORE_BASE_URL.ends_with('/'));
    }

    #[test]
    fn client_is_constructible_and_singleton() {
        let a = get_client() as *const _;
        let b = get_client() as *const _;
        assert_eq!(a, b, "OnceCell must hand out the same Client");
    }
}
