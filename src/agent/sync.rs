//! Sync firecracker socket agent
use super::{SocketAgent, MAX_BUFFER_SIZE};
use crate::reqres::FirecrackerEvent;
use crate::{RtckError, RtckResult};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

#[derive(Debug)]
pub struct SocketAgentSync {
    stream: UnixStream,
}

impl SocketAgent for SocketAgentSync {
    type StreamType = UnixStream;
    fn new<P: AsRef<Path>>(socket_path: P) -> RtckResult<Self> {
        let stream = UnixStream::connect(socket_path)?;
        stream.set_nonblocking(true)?;
        Ok(Self { stream })
    }

    fn from_stream(stream: Self::StreamType) -> Self {
        Self { stream }
    }

    fn into_inner(self) -> Self::StreamType {
        self.stream
    }
}

impl SocketAgentSync {
    pub fn send_request(&mut self, data: &[u8]) -> RtckResult<()> {
        self.stream.write_all(data)?;
        self.stream.flush()?;
        Ok(())
    }

    pub fn recv_response(&mut self) -> RtckResult<Vec<u8>> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut res = httparse::Response::new(&mut headers);

        let mut buf = [0u8; MAX_BUFFER_SIZE];

        let mut vec: Vec<u8> = Vec::new();

        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    vec.extend_from_slice(&mut buf);
                    if n < MAX_BUFFER_SIZE {
                        // No need for checking again
                        break;
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
                Err(e) => return Err(RtckError::Agent(format!("Bad read from socket: {e}"))),
            }
        }

        let body_start = res.parse(&vec).unwrap();
        if body_start.is_partial() {
            return Err(RtckError::Agent("Incomplete response".into()));
        }
        let body_start = body_start.unwrap(); // unwrap safe

        let content_length = res
            .headers
            .iter()
            .find(|h| h.name.to_lowercase() == "content-length")
            .and_then(|h| {
                Some(
                    std::str::from_utf8(h.value)
                        .unwrap()
                        .parse::<usize>()
                        .unwrap(),
                )
            });

        return match content_length {
            None | Some(0) => Ok(b"{ \"empty\": 0 }".to_vec()),
            Some(content_length) => {
                let body = buf[body_start..(body_start + content_length)].to_vec();
                Ok(body)
            }
        };
    }

    pub fn clear_stream(&mut self) -> RtckResult<()> {
        let mut buf = [0; MAX_BUFFER_SIZE];
        let read: bool = loop {
            match self.stream.read(&mut buf) {
                Ok(0) => break true,
                Ok(_) => continue,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break true,
                Err(_) => break false,
            }
        };

        // Clear write buffer
        let write = self.stream.write_all(b"").is_ok();
        let flush = self.stream.flush().is_ok();
        if read && write && flush {
            Ok(())
        } else {
            Err(RtckError::Agent("Fail to clear the socket stream".into()))
        }
    }

    /// Start a single event by passing a FirecrackerEvent like object
    pub fn event<E: FirecrackerEvent>(&mut self, event: E) -> RtckResult<E::Res> {
        if let Err(e) = self.clear_stream() {
            return Err(e);
        }
        if let Err(e) = self.send_request(event.req().as_bytes()) {
            return Err(e);
        }
        let res = match self.recv_response() {
            Ok(res) => res,
            Err(e) => {
                return Err(e);
            }
        };

        E::res(&res).map_err(|e| RtckError::Agent(format!("Fail to decode response: {e}")))
    }

    /// Start some events by passing FirecrackerEvent like objects
    /// Useful since less locking and unlocking needed
    #[allow(unused)]
    pub fn events<E: FirecrackerEvent>(&mut self, events: Vec<E>) -> RtckResult<Vec<E::Res>> {
        self.clear_stream()?;

        // TODO: change to async iterator after `std::async_iter` is available in stable Rust.
        let mut res_vec = Vec::with_capacity(events.len());
        for event in events {
            self.send_request(event.req().as_bytes())?;
            let res = self.recv_response()?;
            let res = E::res(&res)
                .map_err(|e| RtckError::Agent(format!("Fail to decode response: {e}")))?;
            res_vec.push(res);
        }
        Ok(res_vec)
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::HashMap,
        os::unix::net::UnixListener,
        sync::mpsc::{self, Receiver, TryRecvError},
    };

    use crate::{
        agent::{serialize_request, HttpRequest},
        models::FirecrackerVersion,
        reqres::{
            get_firecracker_version::{GetFirecrackerVersion, GetFirecrackerVersionRequest},
            FirecrackerResponse,
        },
    };

    use super::*;
    const SOCKET_PATH: &'static str = "/tmp/rtck_test_uds.sock";

    fn run_server(unique_test_id: usize, expected_request: Vec<u8>, rx: Receiver<()>) {
        let socket_path = &format!("{}{}", SOCKET_PATH, unique_test_id);

        if std::path::Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path).unwrap();
        }

        let listener = UnixListener::bind(socket_path).unwrap();

        loop {
            match rx.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    println!("Terminating.");
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }
            let (mut stream, _) = listener.accept().unwrap();

            let mut buffer = [0; MAX_BUFFER_SIZE];
            let n = stream.read(&mut buffer).unwrap();
            if n > 0 {
                let received_data = &buffer[0..n];
                let body = if &expected_request == received_data {
                    "I've received correct request!".to_string()
                } else {
                    "Bad request!".to_string()
                };
                let body_len = body.len();
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body_len, body);
                stream.write_all(response.as_bytes()).unwrap();
            }
        }
    }

    #[test]
    fn test_recv_response_with_body() {
        // Create an example HTTP request
        let mut request_headers = HashMap::new();
        request_headers.insert("Host".to_string(), "example.com".to_string());
        request_headers.insert("Connection".to_string(), "keep-alive".to_string());
        let body = "this is body".to_string();
        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: request_headers,
            body: Some(body),
        };
        let request_s = serialize_request(&request);
        let request_s_pass = request_s.clone();

        let unique_test_id = 1;
        let (tx, rx) = mpsc::channel();
        let _ = std::thread::spawn(move || {
            run_server(unique_test_id, request_s_pass.as_bytes().to_vec(), rx)
        });
        std::thread::sleep(std::time::Duration::from_secs(1));
        let stream_path = format!("{}{}", SOCKET_PATH, unique_test_id);

        let mut agent = SocketAgentSync::new(stream_path).unwrap();

        agent.send_request(request_s.as_bytes()).unwrap();
        let res = agent.recv_response().unwrap();

        let body = "I've received correct request!".to_string();

        let _ = tx.send(());

        assert_eq!(res, body.as_bytes().to_vec());
    }

    fn event_server(unique_test_id: usize, rx: Receiver<()>) {
        let socket_path = &format!("{}{}", SOCKET_PATH, unique_test_id);

        if std::path::Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path).unwrap();
        }

        let listener = UnixListener::bind(socket_path).unwrap();

        println!("Server ready");

        loop {
            match rx.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    println!("Terminating.");
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }
            let (mut stream, _) = listener.accept().unwrap();

            let mut buffer = [0; MAX_BUFFER_SIZE];
            let n = stream.read(&mut buffer).unwrap();
            if n > 0 {
                let received_data = &buffer[0..n];
                println!("event_server: received_data = {:?}", received_data);

                let body = FirecrackerVersion {
                    firecracker_version: "demo-dev".to_string(),
                };
                let body = serde_json::to_string(&body).unwrap();
                let body_len = body.len();
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body_len, body);
                stream.write_all(response.as_bytes()).unwrap();
            }
        }
    }

    #[test]
    fn test_event() {
        let event = GetFirecrackerVersion(GetFirecrackerVersionRequest);

        let unique_test_id = 2;
        let (tx, rx) = mpsc::channel();
        let _ = std::thread::spawn(move || event_server(unique_test_id, rx));
        std::thread::sleep(std::time::Duration::from_secs(1));
        let stream_path = format!("{}{}", SOCKET_PATH, unique_test_id);
        let mut agent = SocketAgentSync::new(stream_path).unwrap();

        println!("Launching event");
        let res = agent.event(event).unwrap();

        let _ = tx.send(());
        assert!(res.is_succ());

        let body = FirecrackerVersion {
            firecracker_version: "demo-dev".to_string(),
        };
        assert_eq!(res.0.left().unwrap(), body);
    }
}
