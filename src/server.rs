use crate::commands::{echo, get, info, ping, psync, replconf, set};
use crate::replication::ReplicationConfig;
use crate::request::Request;
use crate::resp::{resp_extract_length, resp_process_type};
use crate::server_result::{ServerError, ServerMessage, ServerResult, ServerValue};
use crate::{RESP, storage::Storage};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::{io::AsyncWriteExt, net::TcpStream};

pub struct ServerInfo {
    #[allow(dead_code)]
    pub host: String,
    pub port: u16,
}
use crate::connection::{
    ConnectionMessage, stream_read_data_length, stream_read_line, stream_send_receive_resp,
};

pub struct Server {
    pub info: ServerInfo,
    pub storage: Option<Storage>,
    pub replication: ReplicationConfig,
    pub replica_senders: Vec<mpsc::Sender<ServerMessage>>,
}

impl Server {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            info: ServerInfo { host, port },
            storage: None,
            replication: ReplicationConfig::new_master(),
            replica_senders: Vec::new(),
        }
    }

    pub fn set_storage(&mut self, storage: Storage) {
        self.storage = Some(storage);
    }

    pub fn set_replication(&mut self, replication: ReplicationConfig) {
        self.replication = replication;
    }

    pub fn expire_keys(&mut self) {
        let storage = match self.storage.as_mut() {
            Some(storage) => storage,
            _ => return,
        };
        storage.expire_keys();
    }

    pub fn generate_rdb(&self) -> Vec<u8> {
        let v: Vec<u8> = vec![
            0x52, 0x45, 0x44, 0x49, 0x53, 0x30, 0x30, 0x31, 0x31, 0xfa, 0x09, 0x72, 0x65, 0x64,
            0x69, 0x73, 0x2d, 0x76, 0x65, 0x72, 0x05, 0x37, 0x2e, 0x32, 0x2e, 0x30, 0xfa, 0x0a,
            0x72, 0x65, 0x64, 0x69, 0x73, 0x2d, 0x62, 0x69, 0x74, 0x73, 0xc0, 0x40, 0xfa, 0x05,
            0x63, 0x74, 0x69, 0x6d, 0x65, 0xc2, 0x6d, 0x08, 0xbc, 0x65, 0xfa, 0x08, 0x75, 0x73,
            0x65, 0x64, 0x2d, 0x6d, 0x65, 0x6d, 0xc2, 0xb0, 0xc4, 0x10, 0x00, 0xfa, 0x08, 0x61,
            0x6f, 0x66, 0x2d, 0x62, 0x61, 0x73, 0x65, 0xc0, 0x00, 0xff, 0xf0, 0x6e, 0x3b, 0xfe,
            0xc0, 0xff, 0x5a, 0xa2,
        ];

        v
    }
}

pub async fn run_server(mut server: Server, mut crx: mpsc::Receiver<ConnectionMessage>) {
    let mut interval_timer = tokio::time::interval(Duration::from_millis(10));

    loop {
        tokio::select! {
            Some(message) = crx.recv() => {
                match message {
                    ConnectionMessage::Request(request) => {
                        process_request(request, &mut server).await;
                    }
                }
            }
            _ = interval_timer.tick() => {
                server.expire_keys();
            }
        }
    }
}

pub async fn process_request(request: Request, server: &mut Server) {
    let elements = match &request.value {
        RESP::Array(elements) => elements,
        _ => {
            request.error(ServerError::IncorrectData).await;
            return;
        }
    };

    let mut command = Vec::new();

    for elem in elements.iter() {
        match elem {
            RESP::BulkString(s) => command.push(s.clone()),
            _ => {
                request.error(ServerError::IncorrectData).await;
                return;
            }
        }
    }
    let command_name = command[0].to_lowercase();

    // Process the request using the requested command.
    match command_name.as_str() {
        "echo" => {
            echo::command(server, &request, &command).await;
        }
        "get" => {
            get::command(server, &request, &command).await;
        }
        "ping" => {
            ping::command(server, &request, &command).await;
        }
        "set" => {
            set::command(server, &request, &command).await;
            send_request_to_replicas(&request, server).await;
        }
        "info" => {
            info::command(server, &request, &command).await;
        }
        "replconf" => {
            replconf::command(server, &request, &command).await;
        }
        "psync" => {
            psync::command(server, &request, &command).await;
        }
        _ => {
            request
                .error(ServerError::CommandNotAvailable(command[0].clone()))
                .await;
        }
    }
}
pub async fn send_request_to_replicas(request: &Request, server: &Server) {
    for replica_sender in server.replica_senders.iter() {
        let _ = replica_sender
            .send(ServerMessage::Data(ServerValue::Binary(
                request.binary.clone(),
            )))
            .await;
    }
}

pub async fn handshake(stream: &mut TcpStream, info: &ServerInfo) -> ServerResult {
    let mut buffer = [0; 512];
    let ping = RESP::Array(vec![RESP::BulkString(String::from("PING"))]);

    // Send the command and read the response.
    let resp = stream_send_receive_resp(stream, &ping, &mut buffer)
        .await
        .map_err(|e| ServerError::HandshakeFailed(e.to_string()))?;

    // Check that the response is correct.
    if resp != RESP::SimpleString(String::from("PONG")) {
        return Err(ServerError::HandshakeFailed(String::from("PING failed")));
    };

    let replconf = RESP::Array(vec![
        RESP::BulkString(String::from("REPLCONF")),
        RESP::BulkString(String::from("listening-port")),
        RESP::BulkString(info.port.to_string()),
    ]);

    // Send the command and read the response.
    let resp = stream_send_receive_resp(stream, &replconf, &mut buffer)
        .await
        .map_err(|e| ServerError::HandshakeFailed(e.to_string()))?;

    // Check that the response is correct.
    if resp != RESP::SimpleString(String::from("OK")) {
        return Err(ServerError::HandshakeFailed(format!(
            "Sending {} - Wrong server answer: {}",
            replconf.to_string(),
            resp.to_string()
        )));
    };

    let replconf = RESP::Array(vec![
        RESP::BulkString(String::from("REPLCONF")),
        RESP::BulkString(String::from("capa")),
        RESP::BulkString(String::from("psync2")),
    ]);

    // Send the command and read the response.
    let resp = stream_send_receive_resp(stream, &replconf, &mut buffer)
        .await
        .map_err(|e| ServerError::HandshakeFailed(e.to_string()))?;

    // Check that the response is correct.
    if resp != RESP::SimpleString(String::from("OK")) {
        return Err(ServerError::HandshakeFailed(format!(
            "Sending {} - Wrong server answer: {}",
            replconf.to_string(),
            resp.to_string()
        )));
    };

    let psync = RESP::Array(vec![
        RESP::BulkString(String::from("PSYNC")),
        RESP::BulkString(String::from("?")),
        RESP::BulkString(String::from("-1")),
    ]);

    // Send the PSYNC command.
    stream
        .write_all(psync.to_string().as_bytes())
        .await
        .map_err(|e| {
            ServerError::HandshakeFailed(format!(
                "Sending {} - Cannot write to stream: {}",
                replconf.to_string(),
                e.to_string()
            ))
        })?;

    // Read the RDB length.
    if let Some(_) = stream_read_line(stream, &mut buffer).await.err() {
        return Err(ServerError::HandshakeFailed(String::from(
            "PSYNC failed, cannot read RDB length",
        )));
    }

    let mut index = 0;

    // Remove the dollar sign.
    if let Some(_) = resp_process_type('$', &buffer, &mut index).err() {
        return Err(ServerError::HandshakeFailed(String::from(
            "PSYNC failed, RDB doesn't start with $",
        )));
    }

    // Convert bytes into RDB length.
    let length = resp_extract_length(&buffer, &mut index).unwrap();

    // Read the RDB data.
    if let Some(_) = stream_read_data_length(stream, &mut buffer, length as usize)
        .await
        .err()
    {
        return Err(ServerError::HandshakeFailed(String::from(
            "PSYNC failed, cannot read RDB",
        )));
    }

    Ok(ServerValue::None)
}
