mod socket;

pub use socket::{
    connect_tcp, connect_uds, BufferedSocket, Socket, SocketIntoBox, WithSocket, WriteBuffer,
};