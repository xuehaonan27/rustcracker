use serde::{Deserialize, Serialize};

use crate::utils::Json;

/*
Vsock Defines a vsock device, backed by a set of Unix Domain Sockets, on the host side.

For host-initiated connections, Firecracker will be
listening on the Unix socket identified by the path `uds_path`.
Firecracker will create this socket, bind and listen on it.

Host-initiated connections will be performed by connection
to this socket and issuing a connection forwarding
request to the desired guest-side vsock port
(i.e. `CONNECT 52\n`, to connect to port 52).

For guest-initiated connections, Firecracker will expect host
software to be bound and listening on Unix sockets at `uds_path_<PORT>`.
E.g. "/path/to/host_vsock.sock_52" for port number 52.
*/
#[derive(Serialize, Deserialize)]
pub struct Vsock {
    // Guest Vsock CID
    // Required: true
    // Minimum: 3
    guest_cid: u64,

    // Path to UNIX domain socket, used to proxy vsock connections.
    // Required: true
    uds_path: String,

    // vsock id
    // Required: true
    vsock_id: String,
}

impl<'a> Json<'a> for Vsock {
    type Item = Vsock;
}
