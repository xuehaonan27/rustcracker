use std::fmt::{Display, Error, Formatter};
use std::str::Utf8Error;

/// Errors associated with a header that is invalid.
#[derive(Debug, PartialEq, Eq)]
pub enum HttpHeaderError {
    /// The header is misformatted.
    InvalidFormat(String),
    /// The specified header contains illegal characters.
    InvalidUtf8String(Utf8Error),
    ///The value specified is not valid.
    InvalidValue(String, String),
    /// The content length specified is longer than the limit imposed by Micro Http.
    SizeLimitExceeded(String),
    /// The requested feature is not currently supported.
    UnsupportedFeature(String, String),
    /// The header specified is not supported.
    UnsupportedName(String),
    /// The value for the specified header is not supported.
    UnsupportedValue(String, String),
}

impl Display for HttpHeaderError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::InvalidFormat(header_key) => {
                write!(f, "Header is incorrectly formatted. Key: {}", header_key)
            }
            Self::InvalidUtf8String(header_key) => {
                write!(f, "Header contains invalid characters. Key: {}", header_key)
            }
            Self::InvalidValue(header_name, value) => {
                write!(f, "Invalid value. Key:{}; Value:{}", header_name, value)
            }
            Self::SizeLimitExceeded(inner) => {
                write!(f, "Invalid content length. Header: {}", inner)
            }
            Self::UnsupportedFeature(header_key, header_value) => write!(
                f,
                "Unsupported feature. Key: {}; Value: {}",
                header_key, header_value
            ),
            Self::UnsupportedName(inner) => write!(f, "Unsupported header name. Key: {}", inner),
            Self::UnsupportedValue(header_key, header_value) => write!(
                f,
                "Unsupported value. Key:{}; Value:{}",
                header_key, header_value
            ),
        }
    }
}

/// Errors associated with parsing the HTTP Request from a u8 slice.
#[derive(Debug, PartialEq, Eq)]
pub enum RequestError {
    /// No request was pending while the request body was being parsed.
    BodyWithoutPendingRequest,
    /// Header specified is either invalid or not supported by this HTTP implementation.
    HeaderError(HttpHeaderError),
    /// No request was pending while the request headers were being parsed.
    HeadersWithoutPendingRequest,
    /// The HTTP Method is not supported or it is invalid.
    InvalidHttpMethod(&'static str),
    /// The HTTP Version in the Request is not supported or it is invalid.
    InvalidHttpVersion(&'static str),
    /// The Request is invalid and cannot be served.
    InvalidRequest,
    /// Request URI is invalid.
    InvalidUri(&'static str),
    /// Overflow occurred when parsing a request.
    Overflow,
    /// Underflow occurred when parsing a request.
    Underflow,
    /// Payload too large.
    SizeLimitExceeded(usize, usize),
}

impl Display for RequestError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::BodyWithoutPendingRequest => write!(
                f,
                "No request was pending while the request body was being parsed."
            ),
            Self::HeaderError(inner) => write!(f, "Invalid header. Reason: {}", inner),
            Self::HeadersWithoutPendingRequest => write!(
                f,
                "No request was pending while the request headers were being parsed."
            ),
            Self::InvalidHttpMethod(inner) => write!(f, "Invalid HTTP Method: {}", inner),
            Self::InvalidHttpVersion(inner) => write!(f, "Invalid HTTP Version: {}", inner),
            Self::InvalidRequest => write!(f, "Invalid request."),
            Self::InvalidUri(inner) => write!(f, "Invalid URI: {}", inner),
            Self::Overflow => write!(f, "Overflow occurred when parsing a request."),
            Self::Underflow => write!(f, "Underflow occurred when parsing a request."),
            Self::SizeLimitExceeded(limit, size) => write!(
                f,
                "Request payload with size {} is larger than the limit of {} \
                 allowed by server.",
                size, limit
            ),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResponseError {
    /// Header specified is either invalid or not supported by this HTTP implementation.
    HeaderError(HttpHeaderError),
    /// The HTTP Method is not supported or it is invalid.
    InvalidHttpMethod(&'static str),
    /// The HTTP Version in the Request is not supported or it is invalid.
    InvalidHttpVersion(&'static str),
    /// The HTTP StatusCode is not supported or it is invalid.
    InvalidStatusCode(&'static str),
    /// The Response is invalid and cannot be received.
    InvalidResponse,
    /// Overflow occurred when parsing a request.
    Overflow,
    /// Underflow occurred when parsing a request.
    Underflow,
    /// Payload too large.
    SizeLimitExceeded(usize, usize),
}

impl Display for ResponseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HeaderError(inner) => write!(f, "Invalid header. Reason: {}", inner),
            Self::InvalidHttpMethod(inner) => write!(f, "Invalid HTTP Method: {}", inner),
            Self::InvalidStatusCode(inner) => write!(f, "Invalid HTTP StatusCode: {:?}", inner),
            Self::InvalidHttpVersion(inner) => write!(f, "Invalid HTTP Version: {}", inner),
            Self::InvalidResponse => write!(f, "Invalid response."),
            Self::Overflow => write!(f, "Overflow occurred when parsing a response."),
            Self::Underflow => write!(f, "Underflow occurred when parsing a response."),
            Self::SizeLimitExceeded(limit, size) => write!(
                f,
                "Response payload with size {} is larger than the limit of {} \
                 allowed by server.",
                size, limit
            ),
        }
    }
}

pub enum VersionError {
    /// The HTTP Version in the Request is not supported or it is invalid.
    InvalidHttpVersion(&'static str),
}

impl Display for VersionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHttpVersion(inner) => write!(f, "Invalid HTTP Version: {}", inner),
        }
    }
}

impl From<VersionError> for RequestError {
    fn from(e: VersionError) -> Self {
        match e {
            VersionError::InvalidHttpVersion(inner) => RequestError::InvalidHttpVersion(inner),
        }
    }
}

impl From<VersionError> for ResponseError {
    fn from(e: VersionError) -> Self {
        match e {
            VersionError::InvalidHttpVersion(inner) => ResponseError::InvalidHttpVersion(inner),
        }
    }
}

pub enum EncodingError {
    HeaderError(HttpHeaderError),
    InvalidValue,
}

impl From<EncodingError> for RequestError {
    fn from(e: EncodingError) -> Self {
        match e {
            EncodingError::HeaderError(inner) => RequestError::HeaderError(inner),
            EncodingError::InvalidValue => RequestError::InvalidRequest,
        }
    }
}

impl From<EncodingError> for ResponseError {
    fn from(e: EncodingError) -> Self {
        match e {
            EncodingError::HeaderError(inner) => ResponseError::HeaderError(inner),
            EncodingError::InvalidValue => ResponseError::InvalidResponse,
        }
    }
}

pub enum MethodError {
    /// The HTTP Method is not supported or it is invalid.
    InvalidHttpMethod(&'static str),
}

impl From<MethodError> for RequestError {
    fn from(e: MethodError) -> Self {
        match e {
            MethodError::InvalidHttpMethod(inner) => RequestError::InvalidHttpMethod(inner),
        }
    }
}

impl From<MethodError> for ResponseError {
    fn from(e: MethodError) -> Self {
        match e {
            MethodError::InvalidHttpMethod(inner) => ResponseError::InvalidHttpMethod(inner),
        }
    }
}

/// Errors associated with a HTTP Connection.
#[derive(Debug)]
pub enum ConnectionError {
    /// Attempted to read or write on a closed connection.
    ConnectionClosed,
    /// Attempted to write on a stream when there was nothing to write.
    InvalidWrite,
    /// The request parsing has failed.
    ParseError(RequestError),
    /// Could not perform a read operation from stream successfully.
    StreamReadError(std::io::Error),
    /// Could not perform a write operation to stream successfully.
    StreamWriteError(std::io::Error),
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::ConnectionClosed => write!(f, "Connection closed."),
            Self::InvalidWrite => write!(f, "Invalid write attempt."),
            Self::ParseError(inner) => write!(f, "Parsing error: {}", inner),
            Self::StreamReadError(inner) => write!(f, "Reading stream error: {}", inner),
            Self::StreamWriteError(inner) => write!(f, "Writing stream error: {}", inner),
        }
    }
}

/// Errors pertaining to `HttpServer`.
#[derive(Debug)]
pub enum ServerError {
    /// Error from one of the connections.
    ConnectionError(ConnectionError),
    /// Epoll operations failed.
    IOError(std::io::Error),
    /// Overflow occurred while processing messages.
    Overflow,
    /// Server maximum capacity has been reached.
    ServerFull,
    /// Shutdown requested.
    ShutdownEvent,
    /// Underflow occurred while processing messages.
    Underflow,
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::ConnectionError(inner) => write!(f, "Connection error: {}", inner),
            Self::IOError(inner) => write!(f, "IO error: {}", inner),
            Self::Overflow => write!(f, "Overflow occured while processing messages."),
            Self::ServerFull => write!(f, "Server is full."),
            Self::Underflow => write!(f, "Underflow occured while processing messages."),
            Self::ShutdownEvent => write!(f, "Shutdown requested."),
        }
    }
}

/// Errors pertaining to `HttpClient`
#[derive(Debug)]
pub enum ClientError {
    /// Error from one of the connections.
    ConnectionError(ConnectionError),
    /// Epoll operations failed.
    IOError(std::io::Error),
    /// Overflow occurred while processing messages.
    Overflow,
    /// Client maximum capacity has been reached.
    ClientFull,
    /// Shutdown requested.
    ShutdownEvent,
    /// Underflow occurred while processing messages.
    Underflow,
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::ConnectionError(inner) => write!(f, "Connection error: {}", inner),
            Self::IOError(inner) => write!(f, "IO error: {}", inner),
            Self::Overflow => write!(f, "Overflow occured while processing messages."),
            Self::ClientFull => write!(f, "Client is full."),
            Self::Underflow => write!(f, "Underflow occured while processing messages."),
            Self::ShutdownEvent => write!(f, "Shutdown requested."),
        }
    }
}
