use std::{io::{self, BufRead, Read, Write}, os::unix::net::UnixListener};

use bufstream::BufStream;

fn main() -> io::Result<()> {
    let listener = UnixListener::bind("/tmp/demo_server.sock")?;
    let (stream, addr) = listener.accept()?;
    let mut stream = BufStream::new(stream);
    println!("New coming connection; server listening on: {:?}", addr);
    let mut buf = Vec::new();
    while let Ok(read_bytes) = stream.read_until(b'\n', &mut buf) {
        if read_bytes != 0 {
            println!("Server received {} bytes: {:?}", read_bytes, buf);
            stream.write_all(b"Server Respond: Hello too!")?;
        }
    }
    std::fs::remove_file("/tmp/demo_server.sock")?;
    Ok(())
}