use std::collections::HashMap;
use std::io::{Error as WriteError, Write};

use super::{
    ascii::{COLON, CR, LF, SP},
    error::*,
    Encoding, MediaType, Method,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Header {
    /// Header `Content-Length`.
    ContentLength,
    /// Header `Content-Type`.
    ContentType,
    /// Header `Expect`.
    Expect,
    /// Header `Transfer-Encoding`.
    TransferEncoding,
    /// Header `Server`.
    Server,
    /// Header `Accept`
    Accept,
    /// Header `Accept-Encoding`
    AcceptEncoding,
    /// Header `Deprecation`
    Deprecation,
    /// Header `Allow`
    Allow,
}

impl Header {
    /// Returns a byte slice representation of the object.
    pub fn raw(&self) -> &'static [u8] {
        match self {
            Self::ContentLength => b"Content-Length",
            Self::ContentType => b"Content-Type",
            Self::Expect => b"Expect",
            Self::TransferEncoding => b"Transfer-Encoding",
            Self::Server => b"Server",
            Self::Accept => b"Accept",
            Self::AcceptEncoding => b"Accept-Encoding",
            Self::Deprecation => b"Deprecation",
            Self::Allow => b"Allow",
        }
    }

    /// Parses a byte slice into a Header structure. Header must be ASCII, so also
    /// UTF-8 valid.
    ///
    /// # Errors
    /// `InvalidRequest` is returned if slice contains invalid utf8 characters.
    /// `InvalidHeader` is returned if unsupported header found.
    fn try_from(string: &[u8]) -> Result<Self, RequestError> {
        if let Ok(mut utf8_string) = String::from_utf8(string.to_vec()) {
            utf8_string.make_ascii_lowercase();
            match utf8_string.trim() {
                "content-length" => Ok(Self::ContentLength),
                "content-type" => Ok(Self::ContentType),
                "expect" => Ok(Self::Expect),
                "transfer-encoding" => Ok(Self::TransferEncoding),
                "server" => Ok(Self::Server),
                "accept" => Ok(Self::Accept),
                "accept-encoding" => Ok(Self::AcceptEncoding),
                "deprecation" => Ok(Self::Deprecation),
                "allow" => Ok(Self::Allow),
                invalid_key => Err(RequestError::HeaderError(HttpHeaderError::UnsupportedName(
                    invalid_key.to_string(),
                ))),
            }
        } else {
            Err(RequestError::InvalidRequest)
        }
    }
}

/// Wrapper over the list of headers associated with a Request that we need
/// in order to parse the request correctly and be able to respond to it.
///
/// The only `Content-Type`s supported are `text/plain` and `application/json`, which are both
/// in plain text actually and don't influence our parsing process.
///
/// All the other possible header fields are not necessary in order to serve this connection
/// and, thus, are not of interest to us. However, we still look for header fields that might
/// invalidate our request as we don't support the full set of HTTP/1.1 specification.
/// Such header entries are "Transfer-Encoding: identity; q=0", which means a compression
/// algorithm is applied to the body of the request, or "Expect: 103-checkpoint".
#[derive(Debug, PartialEq, Eq)]
pub struct RequestHeaders {
    /// The `Content-Length` header field tells us how many bytes we need to receive
    /// from the source after the headers.
    content_length: u32,
    /// The `Expect` header field is set when the headers contain the entry "Expect: 100-continue".
    /// This means that, per HTTP/1.1 specifications, we must send a response with the status code
    /// 100 after we have received the headers in order to receive the body of the request. This
    /// field should be known immediately after parsing the headers.
    expect: bool,
    /// `Chunked` is a possible value of the `Transfer-Encoding` header field and every HTTP/1.1
    /// server must support it. It is useful only when receiving the body of the request and should
    /// be known immediately after parsing the headers.
    chunked: bool,
    /// `Accept` header might be used by HTTP clients to enforce server responses with content
    /// formatted in a specific way.
    accept: MediaType,
    /// Hashmap reserved for storing custom headers.
    custom_entries: HashMap<String, String>,
}

impl Default for RequestHeaders {
    /// By default Requests are created with no headers.
    fn default() -> Self {
        Self {
            content_length: Default::default(),
            expect: Default::default(),
            chunked: Default::default(),
            // The default `Accept` media type is plain text. This is inclusive enough
            // for structured and unstructured text.
            accept: MediaType::PlainText,
            custom_entries: HashMap::default(),
        }
    }
}

impl RequestHeaders {
    /// Expects one header line and parses it, updating the header structure or returning an
    /// error if the header is invalid.
    ///
    /// # Errors
    /// `UnsupportedHeader` is returned when the parsed header line is not of interest
    /// to us or when it is unrecognizable.
    /// `InvalidHeader` is returned when the parsed header is formatted incorrectly or suggests
    /// that the client is using HTTP features that we do not support in this implementation,
    /// which invalidates the request.
    ///
    /// # Examples
    ///
    /// ```
    /// use micro_http::Headers;
    ///
    /// let mut request_header = Headers::default();
    /// assert!(request_header
    ///     .parse_header_line(b"Content-Length: 24")
    ///     .is_ok());
    /// assert!(request_header
    ///     .parse_header_line(b"Content-Length: 24: 2")
    ///     .is_err());
    /// ```
    pub fn parse_header_line(&mut self, header_line: &[u8]) -> Result<(), RequestError> {
        // Headers must be ASCII, so also UTF-8 valid.
        match std::str::from_utf8(header_line) {
            Ok(headers_str) => {
                let entry = headers_str.splitn(2, ':').collect::<Vec<&str>>();
                if entry.len() != 2 {
                    return Err(RequestError::HeaderError(HttpHeaderError::InvalidFormat(
                        entry[0].to_string(),
                    )));
                }
                if let Ok(head) = Header::try_from(entry[0].as_bytes()) {
                    match head {
                        Header::ContentLength => match entry[1].trim().parse::<u32>() {
                            Ok(content_length) => {
                                self.content_length = content_length;
                                Ok(())
                            }
                            Err(_) => {
                                Err(RequestError::HeaderError(HttpHeaderError::InvalidValue(
                                    entry[0].to_string(),
                                    entry[1].to_string(),
                                )))
                            }
                        },
                        Header::ContentType => {
                            match MediaType::try_from(entry[1].trim().as_bytes()) {
                                Ok(_) => Ok(()),
                                Err(_) => Err(RequestError::HeaderError(
                                    HttpHeaderError::UnsupportedValue(
                                        entry[0].to_string(),
                                        entry[1].to_string(),
                                    ),
                                )),
                            }
                        }
                        Header::Accept => match MediaType::try_from(entry[1].trim().as_bytes()) {
                            Ok(accept_type) => {
                                self.accept = accept_type;
                                Ok(())
                            }
                            Err(_) => Err(RequestError::HeaderError(
                                HttpHeaderError::UnsupportedValue(
                                    entry[0].to_string(),
                                    entry[1].to_string(),
                                ),
                            )),
                        },
                        Header::TransferEncoding => match entry[1].trim() {
                            "chunked" => {
                                self.chunked = true;
                                Ok(())
                            }
                            "identity" => Ok(()),
                            _ => Err(RequestError::HeaderError(
                                HttpHeaderError::UnsupportedValue(
                                    entry[0].to_string(),
                                    entry[1].to_string(),
                                ),
                            )),
                        },
                        Header::Expect => match entry[1].trim() {
                            "100-continue" => {
                                self.expect = true;
                                Ok(())
                            }
                            _ => Err(RequestError::HeaderError(
                                HttpHeaderError::UnsupportedValue(
                                    entry[0].to_string(),
                                    entry[1].to_string(),
                                ),
                            )),
                        },
                        Header::Server => Ok(()),
                        Header::AcceptEncoding => Ok(Encoding::try_from(entry[1].trim().as_bytes())?),
                        Header::Deprecation => Ok(()),
                        Header::Allow => Ok(()),
                    }
                } else {
                    self.insert_custom_header(
                        entry[0].trim().to_string(),
                        entry[1].trim().to_string(),
                    )?;
                    Ok(())
                }
            }
            Err(utf8_err) => Err(RequestError::HeaderError(
                HttpHeaderError::InvalidUtf8String(utf8_err),
            )),
        }
    }

    /// Returns the content length of the body.
    pub fn content_length(&self) -> u32 {
        self.content_length
    }

    /// Returns `true` if the transfer encoding is chunked.
    #[allow(unused)]
    pub fn chunked(&self) -> bool {
        self.chunked
    }

    /// Returns `true` if the client is expecting the code 100.
    #[allow(unused)]
    pub fn expect(&self) -> bool {
        self.expect
    }

    /// Returns the `Accept` header `MediaType`.
    pub fn accept(&self) -> MediaType {
        self.accept
    }

    /// Returns the custom header `HashMap`.
    pub fn custom_entries(&self) -> &HashMap<String, String> {
        &self.custom_entries
    }

    /// Parses a byte slice into a Headers structure for a HTTP request.
    ///
    /// The byte slice is expected to have the following format: </br>
    ///     * Request Header Lines "<header_line> CRLF"- Optional </br>
    /// There can be any number of request headers, including none, followed by
    /// an extra sequence of Carriage Return and Line Feed.
    /// All header fields are parsed. However, only the ones present in the
    /// [`Headers`](struct.Headers.html) struct are relevant to us and stored
    /// for future use.
    ///
    /// # Errors
    /// The function returns `InvalidHeader` when parsing the byte stream fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use micro_http::RequestHeaders;
    ///
    /// let request_headers = Headers::try_from(b"Content-Length: 55\r\n\r\n");
    /// ```
    pub fn try_from(bytes: &[u8]) -> Result<RequestHeaders, RequestError> {
        // Headers must be ASCII, so also UTF-8 valid.
        if let Ok(text) = std::str::from_utf8(bytes) {
            let mut headers = Self::default();

            let header_lines = text.split("\r\n");
            for header_line in header_lines {
                if header_line.is_empty() {
                    break;
                }
                match headers.parse_header_line(header_line.as_bytes()) {
                    Ok(_)
                    | Err(RequestError::HeaderError(HttpHeaderError::UnsupportedValue(_, _))) => {
                        continue
                    }
                    Err(e) => return Err(e),
                };
            }
            return Ok(headers);
        }
        Err(RequestError::InvalidRequest)
    }
}

impl RequestHeaders {
    /// Writes the headers to `buf` using the HTTP specification.
    pub fn write_all<T: Write>(&self, buf: &mut T) -> Result<(), WriteError> {
        buf.write_all(Header::ContentLength.raw())?;
        buf.write_all(&[COLON, SP])?;
        buf.write_all(self.content_length.to_string().as_bytes())?;
        buf.write_all(&[CR, LF])?;

        buf.write_all(Header::Accept.raw())?;
        buf.write_all(&[COLON, SP])?;
        buf.write_all(self.accept.as_str().as_bytes())?;
        buf.write_all(&[CR, LF])?;

        buf.write_all(&[CR, LF])
    }

    /// Sets the content length to be written in the HTTP request.
    pub fn set_content_length(&mut self, content_length: u32) {
        self.content_length = content_length;
    }

    /// Sets the HTTP request expect
    pub fn set_expect(&mut self, expect: bool) {
        self.expect = expect;
    }

    /// Sets the HTTP request chunked
    pub fn set_chunked(&mut self, chunked: bool) {
        self.chunked = chunked;
    }

    /// Accept header setter.
    pub fn set_accept(&mut self, media_type: MediaType) {
        self.accept = media_type;
    }

    /// Insert a new custom header and value pair into the `HashMap`.
    pub fn insert_custom_header(&mut self, key: String, value: String) -> Result<(), RequestError> {
        self.custom_entries.insert(key, value);
        Ok(())
    }
}

/// Wrapper over the list of headers associated with a HTTP Response.
/// When creating a ResponseHeaders object, the content type is initialized to `text/plain`.
/// The content type can be updated with a call to `set_content_type`.
#[derive(Debug, PartialEq, Eq)]
pub struct ResponseHeaders {
    content_length: Option<i32>,
    content_type: MediaType,
    deprecation: bool,
    server: String,
    allow: Vec<Method>,
    accept_encoding: bool,
    /// Hashmap reserved for storing custom headers.
    custom_entries: HashMap<String, String>,
}

impl Default for ResponseHeaders {
    /// By default Responses are created with no headers.
    fn default() -> Self {
        Self {
            content_length: Default::default(),
            content_type: Default::default(),
            deprecation: false,
            // server: String::from("Firecracker API"),
            server: Default::default(),
            allow: Vec::new(),
            accept_encoding: false,
            custom_entries: Default::default(),
        }
    }
}

impl ResponseHeaders {
    /// Expects one header line and parses it, updating the header structure or returning an
    /// error if the header is invalid.
    ///
    /// # Errors
    /// `UnsupportedHeader` is returned when the parsed header line is not of interest
    /// to us or when it is unrecognizable.
    /// `InvalidHeader` is returned when the parsed header is formatted incorrectly or suggests
    /// that the client is using HTTP features that we do not support in this implementation,
    /// which invalidates the request.
    ///
    pub fn parse_header_line(&mut self, header_line: &[u8]) -> Result<(), ResponseError> {
        // Headers must be ASCII, so also UTF-8 valid.
        match std::str::from_utf8(header_line) {
            Ok(header_str) => {
                let entry = header_str.splitn(2, ':').collect::<Vec<&str>>();
                if entry.len() != 2 {
                    return Err(ResponseError::HeaderError(HttpHeaderError::InvalidFormat(
                        entry[0].to_string(),
                    )));
                }
                if let Ok(head) = Header::try_from(entry[0].as_bytes()) {
                    match head {
                        Header::ContentLength => match entry[1].trim().parse::<i32>() {
                            Ok(content_length) => {
                                self.content_length = Some(content_length);
                                Ok(())
                            }
                            Err(_) => {
                                Err(ResponseError::HeaderError(HttpHeaderError::InvalidValue(
                                    entry[0].to_string(),
                                    entry[1].to_string(),
                                )))
                            }
                        },
                        Header::ContentType => {
                            match MediaType::try_from(entry[1].trim().as_bytes()) {
                                Ok(media_type) => {
                                    self.content_type = media_type;
                                    Ok(())
                                }
                                Err(_) => Err(ResponseError::HeaderError(
                                    HttpHeaderError::UnsupportedValue(
                                        entry[0].to_string(),
                                        entry[1].to_string(),
                                    ),
                                )),
                            }
                        }
                        Header::Accept => Ok(()),
                        Header::TransferEncoding => Ok(()),
                        Header::Expect => Ok(()),
                        Header::Server => match entry[1].trim() {
                            server @ "Firecracker API" => {
                                self.server = server.to_string();
                                Ok(())
                            }
                            _ => Err(ResponseError::HeaderError(
                                HttpHeaderError::UnsupportedValue(
                                    entry[0].to_string(),
                                    entry[1].to_string(),
                                ),
                            )),
                        },
                        Header::AcceptEncoding => Ok(Encoding::try_from(entry[1].trim().as_bytes())?),
                        Header::Deprecation => {
                            self.deprecation = true;
                            Ok(())
                        }
                        Header::Allow => {
                            let methods = entry[1].split(", ").collect::<Vec<&str>>();
                            for method in methods {
                                let method = Method::try_from(method.as_bytes())?;
                                self.allow_method(method);
                            }
                            Ok(())
                        }
                    }
                } else {
                    self.insert_custom_header(
                        entry[0].trim().to_string(),
                        entry[1].trim().to_string(),
                    )?;
                    Ok(())
                }
            }
            Err(utf8_err) => Err(ResponseError::HeaderError(
                HttpHeaderError::InvalidUtf8String(utf8_err),
            )),
        }
    }

    /// Returns the content length of the body.
    pub fn content_length(&self) -> i32 {
        self.content_length.unwrap_or(0)
    }

    /// Returns the content type of the body.
    pub fn content_type(&self) -> MediaType {
        self.content_type
    }

    /// Returns `true` if deprecated.
    pub fn deprecation(&self) -> bool {
        self.deprecation
    }

    /// Returns the server
    pub fn server(&self) -> String {
        self.server.clone()
    }

    /// Returns the allowed methods
    pub fn allow(&self) -> Vec<Method> {
        self.allow.clone()
    }

    /// Returns the custom header `HashMap`.
    pub fn custom_entries(&self) -> &HashMap<String, String> {
        &self.custom_entries
    }

    /// Parses a byte slice into a Headers structure for a HTTP response.
    ///
    /// The byte slice is expected to have the following format: </br>
    ///     * Response Header Lines "<header_line> CRLF"- Optional </br>
    /// There can be any number of response headers, including none, followed by
    /// an extra sequence of Carriage Return and Line Feed.
    /// All header fields are parsed. However, only the ones present in the
    /// [`Headers`](struct.Headers.html) struct are relevant to us and stored
    /// for future use.
    ///
    /// # Errors
    /// The function returns `InvalidHeader` when parsing the byte stream fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use micro_http::ResponseHeaders;
    ///
    /// let response_headers = ResponseHeaders::try_from(b"Content-Length: 55\r\n\r\n");
    /// ```
    pub fn try_from(bytes: &[u8]) -> Result<ResponseHeaders, ResponseError> {
        // Headers must be ASCII, so also UTF-8 valid.
        if let Ok(text) = std::str::from_utf8(bytes) {
            let mut headers = Self::default();

            let header_lines = text.split("\r\n");
            for header_line in header_lines {
                if header_line.is_empty() {
                    break;
                }
                match headers.parse_header_line(header_line.as_bytes()) {
                    Ok(_)
                    | Err(ResponseError::HeaderError(HttpHeaderError::UnsupportedValue(_, _))) => {
                        continue
                    }
                    Err(e) => return Err(e),
                };
            }
            return Ok(headers);
        }
        Err(ResponseError::InvalidResponse)
    }
}

impl ResponseHeaders {
    // The logic pertaining to `Allow` header writing.
    fn write_allow_header<T: Write>(&self, buf: &mut T) -> Result<(), WriteError> {
        if self.allow.is_empty() {
            return Ok(());
        }

        buf.write_all(b"Allow: ")?;

        let delimitator = b", ";
        for (idx, method) in self.allow.iter().enumerate() {
            buf.write_all(method.raw())?;
            // We check above that `self.allow` is not empty.
            if idx < self.allow.len() - 1 {
                buf.write_all(delimitator)?;
            }
        }

        buf.write_all(&[CR, LF])
    }

    // The logic pertaining to `Deprecation` header writing.
    fn write_deprecation_header<T: Write>(&self, buf: &mut T) -> Result<(), WriteError> {
        if !self.deprecation {
            return Ok(());
        }

        buf.write_all(b"Deprecation: true")?;
        buf.write_all(&[CR, LF])
    }

    /// Writes the headers to `buf` using the HTTP specification.
    pub fn write_all<T: Write>(&self, buf: &mut T) -> Result<(), WriteError> {
        buf.write_all(Header::Server.raw())?;
        buf.write_all(&[COLON, SP])?;
        buf.write_all(self.server.as_bytes())?;
        buf.write_all(&[CR, LF])?;

        buf.write_all(b"Connection: keep-alive")?;
        buf.write_all(&[CR, LF])?;

        self.write_allow_header(buf)?;
        self.write_deprecation_header(buf)?;

        if let Some(content_length) = self.content_length {
            buf.write_all(Header::ContentType.raw())?;
            buf.write_all(&[COLON, SP])?;
            buf.write_all(self.content_type.as_str().as_bytes())?;
            buf.write_all(&[CR, LF])?;

            buf.write_all(Header::ContentLength.raw())?;
            buf.write_all(&[COLON, SP])?;
            buf.write_all(content_length.to_string().as_bytes())?;
            buf.write_all(&[CR, LF])?;
            if self.accept_encoding {
                buf.write_all(Header::AcceptEncoding.raw())?;
                buf.write_all(&[COLON, SP])?;
                buf.write_all(b"identity")?;
                buf.write_all(&[CR, LF])?;
            }
        }

        buf.write_all(&[CR, LF])
    } 

    /// Sets the content length to be written in the HTTP response.
    pub fn set_content_length(&mut self, content_length: Option<i32>) {
        self.content_length = content_length;
    }

    /// Sets the HTTP response header server.
    pub fn set_server(&mut self, server: &str) {
        self.server = String::from(server);
    }

    /// Sets the content type to be written in the HTTP response.
    pub fn set_content_type(&mut self, content_type: MediaType) {
        self.content_type = content_type;
    }

    /// Sets the HTTP allowed methods.
    #[allow(unused)]
    pub fn set_allow(&mut self, methods: Vec<Method>) {
        self.allow = methods;
    }

    /// Allows a specific HTTP method.
    pub fn allow_method(&mut self, method: Method) {
        self.allow.push(method);
    }

    /// Sets the `Deprecation` header to be written in the HTTP response.
    /// https://tools.ietf.org/id/draft-dalal-deprecation-header-03.html
    #[allow(unused)]
    pub fn set_deprecation(&mut self) {
        self.deprecation = true;
    }

    /// Sets the encoding type to be written in the HTTP response.
    #[allow(unused)]
    pub fn set_encoding(&mut self) {
        self.accept_encoding = true;
    }

    /// Insert a new custom header and value pair into the `HashMap`.
    pub fn insert_custom_header(&mut self, key: String, value: String) -> Result<(), ResponseError> {
        self.custom_entries.insert(key, value);
        Ok(())
    }
}
