//! Opt-in, synthetic qualification of the authenticated subscription routes.

use crate::auth::OwnedClient;
use crate::error::map_provider;
use crate::{AiError, AiErrorKind, Provider, ProviderClient, Selection};
use base64::Engine as _;
use futures::StreamExt;
use rig::completion::{
    CompletionRequest, CompletionResponse, FinishReason, Message, ProviderToolDefinition,
    ToolDefinition,
};
use rig::message::{ImageDetail, ImageMediaType, Text, ToolCall, ToolResultContent, UserContent};
use rig::providers::copilot;
use rig::rig_reqwest::reqwest::Url;
use rig::streaming::{Item, StreamEvent};
use rig::{DynModel, operation::Completion};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

const TEXT_BYTES: usize = 64 * 1024;
const CITATIONS: usize = 16;
const URL_BYTES: usize = 2048;
const TITLE_BYTES: usize = 256;
const NOTE_PATH: &str = "probe.md";
const NOTE_CODE: &str = "orchard-827";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeKind {
    Low,
    High,
    Image,
    Web,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeTerminal {
    Completed,
    Interrupted,
    Failed(AiError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeCitation {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeReport {
    pub selection: Selection,
    pub kind: ProbeKind,
    pub terminal: ProbeTerminal,
    pub text: String,
    pub read_tool_calls: usize,
    pub web_search_observed: bool,
    pub citations: Vec<ProbeCitation>,
}

impl ProbeKind {
    fn reads_note(self) -> bool {
        matches!(self, Self::Low | Self::High)
    }
}

/// Uses only the supplied authenticated client and its frozen selection.
/// A completion means the bounded probe finished; inspect its evidence and
/// answer separately before qualifying the provider's capability.
pub async fn run_probe(
    client: ProviderClient,
    kind: ProbeKind,
    cancel: CancellationToken,
) -> ProbeReport {
    let selection = client.selection().clone();
    let responses = selection.provider == Provider::Chatgpt
        || copilot::wire::routes_through_responses(&selection.model);
    let mut report = ProbeReport {
        selection,
        kind,
        terminal: ProbeTerminal::Interrupted,
        text: String::new(),
        read_tool_calls: 0,
        web_search_observed: false,
        citations: Vec::new(),
    };
    if cancel.is_cancelled() {
        return report;
    }
    if kind == ProbeKind::Web && !responses {
        report.terminal = failed(AiErrorKind::ModelRefused);
        return report;
    }
    let model: DynModel<Completion> = match client.inner {
        OwnedClient::Chatgpt(client) => client.completion(report.selection.model.clone()).into(),
        OwnedClient::Copilot(client) => client.completion(report.selection.model.clone()).into(),
    };
    run_model(model, responses, cancel, &mut report).await;
    report
}

async fn run_model(
    model: DynModel<Completion>,
    responses: bool,
    cancel: CancellationToken,
    report: &mut ProbeReport,
) {
    let mut request = probe_request(report.kind, responses);
    for _ in 0..2 {
        let response = match collect(&model, request.clone(), &cancel, report).await {
            Ok(response) => response,
            Err(terminal) => {
                report.terminal = terminal;
                return;
            }
        };
        if !matches!(
            response.finish_reason(),
            None | Some(FinishReason::Stop | FinishReason::ToolCalls)
        ) {
            report.terminal = failed(AiErrorKind::Other);
            return;
        }
        let calls = response.tool_calls().collect::<Vec<_>>();
        if calls.is_empty() {
            report.terminal = if response.finish_reason() == Some(FinishReason::ToolCalls)
                || (report.kind.reads_note() && report.read_tool_calls == 0)
            {
                failed(AiErrorKind::InvalidToolUse)
            } else if report.text.is_empty() {
                failed(AiErrorKind::Other)
            } else {
                ProbeTerminal::Completed
            };
            return;
        }
        if !report.kind.reads_note() {
            report.terminal = failed(AiErrorKind::InvalidToolUse);
            return;
        }
        if calls.len() != 1 || report.read_tool_calls != 0 {
            report.terminal = failed(AiErrorKind::ToolLimitReached);
            return;
        }
        let call = calls[0];
        if valid_read(call).is_err() {
            report.terminal = failed(AiErrorKind::InvalidToolUse);
            return;
        }
        let Some(assistant) = response.message() else {
            report.terminal = failed(AiErrorKind::Other);
            return;
        };
        let result = call.result(vec![ToolResultContent::text(NOTE_CODE)]);
        report.read_tool_calls += 1;
        request.chat_history.push(assistant);
        request
            .chat_history
            .push(Message::tool_results(vec![result]));
    }
    report.terminal = failed(AiErrorKind::ToolLimitReached);
}

fn probe_request(kind: ProbeKind, responses: bool) -> CompletionRequest {
    let mut request = match kind {
        ProbeKind::Low | ProbeKind::High => CompletionRequest::new(
            "Read probe.md exactly once with read_note, then return only the code in that note. \
             Do not guess the code or call another tool.",
        ),
        ProbeKind::Image => CompletionRequest::new(Message::User {
            content: vec![
                UserContent::text(
                    "What color fills the upper half and what color fills the lower half of \
                     this image? Answer with only 'upper: COLOR; lower: COLOR'.",
                ),
                UserContent::image_base64(
                    base64::engine::general_purpose::STANDARD
                        .encode(include_bytes!("fixtures/capability.png")),
                    Some(ImageMediaType::PNG),
                    Some(ImageDetail::High),
                ),
            ],
        }),
        ProbeKind::Web => CompletionRequest::new(
            "Use web search to check the official SQLite website for the latest SQLite release. \
             Give the release version and date, citing the official source URL.",
        ),
    };
    if kind.reads_note() {
        let effort = if kind == ProbeKind::Low {
            "low"
        } else {
            "high"
        };
        request = request
            .additional_params(if responses {
                json!({"reasoning": {"effort": effort}})
            } else {
                json!({"reasoning_effort": effort})
            })
            .tool(ToolDefinition {
                name: "read_note".into(),
                description: "Read the synthetic note probe.md.".into(),
                parameters: json!({
                    "type": "object", "additionalProperties": false,
                    "properties": {"path": {"type": "string", "enum": [NOTE_PATH]}},
                    "required": ["path"]
                }),
            });
    }
    if kind == ProbeKind::Web {
        request = request.provider_tool(ProviderToolDefinition::new("web_search"));
    }
    request
}

fn valid_read(call: &ToolCall) -> Result<(), ()> {
    if call.function.name.as_str() != "read_note" {
        return Err(());
    }
    let args = call.function.arguments.as_object().ok_or(())?;
    if args.len() != 1 || args.get("path").and_then(Value::as_str) != Some(NOTE_PATH) {
        return Err(());
    }
    Ok(())
}

async fn collect(
    model: &DynModel<Completion>,
    request: CompletionRequest,
    cancel: &CancellationToken,
    report: &mut ProbeReport,
) -> Result<CompletionResponse, ProbeTerminal> {
    if cancel.is_cancelled() {
        return Err(ProbeTerminal::Interrupted);
    }
    let mut stream = model
        .stream(request)
        .map_err(|error| ProbeTerminal::Failed(map_provider(error)))?;
    loop {
        let item = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(ProbeTerminal::Interrupted),
            item = stream.next() => item,
        };
        let Some(item) = item else { break };
        match item.map_err(|error| ProbeTerminal::Failed(map_provider(error)))? {
            Item::Event(StreamEvent::Text { text, .. }) => {
                append_text(&mut report.text, &text).map_err(|()| failed(AiErrorKind::Other))?;
            }
            Item::Event(StreamEvent::End {
                content: rig::message::AssistantContent::Text(text),
                ..
            }) => record_citations(report, &text),
            Item::Unknown(payload) if completed_web_search(payload.value()) => {
                report.web_search_observed = true;
            }
            _ => {}
        }
    }
    // The provider's already-consumed ending wins over a later cancellation.
    let response = tokio::select! {
        biased;
        result = stream.finish() => result.map_err(|error| ProbeTerminal::Failed(map_provider(error)))?,
        _ = cancel.cancelled() => return Err(ProbeTerminal::Interrupted),
    };
    for content in &response.choice {
        if let rig::message::AssistantContent::Text(text) = content {
            record_citations(report, text);
        }
    }
    // Some gateways send hosted items only in the final response snapshot.
    // Project the completed-call fact and discard the raw document with the
    // response; do not retain provider metadata in the report.
    if response
        .raw
        .get("output")
        .and_then(Value::as_array)
        .is_some_and(|output| output.iter().take(256).any(completed_web_search))
    {
        report.web_search_observed = true;
    }
    Ok(response)
}

fn failed(kind: AiErrorKind) -> ProbeTerminal {
    ProbeTerminal::Failed(AiError::new(kind))
}

fn append_text(output: &mut String, text: &str) -> Result<(), ()> {
    let room = TEXT_BYTES.saturating_sub(output.len());
    if text.len() <= room {
        output.push_str(text);
        return Ok(());
    }
    let mut end = room;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    output.push_str(&text[..end]);
    Err(())
}

fn completed_web_search(value: &Value) -> bool {
    value.get("type").and_then(Value::as_str) == Some("web_search_call")
        && value.get("status").and_then(Value::as_str) == Some("completed")
}

fn record_citations(report: &mut ProbeReport, text: &Text) {
    let Some(annotations) = text
        .additional_params
        .as_ref()
        .and_then(|params| params.wire_extras("openai_responses"))
        .and_then(|extras| extras.get("annotations"))
        .and_then(Value::as_array)
    else {
        return;
    };
    for annotation in annotations.iter().take(256) {
        if annotation.get("type").and_then(Value::as_str) != Some("url_citation") {
            continue;
        }
        let Some(url) = annotation
            .get("url")
            .and_then(Value::as_str)
            .and_then(citation_url)
        else {
            continue;
        };
        let title = annotation
            .get("title")
            .and_then(Value::as_str)
            .filter(|title| {
                !title.is_empty()
                    && title.len() <= TITLE_BYTES
                    && !title.chars().any(char::is_control)
            })
            .map(str::to_owned);
        if let Some(previous) = report
            .citations
            .iter_mut()
            .find(|citation| citation.url == url)
        {
            if previous.title.is_none() {
                previous.title = title;
            }
        } else if report.citations.len() < CITATIONS {
            report.citations.push(ProbeCitation { url, title });
        }
    }
}

fn citation_url(value: &str) -> Option<String> {
    if value.len() > URL_BYTES
        || value.trim() != value
        || value.chars().any(char::is_control)
        || value.as_bytes().windows(3).any(|part| {
            part[0] == b'%'
                && hex(part[1])
                    .zip(hex(part[2]))
                    .is_some_and(|(high, low)| (high * 16 + low).is_ascii_control())
        })
    {
        return None;
    }
    let url = Url::parse(value).ok()?;
    let authority = value.split_once("://")?.1.split(['/', '?', '#']).next()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || authority.contains('@')
    {
        return None;
    }
    let url = url.to_string();
    (url.len() <= URL_BYTES).then_some(url)
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::message::{AdditionalParams, ToolFunction, ToolName};

    fn report() -> ProbeReport {
        ProbeReport {
            selection: Selection {
                provider: Provider::Chatgpt,
                model: "gpt-5.5".into(),
            },
            kind: ProbeKind::Web,
            terminal: ProbeTerminal::Interrupted,
            text: String::new(),
            read_tool_calls: 0,
            web_search_observed: false,
            citations: Vec::new(),
        }
    }

    fn annotated(values: Vec<Value>) -> Text {
        Text {
            text: "not a second output fragment".into(),
            additional_params: AdditionalParams::from_entries([(
                "openai_responses",
                json!({"annotations": values, "secret": "SYNTHETIC_SECRET"}),
            )]),
        }
    }

    #[test]
    fn read_arguments_require_the_exact_fixture_and_schema() {
        for (name, args, accepted) in [
            ("read_note", json!({"path": "probe.md"}), true),
            ("read_note", json!({"path": "other.md"}), false),
            (
                "read_note",
                json!({"path": "probe.md", "extra": true}),
                false,
            ),
            ("read_note", json!({}), false),
            ("read_note", json!({"path": 1}), false),
            ("write_note", json!({"path": "probe.md"}), false),
        ] {
            let call = ToolCall::from_wire(
                "call",
                ToolFunction {
                    name: ToolName::new(name).unwrap(),
                    arguments: args,
                },
            );
            assert_eq!(valid_read(&call).is_ok(), accepted);
        }
    }

    #[test]
    fn text_cap_preserves_exact_utf8_prefix_without_normalization() {
        let mut text = "\u{feff} exact\r\n".to_owned();
        append_text(&mut text, "λ ").unwrap();
        assert_eq!(text, "\u{feff} exact\r\nλ ");
        text = "a".repeat(TEXT_BYTES - 1);
        assert!(append_text(&mut text, "λtail").is_err());
        assert_eq!(text.len(), TEXT_BYTES - 1);
        assert!(append_text(&mut text, "bc").is_err());
        assert_eq!(text.len(), TEXT_BYTES);
        assert!(text.ends_with('b'));
    }

    #[test]
    fn citations_reject_unsafe_urls_and_bound_titles() {
        for value in [
            "javascript:alert(1)",
            "file:///tmp/probe",
            "https://user@example.com",
            "https://user:password@example.com",
            "https://@example.com",
            "https://example.com/\n",
            "https://example.com/%0a",
            "https://example.com/%7F",
            "https://example.com ",
            "not a url",
            "//example.com",
            "https:///",
            "https://",
        ] {
            assert!(citation_url(value).is_none(), "{value:?}");
        }
        let mut report = report();
        record_citations(
            &mut report,
            &annotated(vec![
                json!({"type":"url_citation", "url":"https://example.com", "title":"bad\ntitle"}),
                json!({"type":"url_citation", "url":"https://example.com/", "title":"Source"}),
                json!({"type":"other", "url":"https://ignored.example/", "title":"Ignored"}),
                json!({"type":"url_citation", "url":"http://other.example/", "title":"x".repeat(TITLE_BYTES + 1)}),
            ]),
        );
        assert_eq!(
            report.citations,
            vec![
                ProbeCitation {
                    url: "https://example.com/".into(),
                    title: Some("Source".into())
                },
                ProbeCitation {
                    url: "http://other.example/".into(),
                    title: None
                },
            ]
        );
        assert!(report.text.is_empty());
        assert!(
            !serde_json::to_string(&report)
                .unwrap()
                .contains("SYNTHETIC_SECRET")
        );
    }

    #[test]
    fn citations_are_deduplicated_and_capped_across_repeated_snapshots() {
        let mut report = report();
        let text = annotated((0..32).map(|i| json!({
            "type":"url_citation", "url": format!("https://example.com/{i}"), "title":"Source"
        })).collect());
        record_citations(&mut report, &text);
        record_citations(&mut report, &text);
        assert_eq!(report.citations.len(), CITATIONS);
        assert_eq!(report.citations[15].url, "https://example.com/15");
        assert!(citation_url(&format!("https://example.com/{}", "x".repeat(URL_BYTES))).is_none());
    }

    #[test]
    fn only_completed_hosted_search_items_establish_web_evidence() {
        assert!(completed_web_search(
            &json!({"type":"web_search_call", "status":"completed"})
        ));
        for value in [
            json!({"type":"web_search_call", "status":"in_progress"}),
            json!({"type":"response.web_search_call.searching", "status":"completed"}),
            json!({"type":"function_call", "status":"completed"}),
            json!({"type":"web_search_call"}),
        ] {
            assert!(!completed_web_search(&value));
        }
    }
}
