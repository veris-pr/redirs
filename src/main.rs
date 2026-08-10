use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::resp::{RESP, bytes_to_resp};
use crate::server::process_request;
use crate::storage::Storage;

mod resp;
mod resp_result;
mod server;
mod set;
mod storage;
mod storage_result;

const BUFFER_SIZE: usize = 512;
const ADDRESS: &str = "127.0.0.1";
const PORT: u16 = 6379;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind(format!("{}:{}", ADDRESS, PORT)).await?;
    let storage = Arc::new(Mutex::new(Storage::new()));
    let mut interval_timer = tokio::time::interval(Duration::from_millis(10));
    loop {
        tokio::select! {
            connection = listener.accept() => {
                match connection {
                    Ok((stream, _)) => {
                        tokio::spawn(handle_stream(stream,storage.clone()));
                    },
                    Err(e) => {
                        eprintln!("Error accepting connection: {}", e);
                        continue;
                    }
                }
            }
            _ = interval_timer.tick() => {
                tokio::spawn(expire_keys(storage.clone()));
            }
        }
    }
}

async fn handle_stream(mut stream: TcpStream, storage: Arc<Mutex<Storage>>) {
    let mut buffer = [0; BUFFER_SIZE];
    loop {
        // stream.read(&mut buffer).unwrap();
        // println!("Received: {:?}", buffer);
        // let response = "+PONG\r\n";
        // stream.write(response.as_bytes()).unwrap();
        // stream.flush().unwrap();
        match stream.read(&mut buffer).await {
            Ok(size) if size != 0 => {
                let mut index: usize = 0;
                let request = match bytes_to_resp(&buffer[..size].to_vec(), &mut index) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        return;
                    }
                };
                let response = match process_request(request, storage.clone()) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Error parsing command: {}", e);
                        return;
                    }
                };
                if let Err(e) = stream.write_all(response.to_string().as_bytes()).await {
                    eprintln!("Error writing to socket: {}", e);
                }
            }
            Ok(_) => {
                println!("Connection closed by client");
                break;
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }
}

async fn expire_keys(storage: Arc<Mutex<Storage>>) {
    let mut guard = storage.lock().unwrap();
    guard.expire_keys();
}
