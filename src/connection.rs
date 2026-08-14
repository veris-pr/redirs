use crate::resp::{RESP, bytes_to_resp};
use crate::server::{ServerInfo, handshake};
use crate::server_result::{ServerMessage, ServerValue};
use crate::{request::Request, server_result::ServerError};
use std::fmt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

const BUFFER_SIZE: usize = 512;

#[derive(Debug)]
pub enum ConnectionError {
    CannotWriteToStream(String),
    ServerError(ServerError),
    CannotReadFromStream(String),
    MalformedRESP(String),
    RequestFailed(String, String),
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionError::CannotReadFromStream(string) => {
                write!(f, "Cannot read from stream: {}.", string)
            }
            ConnectionError::MalformedRESP(string) => {
                write!(f, "Cannot convert bytes to RESP: {}.", string)
            }
            ConnectionError::CannotWriteToStream(string) => {
                write!(f, "Cannot write to stream: {}.", string)
            }
            ConnectionError::ServerError(e) => {
                write!(f, "{}", format!("Server error: {}", e))
            }
            ConnectionError::RequestFailed(request, e) => {
                write!(f, "Request {} failed: {}.", request, e)
            }
        }
    }
}

#[derive(Debug)]
pub enum ConnectionMessage {
    Request(Request),
}

type ConnectionResult<T> = Result<T, ConnectionError>;

pub async fn stream_write_resp(stream: &mut TcpStream, data: &RESP) -> ConnectionResult<usize> {
    // Convert the RESP message to a String.
    let string_data = data.to_string();

    // Convert the String into bytes.
    let bytes = string_data.as_bytes();

    // Write the bytes to the TCP stream.
    match stream.write_all(bytes).await {
        Ok(_) => Ok(bytes.len()),
        Err(e) => Err(ConnectionError::CannotWriteToStream(e.to_string())),
    }
}

async fn stream_read_resp(stream: &mut TcpStream, buffer: &mut [u8]) -> ConnectionResult<RESP> {
    // Read bytes from the TCP stream.
    match stream.read(buffer).await {
        Ok(size) => Ok(size),
        Err(e) => Err(ConnectionError::CannotReadFromStream(e.to_string())),
    }?;

    // Set the index to start reading from the first byte.
    let mut index: usize = 0;

    // Convert bytes to RESP.
    bytes_to_resp(&buffer, &mut index).map_err(|e| ConnectionError::MalformedRESP(e.to_string()))
}

pub async fn stream_send_receive_resp(
    stream: &mut TcpStream,
    data: &RESP,
    buffer: &mut [u8],
) -> ConnectionResult<RESP> {
    // Write the RESP command to the stream.
    stream_write_resp(stream, &data)
        .await
        .map_err(|e| ConnectionError::RequestFailed(data.to_string(), e.to_string()))?;

    // Read the response in RESP format from the stream.
    stream_read_resp(stream, buffer)
        .await
        .map_err(|e| ConnectionError::RequestFailed(data.to_string(), e.to_string()))
}

pub async fn run_listener(host: String, port: u16, server_sender: mpsc::Sender<ConnectionMessage>) {
    // Create the TCP listener, bound to the given port.
    let listener = TcpListener::bind(format!("{}:{}", host, port))
        .await
        .unwrap();

    loop {
        // Process each incoming connection.
        tokio::select! {
            // Process a new connection.
            connection = listener.accept() => {
                match connection {
                    // The connection is valid, handle it.
                    Ok((stream, _)) => {
                        // Spawn a task to take care of this connection.
                        tokio::spawn(handle_connection(stream, server_sender.clone()));
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        continue;
                    }
                }
            }
        }
    }
}

async fn handle_connection(mut stream: TcpStream, server_sender: mpsc::Sender<ConnectionMessage>) {
    // Create a buffer to host incoming data.
    let mut buffer = [0; BUFFER_SIZE];
    let (connection_sender, mut connection_receiver) = mpsc::channel::<ServerMessage>(32);
    loop {
        tokio::select! {
            // Data is available in the stream, read it.
            result = stream.read(&mut buffer) => {
                // Check if the incoming data is valid
                // and act accordingly.
                match result {
                    // If the stream returned some data,
                    // process the request.
                    Ok(size) if size != 0 => {
                        // Initialise the index to start at the
                        // beginning of the buffer.
                        let mut index: usize = 0;

                        // Process the bytes in the buffer according to
                        // the content and extract the request. Update the index.
                        let resp = match bytes_to_resp(&buffer[..size].to_vec(), &mut index) {
                            Ok(v) => v,
                            Err(e) => {
                                eprintln!("Error: {}", e);
                                return;
                            }
                        };

                        let request = Request {
                            value: resp,
                            sender: connection_sender.clone(),
                        };

                        match server_sender.send(ConnectionMessage::Request(request)).await {
                            Ok(()) => {},
                            Err(r) => {
                                eprintln!("Error sending request to server: {}", r);
                                return;
                            }
                        }
                    }
                    // If the stream returned no data
                    // the connection has been closed.
                    Ok(_) => {
                        println!("Connection closed");
                        break;
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                        break;
                    }
                }
            }

            Some(response) = connection_receiver.recv() => {
                let _ = match response {
                    ServerMessage::Data(ServerValue::RESP(v)) => stream.write_all(v.to_string().as_bytes()).await,
                    ServerMessage::Error(e) => {
                        eprintln!("Error: {}", ConnectionError::ServerError(e));
                        return;
                    }
                    ServerMessage::Data(ServerValue::None) => Ok(()),
                    ServerMessage::Data(ServerValue::Binary(data)) => stream.write_all(&data).await,
                };
            }
        }
    }
}

pub async fn run_master_listener(
    host: String,
    port: u16,
    info: &ServerInfo,
    server_sender: mpsc::Sender<ConnectionMessage>,
) {
    let mut stream = TcpStream::connect(format!("{}:{}", host, port))
        .await
        .unwrap();
    if let Err(e) = handshake(&mut stream, info).await {
        eprintln!("Handshake failed: {}", e.to_string());
        std::process::exit(1);
    }
    tokio::spawn(async move { handle_connection(stream, server_sender.clone()).await });
}
