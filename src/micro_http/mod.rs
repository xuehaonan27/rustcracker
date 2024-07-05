pub mod body;
pub mod error;
pub mod header;
pub mod media_type;
pub mod method;
pub mod request;
pub mod response;
pub mod version;
pub mod encoding;
pub mod connection;
pub mod server;

pub use body::Body;
pub use error::ClientError;
pub use error::ConnectionError;
pub use error::HttpHeaderError;
pub use error::RequestError;
pub use error::ResponseError;
pub use error::VersionError;
pub use error::EncodingError;
pub use error::ServerError;

pub use header::Header;
pub use header::RequestHeaders;
pub use media_type::MediaType;
pub use method::Method;
pub use version::Version;
pub use encoding::Encoding;

pub use request::Request;
pub use request::RequestLine;
pub use response::Response;
pub use response::StatusCode;

pub use connection::HttpConnection;

pub mod ascii {
    pub const CR: u8 = b'\r';
    pub const COLON: u8 = b':';
    pub const LF: u8 = b'\n';
    pub const SP: u8 = b' ';
    pub const CRLF_LEN: usize = 2;
}

/// Finds the first occurrence of `sequence` in the `bytes` slice.
///
/// Returns the starting position of the `sequence` in `bytes` or `None` if the
/// `sequence` is not found.
pub(crate) fn find(bytes: &[u8], sequence: &[u8]) -> Option<usize> {
    bytes
        .windows(sequence.len())
        .position(|window| window == sequence)
}
