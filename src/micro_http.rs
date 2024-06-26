use crate::RtckResult;

pub enum HttpMethod {
    GET,
    PUT,
    PATCH,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::PUT => "PUT",
        }
    }

    pub fn from_str(s: &str) -> RtckResult<Self> {
        match s {
            "GET" | "Get" | "get" => Ok(HttpMethod::GET),
            "PUT" | "Put" | "put" => Ok(HttpMethod::PUT),
            "PATCH" | "Patch" | "patch" => Ok(HttpMethod::PATCH),
            _ => Err(crate::RtckError {
                class: crate::RtckErrorClass::ParseError,
                desc: "Error HTTP method".to_string(),
            }),
        }
    }
}

pub struct HttpResponse {
    code: usize,
    headers: String,
    body: String,
}

impl HttpResponse {
    pub fn is_fine(&self) -> bool {
        if self.code == 200 || self.code == 204 {
            true
        } else {
            false
        }
    }
    pub fn headers(&self) -> &String {
        &self.headers
    }
    pub fn body(&self) -> &String {
        &self.body
    }
}

pub mod http_io {
    use std::io::BufRead;

    use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt};

    use crate::{RtckError, RtckResult};

    use super::HttpResponse;

    pub fn read_response<S: BufRead>(stream: &mut S) -> RtckResult<HttpResponse> {
        let mut res = String::new();
        let mut buf = String::new();

        let code: usize;
        const HTTP_PATTERN: &'static str = "HTTP/";
        const PATTERN_LEN: usize = HTTP_PATTERN.len();
        loop {
            stream.read_line(&mut buf)?;

            if buf.len() >= PATTERN_LEN {
                match buf.find("HTTP/") {
                    None => (),
                    Some(pos) => {
                        let t = buf.split_at(pos).1;
                        res += t;
                        code = t
                            .split_ascii_whitespace()
                            .skip(1)
                            .next()
                            .ok_or(RtckError::new(
                                crate::RtckErrorClass::ParseError,
                                "Malformed HTTP response",
                            ))?
                            .parse::<usize>()?;
                        break;
                    }
                }
            }
            buf.clear()
        }

        const PATTERN: &'static str = "Content-Length: ";
        let mut len = None::<usize>;
        loop {
            buf.clear();

            stream.read_line(&mut buf)?;
            res += &buf;

            if &buf[0..2] == "\r\n" {
                break;
            }

            len = buf
                .trim_end()
                .trim_start_matches(PATTERN)
                .parse::<usize>()
                .ok();
        }

        match len {
            Some(len) => {
                let mut buf: Vec<u8> = vec![0; len];
                stream.read_exact(&mut buf)?;
                let body = String::from_utf8(buf)?;
                Ok(HttpResponse {
                    code,
                    headers: res,
                    body,
                })
            }
            None => Err(RtckError::new(
                crate::RtckErrorClass::IoError,
                "Fail to read response due to no content length specification",
            )),
        }
    }

    pub async fn read_response_async<S: AsyncBufRead + Unpin>(
        stream: &mut S,
    ) -> RtckResult<HttpResponse> {
        let mut res = String::new();
        let mut buf = String::new();

        let code: usize;
        const HTTP_PATTERN: &'static str = "HTTP/";
        const PATTERN_LEN: usize = HTTP_PATTERN.len();
        loop {
            stream.read_line(&mut buf).await?;
            if buf.len() >= PATTERN_LEN {
                match buf.find("HTTP/") {
                    None => (),
                    Some(pos) => {
                        let t = buf.split_at(pos).1;
                        res += t;
                        code = t
                            .split_ascii_whitespace()
                            .skip(1)
                            .next()
                            .ok_or(RtckError::new(
                                crate::RtckErrorClass::ParseError,
                                "Malformed HTTP response",
                            ))?
                            .parse::<usize>()?;
                        break;
                    }
                }
            }
            buf.clear()
        }

        const PATTERN: &'static str = "Content-Length: ";
        let mut len = None::<usize>;
        loop {
            buf.clear();

            stream.read_line(&mut buf).await?;
            res += &buf;

            if &buf[0..2] == "\r\n" {
                break;
            }

            len = buf
                .trim_end()
                .trim_start_matches(PATTERN)
                .parse::<usize>()
                .ok();
        }

        match len {
            Some(len) => {
                let mut buf: Vec<u8> = vec![0; len];
                stream.read_exact(&mut buf).await?;
                let body = String::from_utf8(buf)?;
                Ok(HttpResponse {
                    code,
                    headers: res,
                    body,
                })
            }
            None => Err(RtckError::new(
                crate::RtckErrorClass::IoError,
                "Fail to read response due to no content length specification",
            )),
        }
    }
}

pub trait Http {
    fn encode(&self) -> RtckResult<String>;
}

#[cfg(test)]
mod test {
    use std::{
        io::{Read, Write},
        os::unix::net::{UnixListener, UnixStream},
    };

    use bufstream::BufStream;

    use super::http_io;

    const SOCKET1: &'static str = "/tmp/api1.sock";
    const SOCKET2: &'static str = "/tmp/api2.sock";

    fn run_server(write_times: usize, path: &'static str) {
        let listener = UnixListener::bind(path).expect("Server bind failed");
        let (mut stream, _addr) = listener.accept().expect("Server accept error");
        for _ in 0..write_times {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\nThis is message body")
                .expect("Server fail to write");
        }
    }

    #[test]
    fn test_read_response() {
        let _ = std::fs::remove_file(SOCKET1);
        std::thread::spawn(|| {
            run_server(1, SOCKET1);
        });
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut stream = bufstream::BufStream::new(
            std::os::unix::net::UnixStream::connect(SOCKET1).expect("Fail to connect"),
        );
        let res = http_io::read_response(&mut stream).expect("Fail to read");
        println!("Got res: {}", res.body);
        assert_eq!(res.code, 200);
        assert_eq!(res.body, "This is message body".to_string());
    }

    #[test]
    fn test_read_response_bad() {
        let _ = std::fs::remove_file(SOCKET2);
        std::thread::spawn(|| {
            run_server(2, SOCKET2);
        });
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut stream = BufStream::new(UnixStream::connect(SOCKET2).expect("Fail to connect"));

        // Bad read
        let mut bad_buf: Vec<u8> = vec![0; 30];
        stream.read_exact(&mut bad_buf).expect("Bad read failed");
        assert_eq!(bad_buf, b"HTTP/1.1 200 OK\r\nContent-Lengt");

        let res = http_io::read_response(&mut stream).expect("Fail to read");
        println!("Got res: {}", res.body);
        assert_eq!(res.code, 200);
        assert_eq!(res.body, "This is message body".to_string());
    }
}

#[cfg(test)]
mod test_async {
    use super::http_io;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt, BufStream},
        net::{UnixListener, UnixStream},
    };

    const SOCKET1: &'static str = "/tmp/apiasync1.sock";
    const SOCKET2: &'static str = "/tmp/apiasync2.sock";

    async fn run_server(write_times: usize, path: &'static str) {
        let listener = UnixListener::bind(path).expect("Server bind failed");
        let (mut stream, _addr) = listener.accept().await.expect("Server accept failed");
        for _ in 0..write_times {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\nThis is message body")
                .await
                .expect("Server fail to write");
        }
    }

    #[tokio::test]
    async fn test_read_response() {
        let _ = tokio::fs::remove_file(SOCKET1).await;
        tokio::spawn(run_server(1, SOCKET1));
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let mut stream =
            BufStream::new(UnixStream::connect(SOCKET1).await.expect("Fail to connect"));
        let res = http_io::read_response_async(&mut stream)
            .await
            .expect("Fail to read");
        println!("Got res: {}", res.body);
        assert_eq!(res.code, 200);
        assert_eq!(res.body, "This is message body".to_string());
    }

    #[tokio::test]
    async fn test_read_response_bad() {
        let _ = tokio::fs::remove_file(SOCKET2).await;
        tokio::spawn(run_server(2, SOCKET2));
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let mut stream =
            BufStream::new(UnixStream::connect(SOCKET2).await.expect("Fail to connect"));

        // Bad read
        let mut bad_buf: Vec<u8> = vec![0; 30];
        stream
            .read_exact(&mut bad_buf)
            .await
            .expect("Bad read failed");
        assert_eq!(bad_buf, b"HTTP/1.1 200 OK\r\nContent-Lengt");

        let res = http_io::read_response_async(&mut stream)
            .await
            .expect("Fail to read");
        println!("Got res: {}", res.body);
        assert_eq!(res.code, 200);
        assert_eq!(res.body, "This is message body".to_string());
    }
}

#[doc(hidden)]
pub(crate) mod bench {
    use std::{
        io::Write,
        os::unix::net::{UnixListener, UnixStream},
    };

    use bufstream::BufStream;

    use crate::micro_http::http_io;

    const SOCKET: &'static str = "/tmp/apibench.sock";

    fn run_server(write_times: usize, path: &'static str) {
        let listener = UnixListener::bind(path).expect("Server bind failed");
        let (mut stream, _addr) = listener.accept().expect("Server accept error");
        for _ in 0..write_times {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\nThis is message body")
                .expect("Server fail to write");
        }
    }

    pub(crate) fn test_read_pressure_interactive(write_times: usize) {
        let _ = std::fs::remove_file(SOCKET);
        let start = std::time::Instant::now();
        std::thread::spawn(move || {
            run_server(write_times, SOCKET);
        });
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut stream = BufStream::new(UnixStream::connect(SOCKET).expect("Fail to connect"));
        for i in 0..write_times {
            http_io::read_response(&mut stream)
                .expect(format!("Fail to read at {i} time").as_str());
        }
        println!("Time: {} ms", start.elapsed().as_millis());
    }
}

#[doc(hidden)]
pub(crate) mod bench_async {
    use tokio::{
        io::{AsyncWriteExt, BufStream},
        net::{UnixListener, UnixStream},
    };

    use super::http_io;

    const SOCKET: &'static str = "/tmp/apibenchasync.sock";

    async fn run_server(write_times: usize, path: &'static str) {
        let listener = UnixListener::bind(path).expect("Server bind failed");
        let (mut stream, _addr) = listener.accept().await.expect("Server accept failed");
        for _ in 0..write_times {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\nThis is message body")
                .await
                .expect("Server fail to write");
        }
    }

    pub(crate) async fn test_read_pressure(write_times: usize) {
        let _ = tokio::fs::remove_file(SOCKET).await;
        let start = tokio::time::Instant::now();
        tokio::spawn(async move { run_server(write_times, SOCKET).await });
        tokio::task::yield_now().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let mut stream =
            BufStream::new(UnixStream::connect(SOCKET).await.expect("Fail to connect"));
        for i in 0..write_times {
            http_io::read_response_async(&mut stream)
                .await
                .expect(format!("Fail to read at {i} time").as_str());
        }
        println!("Time: {} ms", start.elapsed().as_millis());
    }
}
