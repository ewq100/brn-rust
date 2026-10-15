use std::{
    fs,
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    time::Duration,
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let address: SocketAddr = args[1].parse().unwrap();
    let outside = PathBuf::from(&args[2]);
    let scratch = PathBuf::from(&args[3]);
    let external_denied = fs::read(outside).is_err();
    let network_denied = TcpStream::connect_timeout(&address, Duration::from_secs(1)).is_err();
    let scratch_allowed =
        fs::write(scratch.join("witness.txt"), b"synthetic scratch write").is_ok();
    println!(
        "{{\"external_denied\":{external_denied},\"network_denied\":{network_denied},\"scratch_allowed\":{scratch_allowed}}}"
    );
    assert!(external_denied && network_denied && scratch_allowed);
}
