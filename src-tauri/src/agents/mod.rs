pub mod action_items;
pub mod tone_shifter;
pub mod music_matcher;
pub mod translator;
pub mod dev_log;
pub mod brain_dump;
pub mod mental_mirror;

pub use action_items::*;
pub use tone_shifter::*;
pub use music_matcher::*;
pub use translator::*;
pub use dev_log::*;
pub use brain_dump::*;
pub use mental_mirror::*;

/// C5 — classify an HTTP error status into a user-readable, actionable
/// message instead of the previous one-size-fits-all "Service
/// temporarily unavailable". Frontend can deep-link to Settings when
/// the message indicates auth failure.
pub fn classify_api_error(status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 | 403 => {
            "Authentication failed. Please check your API key in Settings.".to_string()
        }
        402 => "API billing issue. Please check your account.".to_string(),
        429 => {
            "Rate limit hit. Please wait a moment and try again.".to_string()
        }
        408 | 504 => "Request timed out. Please try again.".to_string(),
        500..=599 => format!(
            "Service outage ({}). Please try again in a moment.",
            status.as_u16()
        ),
        other => format!("Unexpected error ({}). Please try again.", other),
    }
}

#[cfg(test)]
mod tests {
    use super::classify_api_error;
    use reqwest::StatusCode;

    #[test]
    fn classify_auth_failures() {
        assert!(classify_api_error(StatusCode::UNAUTHORIZED).contains("API key"));
        assert!(classify_api_error(StatusCode::FORBIDDEN).contains("API key"));
    }

    #[test]
    fn classify_rate_limit() {
        let msg = classify_api_error(StatusCode::TOO_MANY_REQUESTS);
        assert!(msg.contains("Rate limit") || msg.contains("rate"));
    }

    #[test]
    fn classify_billing() {
        let msg = classify_api_error(StatusCode::PAYMENT_REQUIRED);
        assert!(msg.contains("billing") || msg.contains("account"));
    }

    #[test]
    fn classify_server_error_mentions_status_code() {
        let msg = classify_api_error(StatusCode::INTERNAL_SERVER_ERROR);
        assert!(msg.contains("500"));
        let msg2 = classify_api_error(StatusCode::BAD_GATEWAY);
        assert!(msg2.contains("502"));
    }

    #[test]
    fn classify_timeout() {
        assert!(classify_api_error(StatusCode::REQUEST_TIMEOUT).contains("timed out"));
        assert!(classify_api_error(StatusCode::GATEWAY_TIMEOUT).contains("timed out"));
    }

    #[test]
    fn classify_unknown_falls_through() {
        let msg = classify_api_error(StatusCode::from_u16(418).unwrap());
        assert!(msg.contains("418"));
    }
}
