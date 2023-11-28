use std::net::SocketAddr;
use chat::server::Server;
use anyhow::Result;

fn main() -> Result<()> {
    let addr: SocketAddr = "127.0.0.1:6378".parse()?;
    let mut server = Server::new(addr)?;
    server.run()?;
    Ok(())
}
