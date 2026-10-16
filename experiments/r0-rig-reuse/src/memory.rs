use rig::{
    completion::Message,
    id::ConversationId,
    memory::{
        CompactingMemory, Compactor, ConversationMemory, InMemoryConversationMemory, MemoryError,
        MemoryPolicy, TokenWindowMemory,
    },
    wasm_compat::WasmBoxedFuture,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[derive(Clone)]
struct Stub(Arc<AtomicUsize>);
impl Compactor for Stub {
    type Artifact = Message;
    fn compact<'a>(
        &'a self,
        _: &'a ConversationId,
        evicted: &'a [Message],
        carry: Option<&'a Message>,
    ) -> WasmBoxedFuture<'a, Result<Message, MemoryError>> {
        Box::pin(async move {
            assert!(!evicted.is_empty());
            assert!(carry.is_none());
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Message::user(
                "Untrusted summary: retain early requirement.",
            ))
        })
    }
}
#[tokio::test]
async fn explicit_policy_and_process_local_summary_never_write_source() {
    let canonical = vec![
        Message::user("early requirement"),
        Message::assistant("ack"),
        Message::user("later"),
        Message::assistant("ack2"),
    ];
    let saved = serde_json::to_vec(&canonical).unwrap();
    let policy = TokenWindowMemory::new(2, |_: &Message| 1);
    let kept = policy.apply(canonical.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&kept).unwrap(),
        serde_json::to_value(&canonical[2..]).unwrap()
    );
    let store = Arc::new(InMemoryConversationMemory::new());
    let id = ConversationId::from("synthetic");
    store.append(&id, canonical.clone()).await.unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let adapter = CompactingMemory::new(store.clone(), policy, Stub(calls.clone()));
    let first = adapter.load(&id).await.unwrap();
    assert_eq!(first.len(), 3);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&adapter.load(&id).await.unwrap()).unwrap()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(adapter);
    let restarted = CompactingMemory::new(
        store.clone(),
        TokenWindowMemory::new(2, |_: &Message| 1),
        Stub(calls.clone()),
    );
    restarted.load(&id).await.unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "restart lost summary/watermark and recomputed"
    );
    assert_eq!(
        saved,
        serde_json::to_vec(&store.load(&id).await.unwrap()).unwrap()
    );
    println!(
        "memory: explicit 4→2 window; stub invoked; stable reload; reconstruction compacts twice; canonical bytes unchanged"
    );
}
