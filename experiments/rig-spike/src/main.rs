use anyhow::{Result, bail};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["graph"] => graph(),
        _ => bail!("usage: rig-spike graph"),
    }
}

fn graph() -> Result<()> {
    let conn = rusqlite::Connection::open_in_memory()?;
    conn.execute_batch(
        "CREATE VIRTUAL TABLE t USING fts5(body, tokenize='unicode61 remove_diacritics 2');
         INSERT INTO t VALUES ('red blue'), ('reddish blueprint');",
    )?;
    let hits: i64 = conn.query_row("SELECT count(*) FROM t WHERE t MATCH '\"red\"'", [], |r| {
        r.get(0)
    })?;
    println!("sqlite {} fts5 hits for red: {hits}", rusqlite::version());
    let embed: fn(fastembed::TextInitOptions) -> _ = fastembed::TextEmbedding::try_new;
    std::hint::black_box(embed);
    let app: fn() -> _ = gpui_kit::application;
    std::hint::black_box(app);
    std::hint::black_box(rig::providers::chatgpt::PROVIDER_NAME);
    std::hint::black_box(rig::providers::copilot::PROVIDER_NAME);
    Ok(())
}
