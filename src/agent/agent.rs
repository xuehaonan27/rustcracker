//! Firecracker Agent
use crate::reqres::FirecrackerEvent;
use fslock::LockFile;
use log::*;
use tokio::io::{AsyncWriteExt, ErrorKind};
use tokio::net::UnixStream;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Bad HTTP request: {0}")]
    BadRequest(String),
    #[error("Bad HTTP response: {0}")]
    BadResponse(String),
    #[error("Bad unix socket: {0}")]
    BadUnixSocket(String),
    #[error("Bad lock file: {0}")]
    BadLockFile(String),
}

pub type AgentResult<T> = std::result::Result<T, AgentError>;

// 1024 bytes are enough for firecracker response headers
const MAX_BUFFER_SIZE: usize = 1024;
#[derive(Debug)]
pub struct Agent {
    stream: UnixStream,
    lock: LockFile,
}

impl Agent {
    #[allow(unused)]
    pub async fn new(stream_path: String, lock_path: String) -> Result<Self, AgentError> {
        let stream = UnixStream::connect(&stream_path)
            .await
            .map_err(|e| AgentError::BadUnixSocket(e.to_string()))?;
        // You should make sure that `lock_path` contains no nul-terminators
        let lock =
            LockFile::open(&lock_path).map_err(|e| AgentError::BadLockFile(e.to_string()))?;
        Ok(Self { stream, lock })
    }

    pub fn from_stream_lock(stream: UnixStream, lock: LockFile) -> Self {
        Self { stream, lock }
    }

    pub fn is_locked(&self) -> bool {
        self.lock.owns_lock()
    }

    pub fn lock(&mut self) -> AgentResult<()> {
        self.lock.lock().map_err(|e| {
            let msg = format!("When locking the lock file: {e}");
            error!("{msg}");
            AgentError::BadLockFile(msg)
        })
    }

    pub fn unlock(&mut self) -> AgentResult<()> {
        self.lock.unlock().map_err(|e| {
            let msg = format!("When unlocking the lock file: {e}");
            error!("{msg}");
            AgentError::BadLockFile(msg)
        })
    }

    /// One should make sure that this Agent is locked up before sending any data into the stream
    pub async fn send_request(&mut self, request: String) -> AgentResult<()> {
        assert!(self.is_locked());

        // Wait for the stream available to write
        self.stream.writable().await.map_err(|e| {
            let msg = format!("When waiting for the socket stream become writable: {e}");
            error!("{msg}");
            AgentError::BadRequest(msg)
        })?;

        self.stream
            .write_all(&request.as_bytes())
            .await
            .map_err(|e| {
                let msg = format!("When writing to the socket stream: {e}");
                error!("{msg}");
                AgentError::BadRequest(msg)
            })?;

        self.stream.flush().await.map_err(|e| {
            let msg = format!("When flushing the socket stream: {e}");
            error!("{msg}");
            AgentError::BadRequest(msg)
        })?;
        Ok(())
    }

    /// One should make sure that this Agent is locked up before receiving any data from the stream
    pub async fn recv_response(&mut self) -> AgentResult<Vec<u8>> {
        assert!(self.is_locked());
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut res = httparse::Response::new(&mut headers);

        let mut buf = [0u8; MAX_BUFFER_SIZE];

        let mut vec: Vec<u8> = Vec::new();
        loop {
            self.stream.readable().await.map_err(|e| {
                let msg = format!("When waiting for the socket stream become readable: {e}");
                error!("{msg}");
                AgentError::BadResponse("waiting readable".into())
            })?;

            match self.stream.try_read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    vec.extend_from_slice(&mut buf);
                    if n < MAX_BUFFER_SIZE {
                        // No need for checking again
                        break;
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
                Err(e) => {
                    let msg = format!("Bad reading from the socket: {e}");
                    error!("{msg}");
                    return Err(AgentError::BadUnixSocket(msg));
                }
            }
        }

        let body_start = res.parse(&vec).unwrap();
        if body_start.is_partial() {
            // Bad response
            let msg = "Got incomplete response";
            error!("{msg}");
            return Err(AgentError::BadResponse(msg.into()));
        }
        let body_start = body_start.unwrap();

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

    pub async fn clear_stream(&mut self) -> AgentResult<()> {
        let mut buf = [0; 1024];
        let read: bool = loop {
            match self.stream.try_read(&mut buf) {
                Ok(0) => break true,
                Ok(_) => continue,
                Err(ref e) if e.kind() == tokio::io::ErrorKind::WouldBlock => break true,
                Err(_) => break false,
            }
        };

        // Clear write buffer
        let write = self.stream.write_all(b"").await.is_ok();
        let flush = self.stream.flush().await.is_ok();
        if read && write && flush {
            Ok(())
        } else {
            let msg = "Fail to clear the socket stream";
            error!("{msg}");
            Err(AgentError::BadUnixSocket(msg.into()))
        }
    }

    /// Start a single event by passing a FirecrackerEvent like object
    pub async fn event<E: FirecrackerEvent>(&mut self, event: E) -> AgentResult<E::Res> {
        self.lock()?;
        if let Err(e) = self.clear_stream().await {
            self.unlock()?;
            return Err(e);
        }
        if let Err(e) = self.send_request(event.req()).await {
            self.unlock()?;
            return Err(e);
        }
        let res = match self.recv_response().await {
            Ok(res) => res,
            Err(e) => {
                self.unlock()?;
                return Err(e);
            }
        };
        self.unlock()?;
        E::decode(&res).map_err(|e| {
            let msg = format!("Fail to decode response: {e}");
            error!("{msg}");
            AgentError::BadResponse(msg.into())
        })
    }

    /// Start some events by passing FirecrackerEvent like objects
    /// Useful since less locking and unlocking needed
    #[allow(unused)]
    pub async fn events<E: FirecrackerEvent>(
        &mut self,
        events: Vec<E>,
    ) -> AgentResult<Vec<E::Res>> {
        self.lock()?;
        self.clear_stream().await?;

        // TODO: change to async iterator after `std::async_iter` is available in stable Rust.
        let mut res_vec = Vec::with_capacity(events.len());
        for event in events {
            self.send_request(event.req()).await?;
            let res = self.recv_response().await?;
            let res = E::decode(&res).map_err(|e| {
                let msg = format!("Fail to decode response: {e}");
                error!("{msg}");
                AgentError::BadResponse(msg.into())
            })?;
            res_vec.push(res);
        }
        self.unlock()?;
        Ok(res_vec)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

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
    const LOCK_PATH: &'static str = "/tmp/rtck_test.lock";

    async fn run_server(unique_test_id: usize, expected_request: Vec<u8>) {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::UnixListener,
        };
        let socket_path = &format!("{}{}", SOCKET_PATH, unique_test_id);

        if std::path::Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path).unwrap();
        }

        let listener = UnixListener::bind(socket_path).unwrap();

        loop {
            let (mut stream, _) = listener.accept().await.unwrap();

            let mut buffer = [0; 1024];
            let n = stream.read(&mut buffer).await.unwrap();
            if n > 0 {
                let received_data = &buffer[0..n];
                let body = if &expected_request == received_data {
                    "I've received correct request!".to_string()
                } else {
                    "Bad request!".to_string()
                };
                let body_len = body.len();
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body_len, body);
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        }
    }

    #[tokio::test]
    async fn test_recv_response_with_body() {
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

        let unique_test_id = 1;
        let server_task = tokio::spawn(run_server(unique_test_id, request_s.as_bytes().to_vec()));
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let stream_path = format!("{}{}", SOCKET_PATH, unique_test_id);
        let lock_path = format!("{}{}", LOCK_PATH, unique_test_id);
        let mut agent = Agent::new(stream_path, lock_path).await.unwrap();

        agent.lock().unwrap();
        agent.send_request(request_s).await.unwrap();
        let res = agent.recv_response().await.unwrap();
        agent.unlock().unwrap();

        let body = "I've received correct request!".to_string();
        server_task.abort();

        assert_eq!(res, body.as_bytes().to_vec());
    }

    async fn event_server(unique_test_id: usize) {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::UnixListener,
        };
        let socket_path = &format!("{}{}", SOCKET_PATH, unique_test_id);

        if std::path::Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path).unwrap();
        }

        let listener = UnixListener::bind(socket_path).unwrap();

        println!("Server ready");

        loop {
            let (mut stream, _) = listener.accept().await.unwrap();

            let mut buffer = [0; 1024];
            let n = stream.read(&mut buffer).await.unwrap();
            if n > 0 {
                let received_data = &buffer[0..n];
                println!("event_server: received_data = {:?}", received_data);

                let body = FirecrackerVersion {
                    firecracker_version: "demo-dev".to_string(),
                };
                let body = serde_json::to_string(&body).unwrap();
                let body_len = body.len();
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body_len, body);
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        }
    }

    #[tokio::test]
    async fn test_event() {
        let event = GetFirecrackerVersion(GetFirecrackerVersionRequest);

        let unique_test_id = 2;
        let server_task = tokio::spawn(event_server(unique_test_id));
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let stream_path = format!("{}{}", SOCKET_PATH, unique_test_id);
        let lock_path = format!("{}{}", LOCK_PATH, unique_test_id);
        let mut agent = Agent::new(stream_path, lock_path).await.unwrap();

        println!("Launching event");
        let res = agent.event(event).await.unwrap();

        server_task.abort();
        assert!(res.is_succ());

        let body = FirecrackerVersion {
            firecracker_version: "demo-dev".to_string(),
        };
        assert_eq!(res.0.left().unwrap(), body);
    }
}
