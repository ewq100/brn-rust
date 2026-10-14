use crate::{AiError, AiErrorKind, AiFailureDetail};
use rig::http_client;
use rig::providers::chatgpt::auth::AuthError;

const MAX_DELAY_SECONDS: u64 = 365 * 24 * 60 * 60;

impl AiError {
    pub fn new(kind: AiErrorKind) -> Self {
        Self {
            kind,
            retry_after_seconds: None,
            detail: None,
        }
    }

    pub(crate) fn with_detail(mut self, detail: AiFailureDetail) -> Self {
        self.detail = Some(Box::new(detail));
        self
    }
}

fn detail(origin: &str) -> AiFailureDetail {
    AiFailureDetail {
        origin: origin.to_owned(),
        ..Default::default()
    }
}

/// A provider code or type kept only when it has the shape of a published
/// snake_case identifier: lowercase, starting with a letter, at most 48 bytes
/// and three digits. Free text, mixed-case or hex tokens, identifiers with
/// hyphens and long values never survive.
pub(crate) fn safe_token(value: &str) -> Option<String> {
    let value = value.trim();
    (value.len() <= 48
        && value.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && value.bytes().filter(u8::is_ascii_digit).count() <= 3
        && value.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b':')
        }))
    .then(|| value.to_owned())
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self.kind {
            AiErrorKind::QuoteNotFound => "The quotation was not found in the saved note body.",
            AiErrorKind::QuoteAmbiguous => {
                "The quotation occurs more than once in the saved note body. Specify its occurrence."
            }
            AiErrorKind::QuoteOccurrenceInvalid => {
                "The quotation occurrence is outside the saved note body."
            }
            AiErrorKind::ReconnectNeeded => "Reconnect this account before continuing.",
            AiErrorKind::CodeExpired => "Login code expired. Retry Connect to obtain a new code.",
            AiErrorKind::RateLimited => "Account usage limit reached.",
            AiErrorKind::Network => "Network request failed.",
            AiErrorKind::ModelRefused => {
                "The selected model is not supported by this provider route."
            }
            AiErrorKind::InvalidToolUse => "The model requested an invalid tool.",
            AiErrorKind::ToolLimitReached => "The tool round limit was reached.",
            AiErrorKind::TimeLimitReached => "The work time limit was reached.",
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

fn pointer_token(value: Option<&serde_json::Value>, pointers: &[&str]) -> Option<String> {
    let value = value?;
    pointers.iter().find_map(|p| {
        value
            .pointer(p)
            .and_then(|v| v.as_str())
            .and_then(safe_token)
    })
}

/// Maps a provider reply (`status` 0 when it had none, as a stream event
/// does) to a safe error. Only allow-listed identifiers leave the body.
fn map_http_error(status: u16, body: &str) -> AiError {
    let parsed = if body.len() <= 1024 * 1024 {
        serde_json::from_str::<serde_json::Value>(body).ok()
    } else {
        None
    };
    let envelope = parsed.as_ref().and_then(|v| v.get("error"));
    let code = pointer_token(
        parsed.as_ref(),
        &["/error/code", "/response/error/code", "/code", "/error"],
    );
    let error_type = pointer_token(parsed.as_ref(), &["/error/type", "/response/error/type"]);
    let event = parsed
        .as_ref()
        .and_then(|v| v.get("type"))
        .and_then(|v| v.as_str());
    let kind = match status {
        401 | 403 => AiErrorKind::ReconnectNeeded,
        429 => AiErrorKind::RateLimited,
        _ => match code.as_deref() {
            Some(
                "model_not_supported"
                | "unsupported_model"
                | "model_not_found"
                | "model_refused"
                | "unsupported_api_for_model",
            ) => AiErrorKind::ModelRefused,
            Some("expired_token" | "expired_device_code") => AiErrorKind::CodeExpired,
            Some("invalid_grant") => AiErrorKind::ReconnectNeeded,
            Some("usage_limit_reached" | "rate_limit_exceeded" | "insufficient_quota") => {
                AiErrorKind::RateLimited
            }
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
    let origin = match (status, event) {
        (0, Some("response.failed")) => "response_failed",
        (0, Some("error")) => "stream_error",
        (0, _) => "provider_reply",
        _ => "http_status",
    };
    let retryable = match status {
        0 => matches!(
            code.as_deref(),
            Some("server_error" | "internal_error" | "server_is_overloaded" | "overloaded")
        ),
        408 | 500..=599 => true,
        _ => false,
    };
    AiError {
        kind,
        retry_after_seconds,
        detail: Some(Box::new(AiFailureDetail {
            status: (status != 0).then_some(status),
            code,
            error_type,
            retryable,
            ..detail(origin)
        })),
    }
}

fn transport_variant(error: &http_client::Error) -> &'static str {
    match error {
        http_client::Error::Protocol(_) => "protocol",
        http_client::Error::InvalidStatusCodeWithDetails { .. } => "status",
        http_client::Error::InvalidHeaderValue(_) => "invalid_header_value",
        http_client::Error::NoHeaders => "no_headers",
        http_client::Error::StreamEnded => "stream_ended",
        http_client::Error::InvalidContentType(_) => "invalid_content_type",
        http_client::Error::Instance(_) => "instance",
    }
}

pub(crate) fn map_transport(error: http_client::Error) -> AiError {
    match error {
        http_client::Error::InvalidStatusCodeWithDetails { status, body, .. } => {
            map_http_error(status.as_u16(), &body)
        }
        error => map_transport_ref(&error),
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
            AiError::new(kind).with_detail(detail("auth"))
        }
    }
}

/// Rig's fixed turn-failure wording, reduced to a label and the provider's
/// leading code when that is a bare identifier.
fn turn_failure(message: &str) -> AiError {
    let rest = [
        "the turn failed, so none of its tool calls ran: ",
        "the provider failed the turn without an answer: ",
        "the provider failed the turn: ",
    ]
    .iter()
    .find_map(|prefix| message.strip_prefix(prefix));
    let Some(rest) = rest else {
        return AiError::new(AiErrorKind::Other).with_detail(detail("response"));
    };
    let rest = rest
        .strip_prefix("Provider finish_reason: ")
        .unwrap_or(rest);
    let code = rest
        .split(": ")
        .next()
        .and_then(safe_token)
        .or_else(|| rest.strip_prefix("incomplete: ").and_then(safe_token));
    let retryable = matches!(
        code.as_deref(),
        Some("server_error" | "internal_error" | "server_is_overloaded" | "overloaded")
    );
    AiError::new(AiErrorKind::Other).with_detail(AiFailureDetail {
        code,
        retryable,
        ..detail("turn_failed")
    })
}

pub(crate) fn map_provider(error: rig::error::ProviderError) -> AiError {
    use rig::error::ProviderError as E;
    if let Some(response) = error.provider_response() {
        return map_http_error(
            response.status.map(|s| s.as_u16()).unwrap_or(0),
            &response.body,
        );
    }
    let retryable = error.is_retryable();
    let (kind, mut info) = match &error {
        E::Relayed(report) => {
            let kind = if report.kind == rig::error::ErrorKind::Http {
                AiErrorKind::Network
            } else {
                AiErrorKind::Other
            };
            let info = AiFailureDetail {
                status: report.http_status,
                code: report.code.as_deref().and_then(safe_token),
                ..detail("relayed")
            };
            (kind, info)
        }
        E::Http(error) => {
            if let Some(status) = error.non_success_status() {
                return map_http_error(status.as_u16(), error.non_success_body().unwrap_or(""));
            }
            return map_transport_ref(error.as_ref());
        }
        // A stream can end early from a disconnect or from replies Rig could
        // not decode, so the kind stays unclassified and the origin says which.
        E::Truncated => (AiErrorKind::Other, detail("truncated")),
        E::Response(message) => {
            let mapped = turn_failure(message);
            (mapped.kind, *mapped.detail.unwrap_or_default())
        }
        E::Json(_) => (AiErrorKind::Other, detail("decode")),
        E::Url(_) | E::Request(_) => (AiErrorKind::Other, detail("request")),
        E::UnsupportedOption(_) => (AiErrorKind::Other, detail("unsupported_option")),
        E::Provider(_) => (AiErrorKind::Other, detail("provider")),
        E::MismatchedDimensions { .. } => (AiErrorKind::Other, detail("response")),
        _ => (AiErrorKind::Other, detail("unknown")),
    };
    info.retryable |= retryable;
    AiError::new(kind).with_detail(info)
}

/// The safe diagnostic of a transport failure, leaving the error intact.
pub(crate) fn transport_detail(error: &http_client::Error) -> AiFailureDetail {
    let mapped = match error {
        http_client::Error::InvalidStatusCodeWithDetails { status, body, .. } => {
            map_http_error(status.as_u16(), body)
        }
        error => map_transport_ref(error),
    };
    *mapped.detail.unwrap_or_default()
}

fn map_transport_ref(error: &http_client::Error) -> AiError {
    let kind = if matches!(
        error,
        http_client::Error::Instance(_) | http_client::Error::StreamEnded
    ) {
        AiErrorKind::Network
    } else {
        AiErrorKind::Other
    };
    AiError::new(kind).with_detail(AiFailureDetail {
        code: Some(transport_variant(error).to_owned()),
        retryable: rig::error::transient_transport(error),
        ..detail("transport")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotation_failures_have_distinct_fixed_safe_messages() {
        for (kind, name, message) in [
            (
                AiErrorKind::QuoteNotFound,
                "quote_not_found",
                "The quotation was not found in the saved note body.",
            ),
            (
                AiErrorKind::QuoteAmbiguous,
                "quote_ambiguous",
                "The quotation occurs more than once in the saved note body. Specify its occurrence.",
            ),
            (
                AiErrorKind::QuoteOccurrenceInvalid,
                "quote_occurrence_invalid",
                "The quotation occurrence is outside the saved note body.",
            ),
        ] {
            let error = AiError {
                kind,
                retry_after_seconds: Some(42),
                detail: None,
            };
            assert_eq!(error.to_string(), message);
            assert_eq!(serde_json::to_value(kind).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<AiErrorKind>(serde_json::json!(name)).unwrap(),
                kind
            );
        }
    }

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
                r#"{"error":{"code":"unsupported_api_for_model","message":"SYNTHETIC_SECRET"}}"#,
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

    fn info(error: &AiError) -> (&str, Option<u16>, Option<&str>, Option<&str>, bool) {
        let d = error.detail.as_ref().expect("detail");
        (
            d.origin.as_str(),
            d.status,
            d.code.as_deref(),
            d.error_type.as_deref(),
            d.retryable,
        )
    }

    fn assert_redacted(error: &AiError) {
        let shown = format!(
            "{error:?} {error} {}",
            serde_json::to_string(error).unwrap()
        );
        for secret in ["SYNTHETIC_SECRET", "sk-synthetic", "acct_", "Bearer"] {
            assert!(!shown.contains(secret), "{secret} in {shown}");
        }
    }

    #[test]
    fn provider_replies_keep_only_status_origin_and_allowlisted_identifiers() {
        let cases = [
            (
                503,
                r#"{"error":{"type":"server_error","code":"server_is_overloaded","message":"SYNTHETIC_SECRET"}}"#,
                AiErrorKind::Other,
                (
                    "http_status",
                    Some(503),
                    Some("server_is_overloaded"),
                    Some("server_error"),
                    true,
                ),
            ),
            (
                0,
                r#"{"type":"error","code":"server_error","message":"SYNTHETIC_SECRET prompt"}"#,
                AiErrorKind::Other,
                ("stream_error", None, Some("server_error"), None, true),
            ),
            (
                0,
                r#"{"type":"response.failed","response":{"id":"resp_SYNTHETIC_SECRET","error":{"code":"context_length_exceeded","type":"invalid_request_error","message":"SYNTHETIC_SECRET"}}}"#,
                AiErrorKind::Other,
                (
                    "response_failed",
                    None,
                    Some("context_length_exceeded"),
                    Some("invalid_request_error"),
                    false,
                ),
            ),
            (
                400,
                r#"{"error":{"code":"rate_limit_exceeded","resets_in_seconds":7}}"#,
                AiErrorKind::RateLimited,
                (
                    "http_status",
                    Some(400),
                    Some("rate_limit_exceeded"),
                    None,
                    false,
                ),
            ),
            (
                0,
                r#"{"misc":"SYNTHETIC_SECRET"}"#,
                AiErrorKind::Other,
                ("provider_reply", None, None, None, false),
            ),
            (
                400,
                r#"{"error":{"code":"sk-synthetic-ABCDEF0123456789","type":"acct_1234567890abcdef"}}"#,
                AiErrorKind::Other,
                ("http_status", Some(400), None, None, false),
            ),
            (
                400,
                r#"{"error":{"code":"Bearer SYNTHETIC_SECRET","type":"9f8e7d6c5b4a39281706"}}"#,
                AiErrorKind::Other,
                ("http_status", Some(400), None, None, false),
            ),
            (
                408,
                "",
                AiErrorKind::Other,
                ("http_status", Some(408), None, None, true),
            ),
        ];
        for (status, body, kind, expected) in cases {
            let error = map_http_error(status, body);
            assert_eq!(error.kind, kind, "{body}");
            assert_eq!(info(&error), expected, "{body}");
            assert_redacted(&error);
        }
        assert_eq!(
            map_http_error(
                400,
                r#"{"error":{"code":"rate_limit_exceeded","resets_in_seconds":7}}"#
            )
            .retry_after_seconds,
            Some(7)
        );
    }

    #[test]
    fn identifiers_must_look_like_published_codes() {
        for kept in [
            "server_error",
            "response.failed",
            "incomplete:max_output_tokens",
            "http_502",
        ] {
            assert_eq!(safe_token(kept).as_deref(), Some(kept));
        }
        for dropped in [
            "",
            "SYNTHETIC_SECRET",
            "Server_error",
            "_leading",
            "550e8400-e29b-41d4-a716-446655440000",
            "a1b2c3d4e5",
            "two words",
            "quote\"",
            &"a".repeat(49),
        ] {
            assert_eq!(safe_token(dropped), None, "{dropped}");
        }
    }

    #[test]
    fn rig_failures_get_distinct_safe_origins_and_unchanged_messages() {
        use rig::error::ProviderError as E;
        let cases = [
            (
                map_provider(E::Truncated),
                ("truncated", None, None, None, true),
            ),
            (
                map_provider(E::Response(
                    "the turn failed, so none of its tool calls ran: server_error: SYNTHETIC_SECRET"
                        .into(),
                )),
                ("turn_failed", None, Some("server_error"), None, true),
            ),
            (
                map_provider(E::Response(
                    "the provider failed the turn: incomplete: max_output_tokens".into(),
                )),
                ("turn_failed", None, Some("incomplete"), None, false),
            ),
            (
                map_provider(E::Response("SYNTHETIC_SECRET".into())),
                ("response", None, None, None, false),
            ),
            (
                map_provider(E::Json(
                    serde_json::from_str::<serde_json::Value>("SYNTHETIC_SECRET").unwrap_err().into(),
                )),
                ("decode", None, None, None, false),
            ),
            (
                map_provider(E::from_provider_body(
                    r#"{"type":"error","code":"internal_error","message":"SYNTHETIC_SECRET"}"#,
                )),
                ("stream_error", None, Some("internal_error"), None, true),
            ),
            (
                map_transport(http_client::Error::StreamEnded),
                ("transport", None, Some("stream_ended"), None, true),
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(info(&error), expected, "{error:?}");
            assert_eq!(error.to_string(), AiError::new(error.kind).to_string());
            assert_redacted(&error);
        }
    }

    #[test]
    fn detail_is_absent_from_legacy_json_and_round_trips() {
        let plain = serde_json::to_value(AiError::new(AiErrorKind::Other)).unwrap();
        assert!(plain.get("detail").is_none());
        let error = map_http_error(502, "");
        let back: AiError = serde_json::from_value(serde_json::to_value(&error).unwrap()).unwrap();
        assert_eq!(back, error);
        let legacy: AiError = serde_json::from_str(r#"{"kind":"other"}"#).unwrap();
        assert_eq!(legacy, AiError::new(AiErrorKind::Other));
    }
}
