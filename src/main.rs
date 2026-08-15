use crate::storage::Storage;
use crate::{replication::ReplicationConfig, resp::RESP};
use clap::Parser;
use connection::{ConnectionMessage, run_listener, run_master_listener};
use server::{Server, run_server};
use tokio::sync::mpsc;

mod commands;
mod connection;
mod replication;
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short='H', long, help="The host address", default_value_t = String::from(ADDRESS))]
    host: String,
    #[arg(short, long, help="The port number", default_value_t = PORT)]
    port: u16,
    #[arg(
        short,
        long,
        help = "The master server for this replica, in the form `address port`"
    )]
    replicaof: Option<String>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let replication_config = match args.replicaof {
        None => ReplicationConfig::new_master(),
        Some(params) => {
            // Split host and port that are provided as a single string.
            let (host, port_string) = match params.split_once(" ") {
                Some(value) => value,
                None => {
                    eprintln!("Please provide 'HOST PORT' separated by space");
                    std::process::exit(1);
                }
            };

            // Convert the port into a number.
            let port: u16 = match port_string.parse() {
                Ok(p) => p,
                Err(_) => {
                    eprintln!("Port is not a number");
                    std::process::exit(1);
                }
            };

            ReplicationConfig::new_replica(host.to_owned(), port)
        }
    };
    let mut storage = Storage::new();
    storage.set_active_expiry(true);
    let mut server = Server::new(args.host.clone(), args.port);
    server.set_storage(storage);
    server.set_replication(replication_config);

    let (server_sender, server_receiver) = mpsc::channel::<ConnectionMessage>(32);

    if let Some(master_config) = server.replication.master.clone() {
        run_master_listener(
            master_config.host.clone(),
            master_config.port,
            &server.info,
            server_sender.clone(),
        )
        .await;
    }

    tokio::spawn(run_server(server, server_receiver));
    run_listener(args.host, args.port, server_sender).await;

    Ok(())
}
