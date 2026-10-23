use crate::{AiError, AiErrorKind, AiFailureDetail};
use rig::http_client;
use rig::providers::chatgpt::auth::AuthError;

const MAX_DELAY_SECONDS: u64 = 365 * 24 * 60 * 60;

impl AiError {
    /// Preserve the existing terminal classification for estimated admission
    /// and provider exhaustion; their notices distinguish the actual cause.
    pub(crate) fn is_context_limit(&self) -> bool {
        self.context_limit_notice().is_some()
    }
    /// Local estimated admission and actual provider exhaustion share the
    /// existing terminal category, while keeping their observable cause honest.
    pub(crate) fn context_limit_notice(&self) -> Option<&'static str> {
        if self.kind != AiErrorKind::Other {
            return None;
        }
        let detail = self.detail.as_ref()?;
        match detail.code.as_deref()? {
            "context_length_exceeded" | "model_max_prompt_tokens_exceeded" => {
                Some("The context limit was reached.")
            }
            "brn_request_size_estimate_exceeded_unknown" if detail.origin == "context_budget" => {
                Some(
                    "The BRN request-size estimate cap was reached; the model context limit is unknown.",
                )
            }
            "brn_request_size_estimate_exceeded" if detail.origin == "context_budget" => Some(
                "The BRN request-size estimate reached the selected-route admission budget; model token fit is not proven.",
            ),
            "brn_tool_result_admission_exceeded" if detail.origin == "context_budget" => {
                Some("The BRN per-tool-result encoded admission estimate cap was reached.")
            }
            _ => None,
        }
    }
    pub(crate) fn skill_unavailable_notice(&self) -> Option<&'static str> {
        (self.kind == AiErrorKind::Other && self.detail.as_ref().is_some_and(|d| d.origin == "skill_catalogue" && d.code.as_deref() == Some("skill_unavailable")))
            .then_some("The selected skill is unavailable. Reinstall its exact approved package or change the scoped selection before continuing.")
    }
    pub(crate) fn mcp_unavailable_notice(&self) -> Option<&'static str> {
        (self.kind == AiErrorKind::Other && self.detail.as_ref().is_some_and(|d|
            d.origin == "mcp_client" && d.code.as_deref() == Some("mcp_unavailable")))
            .then_some("The approved MCP connection or its external-data grant is unavailable. Review the local connection and authorize the exact input again before continuing.")
    }
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

/// Provider codes and types published by the supported providers, kept as is.
const KNOWN_CODES: &[&str] = &[
    "model_not_supported",
    "unsupported_model",
    "model_not_found",
    "model_refused",
    "unsupported_api_for_model",
    "expired_token",
    "expired_device_code",
    "invalid_grant",
    "insufficient_quota",
    "overloaded",
    "content_filter",
    "max_output_tokens",
];
/// Reasons a Responses reply gives for an `incomplete` status.
const INCOMPLETE_REASONS: &[&str] = &["max_output_tokens", "content_filter"];
/// Stands for a provider code or type that is present but not allow-listed.
pub(crate) const UNRECOGNIZED: &str = "unrecognized";

/// A provider code or type reduced to an allow-list: a known code, an
/// `incomplete:<reason>` with a known reason, or a lowercase `[a-z_]` name of
/// at most 48 bytes that ends in `_error` or `_exceeded` or starts with
/// `server_`, `rate_limit` or `usage_limit`. Anything else, including slugs,
/// hostnames, `host:port`, one-word refusals and free text, becomes
/// [`UNRECOGNIZED`]; nothing but these shapes ever leaves a provider reply.
pub(crate) fn provider_code(value: &str) -> String {
    let value = value.trim();
    if let Some(reason) = value.strip_prefix("incomplete:") {
        let reason = reason.trim();
        if INCOMPLETE_REASONS.contains(&reason) {
            return format!("incomplete:{reason}");
        }
        return UNRECOGNIZED.to_owned();
    }
    let shaped = !value.is_empty()
        && value.len() <= 48
        && value.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        && !value.starts_with('_');
    let patterned = value.ends_with("_error")
        || value.ends_with("_exceeded")
        || value.starts_with("server_")
        || value.starts_with("rate_limit")
        || value.starts_with("usage_limit");
    if shaped && (KNOWN_CODES.contains(&value) || patterned) {
        value.to_owned()
    } else {
        UNRECOGNIZED.to_owned()
    }
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(notice) = self.context_limit_notice() {
            return write!(
                f,
                "{notice} Continue starts a fresh invocation with retained progress."
            );
        }
        if let Some(notice) = self.skill_unavailable_notice() {
            return f.write_str(notice);
        }
        if let Some(notice) = self.mcp_unavailable_notice() {
            return f.write_str(notice);
        }
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
    pointers
        .iter()
        .find_map(|p| value.pointer(p).and_then(|v| v.as_str()).map(provider_code))
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
        _ => "other",
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
        AuthError::Store(error) => match error.downcast::<AiError>() {
            Ok(error) => *error,
            Err(_) => AiError::new(AiErrorKind::Storage),
        },
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
/// leading code through the allow-list.
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
    let code = Some(match rest.strip_prefix("incomplete: ") {
        Some(reason) => provider_code(&format!(
            "incomplete:{}",
            reason.split(": ").next().unwrap_or_default()
        )),
        None => provider_code(rest.split(": ").next().unwrap_or_default()),
    });
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
                code: report.code.as_deref().map(provider_code),
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
                (
                    "http_status",
                    Some(400),
                    Some("unrecognized"),
                    Some("unrecognized"),
                    false,
                ),
            ),
            (
                400,
                r#"{"error":{"code":"Bearer SYNTHETIC_SECRET","type":"9f8e7d6c5b4a39281706"}}"#,
                AiErrorKind::Other,
                (
                    "http_status",
                    Some(400),
                    Some("unrecognized"),
                    Some("unrecognized"),
                    false,
                ),
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
    fn provider_codes_pass_only_the_allow_list() {
        for kept in [
            "server_error",
            "server_is_overloaded",
            "invalid_request_error",
            "context_length_exceeded",
            "rate_limit_exceeded",
            "usage_limit_reached",
            "insufficient_quota",
            "unsupported_api_for_model",
            "incomplete:max_output_tokens",
        ] {
            assert_eq!(provider_code(kept), kept);
        }
        for dropped in [
            "",
            "SYNTHETIC_SECRET",
            "Server_error",
            "_server_error",
            "refused",
            "denied",
            "evokessler",
            "my-project-slug",
            "api.openai.com",
            "chatgpt.com:443",
            "localhost:8080",
            "server_error2",
            "acct_1234567890abcdef",
            "550e8400-e29b-41d4-a716-446655440000",
            "two words_error",
            "incomplete:SYNTHETIC_SECRET",
            "incomplete:refused",
            &format!("{}_error", "a".repeat(48)),
        ] {
            assert_eq!(provider_code(dropped), UNRECOGNIZED, "{dropped}");
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
                ("turn_failed", None, Some("incomplete:max_output_tokens"), None, false),
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

#[cfg(test)]
mod skill_notice_tests {
    use super::*;
    #[test]
    fn only_exact_local_unavailable_tuple_has_package_remediation() {
        let notice = "The selected skill is unavailable. Reinstall its exact approved package or change the scoped selection before continuing.";
        let local = |kind, origin: &str, code: Option<&str>| {
            AiError::new(kind).with_detail(AiFailureDetail {
                origin: origin.into(),
                code: code.map(str::to_owned),
                ..Default::default()
            })
        };
        assert_eq!(
            local(
                AiErrorKind::Other,
                "skill_catalogue",
                Some("skill_unavailable")
            )
            .to_string(),
            notice
        );
        for error in [
            local(
                AiErrorKind::ToolRejected,
                "skill_catalogue",
                Some("skill_unavailable"),
            ),
            local(AiErrorKind::Other, "provider", Some("skill_unavailable")),
            local(AiErrorKind::Other, "", Some("skill_unavailable")),
            local(AiErrorKind::Other, "skill_catalogue", None),
            local(AiErrorKind::Other, "skill_catalogue", Some("other_error")),
            AiError::new(AiErrorKind::Other),
            map_http_error(
                400,
                r#"{"origin":"skill_catalogue","error":{"code":"skill_unavailable","message":"skill_catalogue skill_unavailable"}}"#,
            ),
        ] {
            assert!(!error.to_string().contains("Reinstall"));
            assert!(!error.is_context_limit());
        }
    }
}

#[cfg(test)]
mod mcp_diagnostic_tests {
    use super::*;
    #[test]
    fn local_mcp_notice_cannot_be_spoofed_by_provider_detail() {
        let local = |kind, origin: &str, code: Option<&str>| {
            AiError::new(kind).with_detail(AiFailureDetail {
                origin: origin.into(),
                code: code.map(str::to_owned),
                ..Default::default()
            })
        };
        let notice = "The approved MCP connection or its external-data grant is unavailable. Review the local connection and authorize the exact input again before continuing.";
        assert_eq!(
            local(AiErrorKind::Other, "mcp_client", Some("mcp_unavailable")).to_string(),
            notice
        );
        for error in [
            local(
                AiErrorKind::ToolRejected,
                "mcp_client",
                Some("mcp_unavailable"),
            ),
            local(AiErrorKind::Other, "provider", Some("mcp_unavailable")),
            local(AiErrorKind::Other, "", Some("mcp_unavailable")),
            local(AiErrorKind::Other, "mcp_client", None),
            local(AiErrorKind::Other, "mcp_client", Some("other_error")),
            AiError::new(AiErrorKind::Other),
            map_http_error(
                400,
                r#"{"origin":"mcp_client","error":{"code":"mcp_unavailable","message":"mcp_client mcp_unavailable"}}"#,
            ),
        ] {
            assert_ne!(error.to_string(), notice);
            assert!(!error.to_string().contains("authorize the exact input"));
            assert!(!error.is_context_limit());
        }
    }
}
