use connection::{ConnectionMessage, run_listener};
use server::{Server, run_server};
use tokio::sync::mpsc;

use crate::resp::RESP;
use crate::storage::Storage;

mod connection;
mod request;
mod resp;
mod resp_result;
mod server;
mod server_result;
mod set;
mod storage;
mod storage_result;

const ADDRESS: &str = "127.0.0.1";
const PORT: u16 = 6379;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut storage = Storage::new();
    storage.set_active_expiry(true);
    let mut server = Server::new();
    server.set_storage(storage);

    let (server_sender, server_receiver) = mpsc::channel::<ConnectionMessage>(32);
    tokio::spawn(run_server(server, server_receiver));
    run_listener(ADDRESS.to_string(), PORT, server_sender).await;

    Ok(())
}
