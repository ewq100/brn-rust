use super::*;
use rig::agent::StreamingError;
use rig::error::ProviderError;

#[tokio::test]
async fn requires_final_response() {
    let mut stream: StreamingResult = Box::pin(futures::stream::empty());
    let error = print_stream(&mut stream, &mut 0).await.unwrap_err();
    assert_eq!(error.to_string(), "stream ended without a final response");
}

#[tokio::test]
async fn propagates_first_error_without_reading_final_response() {
    let mut stream: StreamingResult = Box::pin(futures::stream::iter([
        Err(StreamingError::Completion(ProviderError::Response(
            "synthetic stream failure".into(),
        ))),
        Ok(MultiTurnStreamItem::FinalResponse(PromptResponse::empty())),
    ]));
    let error = print_stream(&mut stream, &mut 0).await.unwrap_err();
    assert!(error.to_string().contains("synthetic stream failure"));
    assert!(matches!(
        stream.next().await,
        Some(Ok(MultiTurnStreamItem::FinalResponse(_)))
    ));
}

#[tokio::test]
async fn captures_final_response() {
    let response = PromptResponse::new("5", Default::default());
    let mut stream: StreamingResult = Box::pin(futures::stream::iter([Ok(
        MultiTurnStreamItem::FinalResponse(response),
    )]));
    let response = print_stream(&mut stream, &mut 0).await.unwrap();
    assert_eq!(response.output(), "5");
}

#[tokio::test]
async fn integer_schema_and_checked_arithmetic() {
    let params = Adder.parameters();
    assert_eq!(params["properties"]["x"]["type"], "integer");
    assert_eq!(params["properties"]["y"]["type"], "integer");
    let mut context = rig::tool::ToolContext::default();
    assert_eq!(
        Adder
            .call(&mut context, AddArgs { x: 2, y: 3 })
            .await
            .unwrap(),
        5
    );
    for args in [
        AddArgs { x: i64::MAX, y: 1 },
        AddArgs { x: i64::MIN, y: -1 },
    ] {
        assert!(Adder.call(&mut context, args).await.is_err());
    }
}
