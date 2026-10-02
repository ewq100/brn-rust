use crate::{AiError, AiErrorKind};
use rig::http_client;
use rig::providers::chatgpt::auth::AuthError;

const MAX_DELAY_SECONDS: u64 = 365 * 24 * 60 * 60;

impl AiError {
    pub(crate) fn new(kind: AiErrorKind) -> Self {
        Self {
            kind,
            retry_after_seconds: None,
        }
    }
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self.kind {
            AiErrorKind::ReconnectNeeded => "Reconnect this account before continuing.",
            AiErrorKind::CodeExpired => "Login code expired. Retry Connect to obtain a new code.",
            AiErrorKind::RateLimited => "Account usage limit reached.",
            AiErrorKind::Network => "Network request failed.",
            AiErrorKind::ModelRefused => "The selected model is not supported.",
            AiErrorKind::InvalidToolUse => "The model requested an invalid tool.",
            AiErrorKind::ToolLimitReached => "The tool round limit was reached.",
            AiErrorKind::UnsafeCredentials => {
                "Credential storage is unsafe. Check its ownership and permissions."
            }
            AiErrorKind::ToolRejected => "The requested tool operation was rejected.",
            AiErrorKind::IndexStale => "The note index needs refreshing.",
            AiErrorKind::Storage => "Local storage operation failed.",
            AiErrorKind::Other => "The operation did not complete.",
        };
        f.write_str(text)?;
        if self.kind == AiErrorKind::RateLimited
            && let Some(delay) = self.retry_after_seconds.filter(|d| *d <= MAX_DELAY_SECONDS)
        {
            write!(f, " Retry after {delay} seconds.")?;
        }
        Ok(())
    }
}

impl std::error::Error for AiError {}

fn map_http_error(status: u16, body: &str) -> AiError {
    let parsed = if body.len() <= 1024 * 1024 {
        serde_json::from_str::<serde_json::Value>(body).ok()
    } else {
        None
    };
    let envelope = parsed.as_ref().and_then(|v| v.get("error"));
    let code = envelope.and_then(|v| {
        v.get("code")
            .and_then(|v| v.as_str())
            .or_else(|| v.as_str())
    });
    let kind = match status {
        401 | 403 => AiErrorKind::ReconnectNeeded,
        429 => AiErrorKind::RateLimited,
        _ => match code {
            Some(
                "model_not_supported" | "unsupported_model" | "model_not_found" | "model_refused",
            ) => AiErrorKind::ModelRefused,
            Some("expired_token" | "expired_device_code") => AiErrorKind::CodeExpired,
            Some("invalid_grant") => AiErrorKind::ReconnectNeeded,
            _ => AiErrorKind::Other,
        },
    };
    let retry_after_seconds = if kind == AiErrorKind::RateLimited {
        envelope
            .and_then(|v| v.get("resets_in_seconds"))
            .and_then(|v| v.as_u64())
            .filter(|v| *v <= MAX_DELAY_SECONDS)
    } else {
        None
    };
    AiError {
        kind,
        retry_after_seconds,
    }
}

pub(crate) fn map_transport(error: http_client::Error) -> AiError {
    match error {
        http_client::Error::InvalidStatusCodeWithDetails { status, body, .. } => {
            map_http_error(status.as_u16(), &body)
        }
        http_client::Error::Instance(_) | http_client::Error::StreamEnded => {
            AiError::new(AiErrorKind::Network)
        }
        _ => AiError::new(AiErrorKind::Other),
    }
}

pub(crate) fn map_auth(error: AuthError) -> AiError {
    match error {
        AuthError::Http(error) => map_transport(error),
        AuthError::Io(_) => AiError::new(AiErrorKind::Storage),
        AuthError::Json(_) => AiError::new(AiErrorKind::ReconnectNeeded),
        AuthError::Message(message) => {
            // Rig 0.43 loses typed status on ChatGPT refresh/device failures.
            // Inspect only its fixed prefix/status; never retain the diagnostic.
            for prefix in [
                "ChatGPT token refresh failed: ",
                "ChatGPT device authorization failed: ",
            ] {
                if let Some(rest) = message.strip_prefix(prefix)
                    && let Some(status) = rest
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse::<u16>().ok())
                {
                    let body = rest.find('{').map(|i| &rest[i..]).unwrap_or("");
                    return map_http_error(status, body);
                }
            }
            let kind = match message.as_str() {
                "Timed out waiting for ChatGPT device authorization"
                | "Timed out waiting for GitHub Copilot device authorization"
                | "GitHub device authorization expired before it completed" => {
                    AiErrorKind::CodeExpired
                }
                "ChatGPT sign-in required. Reconnect ChatGPT in Settings before using this provider."
                | "GitHub Copilot sign-in required. Reconnect Copilot in Settings before using this provider."
                | "GitHub device authorization was denied" => AiErrorKind::ReconnectNeeded,
                _ => AiErrorKind::Other,
            };
            AiError::new(kind)
        }
    }
}

pub(crate) fn map_provider(error: rig::error::ProviderError) -> AiError {
    if let Some(response) = error.provider_response() {
        return map_http_error(
            response.status.map(|s| s.as_u16()).unwrap_or(0),
            &response.body,
        );
    }
    match error {
        rig::error::ProviderError::Http(error) => {
            if let Some(status) = error.non_success_status() {
                map_http_error(status.as_u16(), error.non_success_body().unwrap_or(""))
            } else if matches!(
                error.as_ref(),
                http_client::Error::Instance(_) | http_client::Error::StreamEnded
            ) {
                AiError::new(AiErrorKind::Network)
            } else {
                AiError::new(AiErrorKind::Other)
            }
        }
        _ => AiError::new(AiErrorKind::Other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_errors_never_retain_raw_payloads() {
        let body = r#"{"error":{"code":"usage_limit_reached",
            "resets_in_seconds":42},"access_token":"SYNTHETIC_SECRET"}"#;
        let error = map_http_error(429, body);
        assert_eq!(error.kind, AiErrorKind::RateLimited);
        assert_eq!(error.retry_after_seconds, Some(42));
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains("SYNTHETIC_SECRET"));
        assert!(!rendered.contains("access_token"));
    }

    #[test]
    fn statuses_codes_and_bounded_delays_are_allowlisted() {
        for (status, body, kind, delay) in [
            (401, "SYNTHETIC_SECRET", AiErrorKind::ReconnectNeeded, None),
            (403, "{}", AiErrorKind::ReconnectNeeded, None),
            (
                429,
                r#"{"error":{"resets_in_seconds":-1}}"#,
                AiErrorKind::RateLimited,
                None,
            ),
            (
                429,
                r#"{"error":{"resets_in_seconds":18446744073709551615}}"#,
                AiErrorKind::RateLimited,
                None,
            ),
            (
                429,
                r#"{"error":{"resets_in_seconds":"42"}}"#,
                AiErrorKind::RateLimited,
                None,
            ),
            (
                400,
                r#"{"error":{"code":"model_not_supported"}}"#,
                AiErrorKind::ModelRefused,
                None,
            ),
            (
                400,
                r#"{"error":{"code":"unsupported_model"}}"#,
                AiErrorKind::ModelRefused,
                None,
            ),
            (
                400,
                r#"{"error":{"code":"SYNTHETIC_SECRET"}}"#,
                AiErrorKind::Other,
                None,
            ),
            (500, "not json SYNTHETIC_SECRET", AiErrorKind::Other, None),
        ] {
            let error = map_http_error(status, body);
            assert_eq!((error.kind, error.retry_after_seconds), (kind, delay));
            assert!(!format!("{error:?} {error}").contains("SYNTHETIC_SECRET"));
        }
    }

    #[test]
    fn typed_transport_auth_storage_and_provider_errors_are_safe_and_distinct() {
        let network = map_transport(http_client::Error::Instance(Box::new(
            std::io::Error::other("SYNTHETIC_SECRET"),
        )));
        let storage = map_auth(AuthError::Io(std::io::Error::other("SYNTHETIC_SECRET")));
        let expired = map_auth(AuthError::Message(
            "Timed out waiting for ChatGPT device authorization".into(),
        ));
        let unknown = map_auth(AuthError::Message("SYNTHETIC_SECRET".into()));
        for (error, kind) in [
            (network, AiErrorKind::Network),
            (storage, AiErrorKind::Storage),
            (expired, AiErrorKind::CodeExpired),
            (unknown, AiErrorKind::Other),
        ] {
            assert_eq!(error.kind, kind);
            assert!(std::error::Error::source(&error).is_none());
            assert!(!format!("{error:?} {error}").contains("SYNTHETIC_SECRET"));
            assert!(
                !serde_json::to_string(&error)
                    .unwrap()
                    .contains("SYNTHETIC_SECRET")
            );
        }
        for status in [400, 401, 403, 429, 500] {
            let body = r#"{"error":{"code":"unsupported_model","resets_in_seconds":42},"access_token":"SYNTHETIC_SECRET"}"#;
            let error = map_provider(rig::error::ProviderError::from_http_response(
                http_client::StatusCode::from_u16(status).unwrap(),
                body,
            ));
            let expected = map_http_error(status, body);
            assert_eq!(error, expected);
            assert!(!format!("{error:?} {error}").contains("SYNTHETIC_SECRET"));
        }
    }

    #[test]
    fn pinned_chatgpt_string_wrappers_are_mapped_without_retaining_diagnostics() {
        for (message, kind, delay) in [
            (
                r#"ChatGPT token refresh failed: 429 Too Many Requests {"error":{"resets_in_seconds":42},"access_token":"SYNTHETIC_SECRET"}"#,
                AiErrorKind::RateLimited,
                Some(42),
            ),
            (
                "ChatGPT token refresh failed: 403 Forbidden SYNTHETIC_SECRET",
                AiErrorKind::ReconnectNeeded,
                None,
            ),
            (
                r#"ChatGPT device authorization failed: 400 Bad Request {"error":"expired_token","device_code":"SYNTHETIC_SECRET"}"#,
                AiErrorKind::CodeExpired,
                None,
            ),
        ] {
            let error = map_auth(AuthError::Message(message.into()));
            assert_eq!((error.kind, error.retry_after_seconds), (kind, delay));
            assert!(!format!("{error:?} {error}").contains("SYNTHETIC_SECRET"));
        }
    }
}
