use crate::replication::Role;
use crate::request::Request;
use crate::resp::bulk_string_from_vec;
use crate::server::Server;
use crate::server_result::ServerValue;

pub async fn command(server: &mut Server, request: &Request, _command: &Vec<String>) {
    // Get the replication info from the server.
    let replication_info = server.replication.info();

    // Add a header to the output.
    let mut output = vec![String::from("# Replication")];

    // Add the correct content to the output
    // according to the replication role.
    match replication_info.role {
        Role::Replica => output.push(String::from("role:slave")),
        Role::Master => output.push(String::from("role:master")),
    };

    output.push(format!("master_replid:{}", replication_info.replid));
    output.push(format!(
        "master_repl_offset:{}",
        replication_info.repl_offset
    ));

    request
        .data(ServerValue::RESP(bulk_string_from_vec(output)))
        .await;
}
