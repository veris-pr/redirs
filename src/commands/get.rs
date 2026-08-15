use crate::request::Request;
use crate::resp::RESP;
use crate::server::Server;
use crate::server_result::{ServerError, ServerValue};

pub async fn command(server: &mut Server, request: &Request, command: &Vec<String>) {
    let storage = match server.storage.as_mut() {
        Some(storage) => storage,
        _ => {
            request.error(ServerError::StorageNotInitialised).await;
            return;
        }
    };

    if command.len() != 2 {
        request
            .error(ServerError::CommandSyntaxError(command.join(" ")))
            .await;
        return;
    }

    let output = storage.get(command[1].clone());

    match output {
        Ok(Some(v)) => request.data(ServerValue::RESP(RESP::BulkString(v))).await,
        Ok(None) => request.data(ServerValue::RESP(RESP::Null)).await,
        Err(_) => {
            request
                .error(ServerError::CommandInternalError(command.join(" ")))
                .await
        }
    };
}
