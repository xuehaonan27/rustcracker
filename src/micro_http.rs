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

        // Get HTTP status code
        stream.read_line(&mut buf)?;
        res += &buf;
        let code = buf
            .split_ascii_whitespace()
            .skip(1)
            .next()
            .ok_or(RtckError::new(
                crate::RtckErrorClass::ParseError,
                "Malformed HTTP response",
            ))?
            .parse::<usize>()?;

        const PATTERN: &'static str = "Content-Length: ";
        let mut len = None::<usize>;
        loop {
            buf.clear();

            stream.read_line(&mut buf)?;
            res += &buf;

            if &buf[0..2] == "\r\n" {
                break;
            }

            len = buf.trim_start_matches(PATTERN).parse::<usize>().ok();
        }

        match len {
            Some(len) => {
                let mut buf: Vec<u8> = Vec::with_capacity(len);
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

        // Get HTTP status code
        stream.read_line(&mut buf).await?;
        res += &buf;
        let code = buf
            .split_ascii_whitespace()
            .skip(1)
            .next()
            .ok_or(RtckError::new(
                crate::RtckErrorClass::ParseError,
                "Malformed HTTP response",
            ))?
            .parse::<usize>()?;

        const PATTERN: &'static str = "Content-Length: ";
        let mut len = None::<usize>;
        loop {
            buf.clear();

            stream.read_line(&mut buf).await?;
            res += &buf;

            if &buf[0..2] == "\r\n" {
                break;
            }

            len = buf.trim_start_matches(PATTERN).parse::<usize>().ok();
        }

        match len {
            Some(len) => {
                let mut buf: Vec<u8> = Vec::with_capacity(len);
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
