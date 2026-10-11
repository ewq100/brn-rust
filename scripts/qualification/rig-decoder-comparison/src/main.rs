use serde_json::{Value, json};
fn event(value: Value) -> String {
    format!("data: {value}\n\n")
}
fn finish(text: &str) -> String {
    event(
        json!({"type":"response.completed","sequence_number":100,"response":{
        "id":"resp_synthetic","object":"response","created_at":1,"status":"completed",
        "error":null,"incomplete_details":null,"instructions":null,"max_output_tokens":null,
        "model":"synthetic","usage":{"input_tokens":1,"input_tokens_details":{"cached_tokens":0},
        "output_tokens":1,"output_tokens_details":{"reasoning_tokens":0},"total_tokens":2},
        "output":[{"type":"message","id":"msg_synthetic","status":"completed","role":"assistant",
        "content":[{"type":"output_text","annotations":[],"text":text}]}],"tools":[]}}),
    ) + "data: [DONE]\n\n"
}
fn delta(value: Value, seq: u64) -> String {
    event(
        json!({"type":"response.output_text.delta","delta":value,"item_id":"msg_synthetic",
        "output_index":0,"content_index":0,"sequence_number":seq}),
    )
}
macro_rules! compare_version {
    ($module:ident, $rig:ident, $version:literal) => { mod $module {
        use super::*;
        pub async fn run(case: &str, sse: String) -> (bool, String) {
            let http = $rig::test_utils::SequencedHttpClient::new([
                $rig::test_utils::MockHttpResponse::success_typed(sse, "text/event-stream")
            ]);
            let client = $rig::providers::openai::OpenAIConfig::with_key(
                &$rig::providers::chatgpt::DIALECT, "synthetic-not-a-credential"
            ).connect(http.clone());
            let model = client.responses("gpt-6.1-sol");
            let result = model.stream("Synthetic decoder comparison only").unwrap().finish().await;
            let observed = match result {
                Ok(reply) => {
                    let text = reply.choice.iter().filter_map(|part| match part {
                        $rig::message::AssistantContent::Text(text) => Some(text.text.as_str()), _ => None
                    }).collect::<String>();
                    println!("{}", json!({"rig":$version,"fixture":case,"terminal":"success", "finish_reason":format!("{:?}",reply.finish_reason()),"text":text,"http_requests":http.requests().len(),"tool_dispatches":0}));
                    (true, text)
                },
                Err(_) => {
                    println!("{}", json!({"rig":$version,"fixture":case,"terminal":"error", "text":"","http_requests":http.requests().len(),"tool_dispatches":0}));
                    (false, String::new())
                },
            };
            assert_eq!(http.requests().len(), 1, "one request; no retry");
            assert_eq!(http.remaining_responses(), 0);
            observed
        }
    } };
}
compare_version!(old, rig43, "0.43.0");
compare_version!(new, rig44, "0.44.0");
#[tokio::main(flavor = "current_thread")]
async fn main() {
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
    ] {
        let old = old::run(name, sse.clone()).await;
        let new = new::run(name, sse).await;
        match name {
            "valid_control" => {
                assert_eq!(old, (true, "Synthetic valid answer".into()));
                assert_eq!(new, old);
            }
            "recognized_text_delta_wrong_type" => {
                assert_eq!(old, (false, String::new()));
                assert_eq!(new, (true, "Synthetic later text".into()));
            }
            "invalid_json" => {
                assert_eq!(old, (false, String::new()));
                assert_eq!(new, old);
            }
            _ => unreachable!(),
        }
    }
}
