//! One explicitly authorized public-query qualification of the production adapter.
//! No default query, retries, workspace content, or account override.
use brn_ai::{Provider, ReasoningEffort, Selection, subscription_search};
use serde_json::json;
use std::process::ExitCode;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 || args[0] != "--live-public-query" {
        eprintln!(
            "usage: subscription-search --live-public-query EXACT_MODEL PUBLIC_QUERY\nRequires owner authorization and a durable live-turn reservation; uses normal managed login, medium effort, one request, 60-second deadline."
        );
        return ExitCode::from(2);
    }
    let request = subscription_search::Request {
        selection: Selection {
            provider: Provider::Chatgpt,
            model: args[1].clone(),
        },
        effort: ReasoningEffort::Medium,
        query: args[2].clone(),
        auth_file: None,
    };
    match subscription_search::search(request, CancellationToken::new()).await {
        Ok(result) => {
            let sources: Vec<_> = result
                .sources
                .into_iter()
                .map(|s| json!({"url":s.url,"title":s.title}))
                .collect();
            println!(
                "{}",
                json!({"status":"passed","answer":result.answer,"sources":sources,"provider":"chatgpt","model":result.provenance.model,"effort":"medium","cost":null})
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!(
                "{}",
                json!({"status":"failed","error":format!("{error:?}"),"message":error.to_string(),"cost":null})
            );
            ExitCode::FAILURE
        }
    }
}
