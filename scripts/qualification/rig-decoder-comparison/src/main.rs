use futures::StreamExt;
use serde_json::{Value, json};

fn event(value: Value) -> String {
    format!("data: {value}\n\n")
}
fn message(text: &str) -> Value {
    json!({"type":"message","id":"msg_synthetic","status":"completed","role":"assistant",
        "content":[{"type":"output_text","annotations":[],"text":text}]})
}
fn terminal(kind: &str, status: &str, output: Value) -> String {
    event(json!({"type":kind,"sequence_number":100,"response":{
        "id":"resp_synthetic","object":"response","created_at":1,"status":status,
        "error":null,"incomplete_details":if status == "incomplete" {json!({"reason":"max_output_tokens"})} else {Value::Null},
        "instructions":null,"max_output_tokens":null,"model":"synthetic",
        "usage":{"input_tokens":1,"input_tokens_details":{"cached_tokens":0},
            "output_tokens":1,"output_tokens_details":{"reasoning_tokens":0},"total_tokens":2},
        "output":output,"tools":[]}}))
        + "data: [DONE]\n\n"
}
fn finish(text: &str) -> String {
    terminal("response.completed", "completed", json!([message(text)]))
}
fn delta(value: Value, seq: u64) -> String {
    event(
        json!({"type":"response.output_text.delta","delta":value,"item_id":"msg_synthetic",
        "output_index":0,"content_index":0,"sequence_number":seq}),
    )
}
fn done(item: Value) -> String {
    event(
        json!({"type":"response.output_item.done","output_index":0,"sequence_number":90,"item":item}),
    )
}

#[derive(Debug, PartialEq)]
struct Observed {
    terminal: String,
    preview: String,
    final_text: String,
    finish_reason: String,
    tool_calls: usize,
    marked_invalid_calls: usize,
}
macro_rules! compare_version {
    ($module:ident, $rig:ident, $version:literal) => {
        mod $module {
            use super::*;
            pub async fn run(case: &str, sse: String) -> Observed {
                let http = $rig::test_utils::SequencedHttpClient::new([
                    $rig::test_utils::MockHttpResponse::success_typed(sse, "text/event-stream")
                ]);
                let client = $rig::providers::openai::OpenAIConfig::with_key(
                    &$rig::providers::chatgpt::DIALECT, "synthetic-not-a-credential"
                ).connect(http.clone());
                let model = client.responses("gpt-6.1-sol");
                let mut stream = model.stream("Synthetic decoder comparison only").unwrap();
                let mut preview = String::new();
                while let Some(item) = stream.next().await {
                    if let Ok($rig::streaming::Item::Event($rig::streaming::StreamEvent::Text { text, .. })) = item {
                        preview.push_str(&text);
                    }
                }
                // The stream is provisional. Never reconstruct successful output from it.
                let result = stream.finish().await;
                let observed = match result {
                    Ok(reply) => Observed {
                        terminal: "success".into(),
                        preview,
                        final_text: reply.choice.iter().filter_map(|part| match part {
                            $rig::message::AssistantContent::Text(text) => Some(text.text.as_str()), _ => None
                        }).collect(),
                        finish_reason: format!("{:?}", reply.finish_reason()),
                        tool_calls: reply.tool_calls().count(),
                        marked_invalid_calls: reply.tool_calls().filter(|call| {
                            serde_json::to_value(&call.function).unwrap().get("invalid_arguments").is_some_and(|value| !value.is_null())
                        }).count(),
                    },
                    Err(error) => Observed {
                        terminal: match error {
                            $rig::error::ProviderError::Truncated => "truncated",
                            $rig::error::ProviderError::Json(_) => "json_error",
                            _ => "error",
                        }.into(),
                        preview,
                        final_text: String::new(),
                        finish_reason: "None".into(),
                        tool_calls: 0,
                        marked_invalid_calls: 0,
                    },
                };
                println!("{}", json!({"rig":$version,"fixture":case,"terminal":observed.terminal,
                    "preview":observed.preview,"final_text":observed.final_text,
                    "finish_reason":observed.finish_reason,"decoded_tool_calls":observed.tool_calls,
                    "marked_invalid_calls":observed.marked_invalid_calls,
                    "http_requests":http.requests().len(),"tool_dispatches":0}));
                assert_eq!(http.requests().len(), 1, "one request; no retry");
                assert_eq!(http.remaining_responses(), 0);
                observed
            }
        }
    };
}
compare_version!(old, rig43, "0.43.0");
compare_version!(new, rig44, "0.44.0");

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let invalid_call = json!({"type":"function_call","id":"fc_synthetic","call_id":"call_synthetic",
        "name":"synthetic_write","arguments":"{invalid synthetic JSON}","status":"completed"});
    for (name, sse) in [
        (
            "valid_control",
            delta(json!("Synthetic valid answer"), 1) + &finish("Synthetic valid answer"),
        ),
        (
            "recognized_text_delta_wrong_type",
            delta(json!(42), 1)
                + &delta(json!("Synthetic later text"), 2)
                + &finish("Synthetic later text"),
        ),
        (
            "invalid_json",
            "data: {invalid synthetic JSON}\n\n".to_owned()
                + &delta(json!("Synthetic later text"), 2)
                + &finish("Synthetic later text"),
        ),
        (
            "invalid_json_after_preview",
            delta(json!("Synthetic partial"), 1)
                + "data: {invalid synthetic JSON}\n\n"
                + &finish("Synthetic later text"),
        ),
        (
            "harmless_provider_metadata",
            event(json!({"type":"synthetic.gateway.metadata","opaque":42}))
                + &event(
                    json!({"type":"response.output_text.delta","delta":"Synthetic valid answer","item_id":"msg_synthetic",
                "output_index":0,"content_index":0,"sequence_number":1,"obfuscation":"synthetic-padding"}),
                )
                + &finish("Synthetic valid answer"),
        ),
        (
            "stream_A_completed_B",
            delta(json!("Synthetic A"), 1) + &finish("Synthetic B"),
        ),
        (
            "stream_A_item_done_B",
            delta(json!("Synthetic A"), 1) + &done(message("Synthetic B")) + &finish("Synthetic B"),
        ),
        (
            "numeric_delta_without_final",
            delta(json!(42), 1) + &delta(json!("Synthetic partial"), 2),
        ),
        ("truncated_stream", delta(json!("Synthetic partial"), 1)),
        (
            "sentinel_without_final",
            delta(json!("Synthetic partial"), 1) + "data: [DONE]\n\n",
        ),
        (
            "provider_incomplete",
            delta(json!("Synthetic partial"), 1)
                + &terminal(
                    "response.incomplete",
                    "incomplete",
                    json!([message("Synthetic partial")]),
                ),
        ),
        (
            "malformed_tool_arguments",
            event(
                json!({"type":"response.output_item.added","output_index":0,"sequence_number":1,
                "item":{"type":"function_call","id":"fc_synthetic","call_id":"call_synthetic",
                    "name":"synthetic_write","arguments":"","status":"in_progress"}}),
            ) + &event(
                json!({"type":"response.function_call_arguments.delta","output_index":0,"sequence_number":2,
                "item_id":"fc_synthetic","delta":"{invalid synthetic JSON}"}),
            ) + &done(invalid_call.clone())
                + &terminal("response.completed", "completed", json!([invalid_call])),
        ),
    ] {
        let old = old::run(name, sse.clone()).await;
        let new = new::run(name, sse).await;
        match name {
            "valid_control" | "harmless_provider_metadata" => {
                assert_eq!(old.terminal, "success");
                assert_eq!(old.final_text, "Synthetic valid answer");
                assert_eq!(new, old);
            }
            "recognized_text_delta_wrong_type" => {
                assert_ne!(old.terminal, "success");
                assert_eq!(new.terminal, "success");
                assert_eq!(new.final_text, "Synthetic later text");
            }
            "invalid_json" | "invalid_json_after_preview" => {
                assert_ne!(old.terminal, "success");
                assert_ne!(new.terminal, "success");
                assert_eq!(old.final_text, "");
                assert_eq!(new.final_text, "");
                if name == "invalid_json_after_preview" {
                    assert_eq!(old.preview, "Synthetic partial");
                    assert_eq!(new.preview, "Synthetic partial");
                }
            }
            "stream_A_completed_B" | "stream_A_item_done_B" => {
                assert_eq!(old.terminal, "success");
                assert_eq!(old.preview, "Synthetic A");
                assert_eq!(old.final_text, "Synthetic A");
                assert_eq!(new.terminal, "success");
                assert_eq!(new.preview, "Synthetic A");
                assert_eq!(new.final_text, "Synthetic B");
            }
            "numeric_delta_without_final" | "truncated_stream" | "sentinel_without_final" => {
                assert_ne!(old.terminal, "success");
                assert_eq!(new.terminal, "truncated");
                assert_eq!(new.final_text, "");
            }
            "provider_incomplete" => {
                assert_eq!(new.finish_reason, "Some(Length)");
                assert_eq!(old.finish_reason, "Some(Length)");
            }
            "malformed_tool_arguments" => {
                assert_eq!(new.terminal, "success");
                assert_eq!(new.tool_calls, 1);
                assert_eq!(new.marked_invalid_calls, 1);
                assert_eq!(old.tool_calls, 0);
                assert_eq!(old.final_text, "");
            }
            _ => unreachable!(),
        }
    }
}
