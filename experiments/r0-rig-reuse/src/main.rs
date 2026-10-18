//! Credential-free C0 proofs. All HTTP is scripted; MCP uses in-process duplex IO.
fn main() {
    println!("Run cargo test --locked --offline -- --nocapture");
}

#[cfg(test)]
mod mcp;
#[cfg(test)]
mod memory;
#[cfg(test)]
mod prefix;
