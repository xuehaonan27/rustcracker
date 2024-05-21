pub mod command;
pub mod micro_http;
pub mod models;
pub mod ops_res;
pub mod ser;
pub mod events;

use std::{io, num::ParseIntError, string::FromUtf8Error};

use serde::{Deserialize, Serialize};

mod rtck_conn {
    use std::io::{BufRead, Write};

    use crate::{
        micro_http::{http_io, Http, HttpResponse},
        RtckResult,
    };

    pub struct RtckConn<S> {
        // Stream of connection
        pub(crate) stream: S,
    }

    impl<S> RtckConn<S> {
        pub fn from_stream(stream: S) -> RtckConn<S> {
            RtckConn { stream }
        }
    }

    impl<S: BufRead> RtckConn<S> {
        pub fn read_response(&mut self) -> RtckResult<HttpResponse> {
            http_io::read_response(&mut self.stream)
        }
    }

    impl<S: Write> RtckConn<S> {
        pub fn write_request<T: AsRef<str>>(&mut self, req: &T) -> RtckResult<()> {
            Ok(self.stream.write_all(req.as_ref().as_bytes())?)
        }
    }
}

mod rtck_conn_async {
    use tokio::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt};

    use crate::{
        micro_http::{http_io, HttpResponse},
        RtckResult,
    };

    pub struct RtckConnAsync<S> {
        // Async stream of connection
        pub(crate) stream: S,
    }

    impl<S> RtckConnAsync<S> {
        pub fn from_stream(stream: S) -> RtckConnAsync<S> {
            RtckConnAsync { stream }
        }
    }

    impl<S: AsyncBufRead + Unpin> RtckConnAsync<S> {
        pub async fn read_response(&mut self) -> RtckResult<HttpResponse> {
            http_io::read_response_async(&mut self.stream).await
        }
    }

    impl<S: AsyncWrite + Unpin> RtckConnAsync<S> {
        pub async fn write_request<T: AsRef<str>>(&mut self, req: &T) -> RtckResult<()> {
            Ok(self.stream.write_all(req.as_ref().as_bytes()).await?)
        }
    }
}

pub mod rtck {
    use std::io::{BufRead, Write};

    use crate::{
        micro_http::Http,
        ops_res::{Operation, Response},
        rtck_conn::RtckConn,
        RtckResult,
    };

    pub struct Rtck<S> {
        conn: RtckConn<S>,
    }

    impl<S> Rtck<S> {
        pub fn from_stream(stream: S) -> Self {
            Self {
                conn: RtckConn::from_stream(stream),
            }
        }
    }

    impl<S: BufRead> Rtck<S> {
        pub fn recv_response<R: Response>(&mut self) -> RtckResult<R::Data> {
            let res = self.conn.read_response()?;
            Ok(R::decode(&res)?)
        }
    }

    impl<S: Write> Rtck<S> {
        pub fn send_request(&mut self, ops: &dyn Operation) -> RtckResult<()> {
            let req = ops.encode().encode();
            self.conn.write_request(&req)
        }
    }
}

pub mod rtck_async {
    use tokio::io::{AsyncBufRead, AsyncWrite};

    use crate::{
        micro_http::Http,
        ops_res::{Operation, Response},
        rtck_conn_async::RtckConnAsync,
        RtckResult,
    };

    pub struct RtckAsync<S> {
        conn: RtckConnAsync<S>,
    }

    impl<S> RtckAsync<S> {
        pub fn from_stream(stream: S) -> Self {
            Self {
                conn: RtckConnAsync::from_stream(stream),
            }
        }
    }

    impl<S: AsyncBufRead + Unpin> RtckAsync<S> {
        pub async fn recv_response<R: Response>(&mut self) -> RtckResult<R::Data> {
            let res = self.conn.read_response().await?;
            Ok(R::decode(&res)?)
        }
    }

    impl<S: AsyncWrite + Unpin> RtckAsync<S> {
        pub async fn send_request(&mut self, ops: &dyn Operation) -> RtckResult<()> {
            let req = ops.encode().encode();
            self.conn.write_request(&req).await
        }
    }
}

pub use serde_json::Value as Any;
pub type Dictionary = serde_json::Map<String, Any>;

/* ------------------------------ Error ------------------------------ */
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RtckErrorClass {
    /// Generic error, most errors should fall into this field
    GenericError,
    /// Error when serializing/deserializing
    SerdeError,
    /// Error when performing I/O
    IoError,
    /// Error when parsing
    ParseError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtckError {
    class: RtckErrorClass,
    desc: String,
}

impl RtckError {
    pub fn new<S: AsRef<str>>(class: RtckErrorClass, desc: S) -> Self {
        Self {
            class,
            desc: desc.as_ref().to_string(),
        }
    }
}

impl std::error::Error for RtckError {
    fn description(&self) -> &str {
        &self.desc
    }
}

impl std::fmt::Display for RtckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.desc, f)
    }
}

impl From<io::Error> for RtckError {
    fn from(e: io::Error) -> Self {
        RtckError {
            class: RtckErrorClass::IoError,
            desc: e.to_string(),
        }
    }
}

impl From<FromUtf8Error> for RtckError {
    fn from(e: FromUtf8Error) -> Self {
        RtckError {
            class: RtckErrorClass::ParseError,
            desc: e.to_string(),
        }
    }
}

impl From<std::str::Utf8Error> for RtckError {
    fn from(e: std::str::Utf8Error) -> Self {
        RtckError {
            class: RtckErrorClass::ParseError,
            desc: e.to_string(),
        }
    }
}

impl From<ParseIntError> for RtckError {
    fn from(e: ParseIntError) -> Self {
        RtckError {
            class: RtckErrorClass::ParseError,
            desc: e.to_string(),
        }
    }
}

impl From<serde_json::Error> for RtckError {
    fn from(e: serde_json::Error) -> Self {
        RtckError {
            class: RtckErrorClass::SerdeError,
            desc: e.to_string(),
        }
    }
}

pub type RtckResult<T> = std::result::Result<T, RtckError>;

mod error_serde {
    use serde::{Deserialize, Serialize};

    use super::{Any, RtckErrorClass};

    #[derive(Deserialize)]
    pub struct RtckErrorValue {
        pub class: RtckErrorClass,
        pub desc: String,
    }

    #[derive(Serialize)]
    pub struct RtckErrorValueSer<'a> {
        pub class: &'a RtckErrorClass,
        pub desc: &'a str,
    }

    #[derive(Serialize)]
    struct RtckErrorSer<'a> {
        error: RtckErrorValueSer<'a>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<&'a Any>,
    }
}
